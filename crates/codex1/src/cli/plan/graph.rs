//! `codex1 plan graph` — emit the task DAG in Mermaid, Graphviz, or JSON
//! form. Node styling reflects each task's effective status (derived from
//! STATE.json task records plus DAG readiness).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::waves::{load_plan_tasks, ParsedTask};
use super::GraphFormat;
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::state::{self, TaskStatus};

/// Node-level status used in graph output. Mirrors the CLI contract —
/// same snake_case tokens that STATE.json uses for TaskStatus, with
/// "ready"/"blocked" filled in for tasks that have no record yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeStatus {
    Complete,
    InProgress,
    AwaitingReview,
    Ready,
    Blocked,
    Superseded,
}

impl NodeStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::InProgress => "in_progress",
            Self::AwaitingReview => "awaiting_review",
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Superseded => "superseded",
        }
    }
}

pub fn run(ctx: &Ctx, format: GraphFormat, out: Option<PathBuf>) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let tasks = load_plan_tasks(&paths, &state)?;
    let globally_blocked =
        state.replan.triggered || state::readiness::has_current_dirty_review(&state);
    let statuses = derive_node_statuses(&tasks, &state.tasks, globally_blocked);

    let (inline_key, body) = match format {
        GraphFormat::Mermaid => ("mermaid", render_mermaid(&tasks, &statuses)),
        GraphFormat::Dot => ("dot", render_dot(&tasks, &statuses)),
        GraphFormat::Json => ("graph", render_json(&tasks, &statuses).to_string()),
    };

    let data = if let Some(path) = out {
        write_to(&path, &body)?;
        json!({ "path": path })
    } else if inline_key == "graph" {
        // Re-parse the JSON so it's emitted as structured data, not a string.
        let graph: Value = serde_json::from_str(&body)?;
        json!({ "graph": graph })
    } else {
        json!({ inline_key: body })
    };

    let env = JsonOk::new(Some(state.mission_id.clone()), Some(state.revision), data);
    println!("{}", env.to_pretty());
    Ok(())
}

fn write_to(path: &Path, body: &str) -> CliResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn derive_node_statuses(
    tasks: &[ParsedTask],
    state_tasks: &BTreeMap<String, crate::state::TaskRecord>,
    globally_blocked: bool,
) -> BTreeMap<String, NodeStatus> {
    let mut out = BTreeMap::new();
    for t in tasks {
        let status = match state_tasks.get(&t.id).map(|r| r.status.clone()) {
            Some(TaskStatus::Complete) => NodeStatus::Complete,
            Some(TaskStatus::InProgress) => NodeStatus::InProgress,
            Some(TaskStatus::AwaitingReview) => NodeStatus::AwaitingReview,
            Some(TaskStatus::Superseded) => NodeStatus::Superseded,
            Some(TaskStatus::Ready) => NodeStatus::Ready,
            Some(TaskStatus::Pending) | None => {
                if !globally_blocked && deps_are_done(t, state_tasks) {
                    NodeStatus::Ready
                } else {
                    NodeStatus::Blocked
                }
            }
        };
        out.insert(t.id.clone(), status);
    }
    out
}

fn deps_are_done(
    task: &ParsedTask,
    state_tasks: &BTreeMap<String, crate::state::TaskRecord>,
) -> bool {
    let is_review = matches!(task.kind.as_deref(), Some("review"));
    let targets: std::collections::BTreeSet<&str> = task
        .review_target
        .as_ref()
        .map(|target| target.tasks.iter().map(String::as_str).collect())
        .unwrap_or_default();
    task.depends_on.iter().all(|dep| {
        state_tasks.get(dep).is_some_and(|r| {
            matches!(r.status, TaskStatus::Complete)
                || (is_review
                    && targets.contains(dep.as_str())
                    && matches!(r.status, TaskStatus::AwaitingReview))
        })
    })
}

