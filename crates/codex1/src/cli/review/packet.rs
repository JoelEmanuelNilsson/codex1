//! `codex1 review packet <id>` — emit a reviewer-subagent packet.
//!
//! Read-only. The packet names the target tasks, their specs/proofs, the
//! files they touched (from PLAN.yaml `write_paths`), the mission summary
//! (interpreted destination from OUTCOME.md), and the standing reviewer
//! instructions literal.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::cli::review::plan_read::{fetch_review_task, load_tasks, review_targets, PlanTask};
use crate::cli::Ctx;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;
use crate::core::mission::resolve_mission;
use crate::core::paths::{
    ensure_artifact_file_read_safe, resolve_existing_mission_file, MissionPaths,
};
use crate::state::{self, schema::MissionState};

/// Standing reviewer instructions. See `04-roles-models-prompts.md`.
pub const REVIEWER_INSTRUCTIONS: &str = "You are a Codex1 reviewer. Do not edit files. Do not invoke Codex1 skills. Do not record mission truth. Do not run commands that mutate mission state. Do not perform repairs.

You may: inspect files, inspect diffs, run safe read-only commands, run tests if explicitly allowed.

Return only: NONE or P0/P1/P2 findings with evidence refs and concise rationale.

Only review the assigned target against the mission, outcome, plan, and profile. Do not review unrelated future work.";

pub fn run(ctx: &Ctx, task_id: &str) -> CliResult<()> {
    let paths = resolve_mission(&ctx.selector(), true)?;
    let state = state::load(&paths)?;
    let plan_tasks = load_tasks(&paths)?;
    let review_task = fetch_review_task(&plan_tasks, task_id)?;
    let targets = review_targets(&review_task)?;

    // Build per-target spec/proof refs + aggregated diffs from write_paths.
    let mut target_specs: Vec<Value> = Vec::new();
    let mut proofs: Vec<String> = Vec::new();
    let mut diffs: Vec<Value> = Vec::new();
    for tid in &targets {
        let target = plan_tasks.get(tid).cloned();
        target_specs.push(target_spec_value(&paths, tid, target.as_ref())?);
        if let Some(p) = proof_path_string(&paths, &state, tid) {
            proofs.push(p);
        }
        if let Some(t) = target.as_ref() {
            for path in &t.write_paths {
                diffs.push(json!({ "path": path }));
            }
        }
    }

    let profiles = review_task.review_profiles.clone();
    let primary_profile = profiles
        .first()
        .cloned()
        .unwrap_or_else(|| "code_bug_correctness".to_string());
    let mission_summary =
        read_interpreted_destination(&paths, &paths.outcome()).unwrap_or_default();

    // `proofs` is the canonical field name from `docs/cli-contract-schemas.md`.
    // `target_specs`, `profiles`, `mission_id`, and `reviewer_instructions`
    // are additive extensions documented alongside.
    let env = JsonOk::new(
        Some(state.mission_id.clone()),
        Some(state.revision),
        json!({
            "task_id": task_id,
            "review_profile": primary_profile,
            "profiles": profiles,
            "targets": targets,
            "target_specs": target_specs,
            "proofs": proofs,
            "diffs": diffs,
            "mission_summary": mission_summary,
            "mission_id": state.mission_id,
            "reviewer_instructions": REVIEWER_INSTRUCTIONS,
        }),
    );
    println!("{}", env.to_pretty());
    Ok(())
}

fn target_spec_value(
    paths: &MissionPaths,
    task_id: &str,
    task: Option<&PlanTask>,
) -> CliResult<Value> {
    let spec_path = task
        .and_then(|t| t.spec.clone())
        .unwrap_or_else(|| format!("specs/{task_id}/SPEC.md"));
    // `spec_path` in PLAN.yaml is a string relative to the mission dir,
    // e.g. `specs/T2/SPEC.md`. Resolve it against the mission dir for
    // the file read, but report the original string to the caller.
    let spec_absolute = resolve_spec_path(paths, &spec_path)?;
    let excerpt = read_excerpt(&spec_absolute).unwrap_or_default();
    Ok(json!({
        "task_id": task_id,
        "spec_path": spec_path,
        "spec_excerpt": excerpt,
    }))
}

fn resolve_spec_path(paths: &MissionPaths, spec_str: &str) -> CliResult<PathBuf> {
    resolve_existing_mission_file(paths, spec_str, "review_target.spec")
}

fn proof_path_string(paths: &MissionPaths, state: &MissionState, task_id: &str) -> Option<String> {
    if let Some(raw) = state
        .tasks
        .get(task_id)
        .and_then(|r| r.proof_path.as_deref())
    {
        let path = Path::new(raw);
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            paths.mission_dir.join(path)
        };
        if abs.is_file() {
            return Some(relative_from_repo(paths, &abs));
        }
    }
    let abs = paths.proof_file_for(task_id);
    if abs.is_file() {
        Some(relative_from_repo(paths, &abs))
    } else {
        None
    }
}

fn relative_from_repo(paths: &MissionPaths, abs: &Path) -> String {
    abs.strip_prefix(&paths.repo_root).map_or_else(
        |_| abs.to_string_lossy().into_owned(),
        |p| p.to_string_lossy().replace('\\', "/"),
    )
}

fn read_excerpt(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    const LIMIT: usize = 2_000;
    if raw.len() <= LIMIT {
        Some(raw)
    } else {
        let mut s = raw;
        s.truncate(LIMIT);
        s.push_str("\n…[truncated]");
        Some(s)
    }
}

/// Extract `interpreted_destination` from the YAML frontmatter of
/// OUTCOME.md. Returns `None` if the file is missing, has no
/// frontmatter, the key is absent, or the YAML fails to parse. Matches
/// the sibling implementations in `task/worker_packet.rs` and
/// `close/closeout.rs` — both tolerate parse failures silently because
/// the packet/closeout is an informational artifact, not a gate.
///
/// Prior implementations did substring scanning of the raw bytes, which
/// leaked the YAML block-scalar indicator (`|` / `>`) into the output
/// whenever whitespace between the key and the indicator defeated the
/// naive equality check. `serde_yaml::from_str` on the frontmatter
/// yields the parsed string body directly — no indicator leakage.
fn read_interpreted_destination(paths: &MissionPaths, outcome: &Path) -> Option<String> {
    ensure_artifact_file_read_safe(paths, outcome, "OUTCOME.md").ok()?;
    let raw = std::fs::read_to_string(outcome).ok()?;
    let frontmatter = extract_frontmatter(&raw)?;
    let doc: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;
    let dest = doc.get("interpreted_destination")?.as_str()?.trim();
    if dest.is_empty() {
        None
    } else {
        Some(dest.to_string())
    }
}

/// Pull the YAML frontmatter out of a markdown file. Returns the inner
/// block (without the `---` fences). If the whole file is YAML (no
/// leading `---`), returns the file verbatim. Tolerant of CRLF
/// line endings.
fn extract_frontmatter(raw: &str) -> Option<&str> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if let Some(rest) = trimmed.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            return Some(&rest[..end]);
        }
    }
    if let Some(rest) = trimmed.strip_prefix("---\r\n") {
        if let Some(end) = rest.find("\r\n---") {
            return Some(&rest[..end]);
        }
    }
    Some(trimmed)
}
