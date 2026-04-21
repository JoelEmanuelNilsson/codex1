//! Worker-subagent packet assembly for `codex1 task packet`.

use std::fs;

use serde_json::{json, Value as JsonValue};

use crate::core::error::CliError;
use crate::core::paths::{resolve_existing_mission_file, MissionPaths};

use super::lifecycle::PlanTask;

const SPEC_EXCERPT_MAX: usize = 2000;

/// Standing worker-role instructions included in every packet.
pub const WORKER_INSTRUCTIONS: &str = "You are a Codex1 worker for task <TASK_ID>.

You may: edit files inside write_paths; run proof commands; read any file.
You must not: modify OUTCOME.md, PLAN.yaml, STATE.json, EVENTS.jsonl, reviews, CLOSEOUT.md. Do not record review results. Do not replan. Do not mark missions complete.

When done, report changed files, proof commands run, proof results, blockers, and assumptions.";

/// Build the packet JSON object for a task. Reads SPEC.md and the
/// OUTCOME.md frontmatter from disk. Missing files degrade gracefully
/// (empty strings) so a worker still gets a usable envelope.
pub fn build_packet(paths: &MissionPaths, plan_task: &PlanTask) -> Result<JsonValue, CliError> {
    let spec_rel = plan_task
        .spec
        .clone()
        .unwrap_or_else(|| format!("specs/{}/SPEC.md", plan_task.id));
    let spec_abs = resolve_existing_mission_file(paths, &spec_rel, "task.spec")?;
    let spec_body = fs::read_to_string(&spec_abs).unwrap_or_default();
    let spec_excerpt = truncate_chars(&spec_body, SPEC_EXCERPT_MAX);

    let mission_summary = read_interpreted_destination(paths).unwrap_or_default();
    let spec_rel_under_plans = format!(
        "PLANS/{}/{}",
        paths.mission_id,
        spec_rel.trim_start_matches("./")
    );

    let instructions = WORKER_INSTRUCTIONS.replace("<TASK_ID>", &plan_task.id);

    Ok(json!({
        "task_id": plan_task.id,
        "title": plan_task.title,
        "kind": plan_task.kind,
        "spec_excerpt": spec_excerpt,
        "spec_path": spec_rel_under_plans,
        "read_paths": plan_task.read_paths,
        "write_paths": plan_task.write_paths,
        "proof_commands": plan_task.proof,
        "mission_summary": mission_summary,
        "mission_id": paths.mission_id,
        "worker_instructions": instructions,
    }))
}

/// Parse OUTCOME.md, return `interpreted_destination` from the YAML
/// frontmatter (block delimited by leading `---` / trailing `---`).
fn read_interpreted_destination(paths: &MissionPaths) -> Option<String> {
    let raw = fs::read_to_string(paths.outcome()).ok()?;
    let frontmatter = extract_frontmatter(&raw)?;
    let doc: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;
    doc.get("interpreted_destination")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
}

/// Pull the YAML frontmatter out of a markdown file. Returns the inner
/// block (without the `---` fences). If the whole file is YAML (no
/// leading `---`), returns the file verbatim.
fn extract_frontmatter(raw: &str) -> Option<&str> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if let Some(rest) = trimmed.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            return Some(&rest[..end]);
        }
    }
    // Tolerate CRLF.
    if let Some(rest) = trimmed.strip_prefix("---\r\n") {
        if let Some(end) = rest.find("\r\n---") {
            return Some(&rest[..end]);
        }
    }
    Some(trimmed)
}

/// Char-safe truncate so we never split a multi-byte boundary.
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.trim_end().to_string();
    }
    s.chars()
        .take(max)
        .collect::<String>()
        .trim_end()
        .to_string()
}