fn render_mermaid(tasks: &[ParsedTask], statuses: &BTreeMap<String, NodeStatus>) -> String {
    let mut out = String::new();
    out.push_str("flowchart TD\n");
    out.push_str("    classDef complete fill:#b7e4c7,stroke:#2d6a4f,color:#1b4332\n");
    out.push_str("    classDef in_progress fill:#ffe066,stroke:#b08900,color:#5a4500\n");
    out.push_str("    classDef awaiting_review fill:#ffd6a5,stroke:#c75d2c,color:#5a2b12\n");
    out.push_str("    classDef ready fill:#bde0fe,stroke:#1d4ed8,color:#0b2559\n");
    out.push_str("    classDef blocked fill:#e5e7eb,stroke:#6b7280,color:#374151\n");
    out.push_str(
        "    classDef superseded fill:#f5f5f4,stroke:#a8a29e,color:#57534e,stroke-dasharray: 4 2\n",
    );

    for t in tasks {
        let label = node_label(&t.id, &t.title);
        let _ = writeln!(out, "    {}[\"{label}\"]", t.id);
    }
    for t in tasks {
        for dep in &t.depends_on {
            let _ = writeln!(out, "    {dep} --> {}", t.id);
        }
    }
    for t in tasks {
        if let Some(status) = statuses.get(&t.id) {
            let _ = writeln!(out, "    class {} {}", t.id, status.as_str());
        }
    }
    out
}

/// Shared node label: `"ID · title"` with quotes/newlines stripped so the
/// result is safe inside either Mermaid `["..."]` nodes or Graphviz
/// `label="..."` attributes.
fn node_label(id: &str, title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| match c {
            '"' => '\'',
            '\n' | '\r' => ' ',
            c => c,
        })
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        id.to_string()
    } else {
        format!("{id} · {clean}")
    }
}

fn render_dot(tasks: &[ParsedTask], statuses: &BTreeMap<String, NodeStatus>) -> String {
    let mut out = String::new();
    out.push_str("digraph Plan {\n");
    out.push_str("    rankdir=TB;\n");
    out.push_str("    node [shape=box, style=\"rounded,filled\"];\n");
    for t in tasks {
        let status = statuses.get(&t.id).copied().unwrap_or(NodeStatus::Blocked);
        let (fill, stroke) = dot_colors(status);
        let label = node_label(&t.id, &t.title);
        let _ = writeln!(
            out,
            "    {} [label=\"{label}\", fillcolor=\"{fill}\", color=\"{stroke}\"];",
            t.id
        );
    }
    for t in tasks {
        for dep in &t.depends_on {
            let _ = writeln!(out, "    {dep} -> {};", t.id);
        }
    }
    out.push_str("}\n");
    out
}

fn dot_colors(status: NodeStatus) -> (&'static str, &'static str) {
    match status {
        NodeStatus::Complete => ("#b7e4c7", "#2d6a4f"),
        NodeStatus::InProgress => ("#ffe066", "#b08900"),
        NodeStatus::AwaitingReview => ("#ffd6a5", "#c75d2c"),
        NodeStatus::Ready => ("#bde0fe", "#1d4ed8"),
        NodeStatus::Blocked => ("#e5e7eb", "#6b7280"),
        NodeStatus::Superseded => ("#f5f5f4", "#a8a29e"),
    }
}

fn render_json(tasks: &[ParsedTask], statuses: &BTreeMap<String, NodeStatus>) -> Value {
    let nodes: Vec<Value> = tasks
        .iter()
        .map(|t| {
            let status = statuses
                .get(&t.id)
                .copied()
                .unwrap_or(NodeStatus::Blocked)
                .as_str();
            json!({
                "id": t.id,
                "title": t.title,
                "kind": t.kind,
                "status": status,
            })
        })
        .collect();
    let edges: Vec<Value> = tasks
        .iter()
        .flat_map(|t| {
            t.depends_on
                .iter()
                .map(|dep| json!({ "from": dep, "to": t.id }))
                .collect::<Vec<_>>()
        })
        .collect();
    json!({ "nodes": nodes, "edges": edges })
}
