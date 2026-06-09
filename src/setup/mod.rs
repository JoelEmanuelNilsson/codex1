use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use crate::cli::{
    SetupBackupRestoreArgs, SetupBackupsCommand, SetupCommand, SetupRepoArgs, SetupStatusArgs,
};
use crate::envelope;
use crate::error::{Codex1Error, Result};
use crate::paths::discover_repo_root;

mod catalog;
mod guidance;
mod workspace;

use workspace::SetupPlan;

#[derive(Clone, Debug, Serialize)]
pub struct SetupStatus {
    pub repo: PathBuf,
    pub marker: SetupFileState,
    pub skill: SetupFileState,
    pub skills: Vec<SetupManagedSkill>,
    pub supporting_doc: SetupFileState,
    pub supporting_docs: Vec<SetupManagedDoc>,
    pub guidance: SetupFileState,
    pub repo_bundle_materialized: bool,
    pub backups_available: usize,
    pub warnings: Vec<String>,
    pub anti_oracle: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupManagedSkill {
    pub path: &'static str,
    pub state: SetupFileState,
}

#[derive(Clone, Debug, Serialize)]
pub struct SetupManagedDoc {
    pub path: &'static str,
    pub state: SetupFileState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetupFileState {
    Current,
    Missing,
    Stale,
    Invalid,
}

pub fn run(cli_json: bool, global_repo: Option<PathBuf>, command: SetupCommand) -> Result<()> {
    match command {
        SetupCommand::Install(mut args) | SetupCommand::Enable(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, install(args)?)
        }
        SetupCommand::Disable(mut args) | SetupCommand::Uninstall(mut args) => {
            args.repo = args.repo.or_else(|| global_repo.clone());
            emit(cli_json, uninstall(args)?)
        }
        SetupCommand::Status(mut args) => {
            args.repo = args.repo.or(global_repo);
            emit(cli_json, status_value(args)?)
        }
        SetupCommand::Doctor(mut args) => {
            args.repo = args.repo.or(global_repo);
            emit(cli_json, doctor(args)?)
        }
        SetupCommand::Backups { command } => match command {
            SetupBackupsCommand::List => {
                let args = SetupStatusArgs { repo: global_repo };
                emit(cli_json, backups_list(args)?)
            }
            SetupBackupsCommand::Restore(mut args) => {
                args.repo = args.repo.or(global_repo);
                emit(cli_json, backups_restore(args)?)
            }
        },
    }
}

fn install(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let mut plan = SetupPlan::new(args.dry_run);
    materialize_bundle(&repo, &mut plan, args.dry_run)?;
    Ok(json!({
        "summary": "setup installed repo-scoped Codex1 guidance",
        "repo": repo,
        "plan": plan,
        "anti_oracle": "setup materializes artifact workflow guidance only",
    }))
}

fn uninstall(args: SetupRepoArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let mut plan = SetupPlan::new(args.dry_run);
    remove_bundle(&repo, &mut plan, args.dry_run)?;
    Ok(json!({
        "summary": "setup removed repo-scoped Codex1 guidance",
        "repo": repo,
        "plan": plan,
        "anti_oracle": "setup removal does not delete mission artifacts",
    }))
}

fn status_value(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let status = status(args.repo)?;
    Ok(json!({
        "summary": "setup status complete",
        "status": status,
    }))
}

fn doctor(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let status = status(args.repo)?;
    let backup_manifest = match workspace::read_manifest(&status.repo) {
        Ok(_) => json!({"name": "backup_manifest", "ok": true}),
        Err(error) => {
            json!({"name": "backup_manifest", "ok": false, "error": error.to_string()})
        }
    };
    let mut checks = vec![
        json!({"name": "bundle_marker", "ok": status.marker == SetupFileState::Current}),
        json!({"name": "managed_skill", "ok": status.skill == SetupFileState::Current}),
        json!({"name": "managed_supporting_doc", "ok": status.supporting_doc == SetupFileState::Current}),
        json!({"name": "managed_guidance", "ok": status.guidance == SetupFileState::Current}),
        backup_manifest,
    ];
    checks.extend(status.skills.iter().map(|skill| {
        json!({
            "name": "managed_skill_file",
            "path": skill.path,
            "ok": skill.state == SetupFileState::Current,
        })
    }));
    checks.extend(status.supporting_docs.iter().map(|doc| {
        json!({
            "name": "managed_supporting_doc_file",
            "path": doc.path,
            "ok": doc.state == SetupFileState::Current,
        })
    }));
    Ok(json!({
        "summary": "setup doctor complete",
        "checks": checks,
        "status": status,
        "anti_oracle": "setup doctor diagnoses repo guidance mechanics only",
    }))
}

fn status(repo_arg: Option<PathBuf>) -> Result<SetupStatus> {
    let repo = resolve_repo(repo_arg)?;
    let marker = marker_state(&repo);
    let skills: Vec<_> = catalog::entries_by_role(catalog::BundleFileRole::ManagedSkill)
        .map(|entry| SetupManagedSkill {
            path: entry.relative,
            state: owned_file_state(&repo, entry.relative),
        })
        .collect();
    let skill = aggregate_states(skills.iter().map(|skill| skill.state));
    let supporting_docs: Vec<_> = catalog::entries_by_role(catalog::BundleFileRole::SupportingDoc)
        .map(|entry| SetupManagedDoc {
            path: entry.relative,
            state: owned_file_state(&repo, entry.relative),
        })
        .collect();
    let supporting_doc = aggregate_states(supporting_docs.iter().map(|doc| doc.state));
    let guidance = guidance_state(&repo);
    let mut warnings = Vec::new();
    for (name, state) in [
        ("marker", marker),
        ("skill", skill),
        ("supporting_doc", supporting_doc),
        ("guidance", guidance),
    ] {
        match state {
            SetupFileState::Current | SetupFileState::Missing => {}
            SetupFileState::Stale => warnings.push(format!("{name} is stale")),
            SetupFileState::Invalid => warnings.push(format!("{name} is invalid")),
        }
    }
    let backups_available = workspace::read_manifest(&repo)
        .map(|manifest| manifest.records.len())
        .unwrap_or(0);
    Ok(SetupStatus {
        repo,
        marker,
        skill,
        skills,
        supporting_doc,
        supporting_docs,
        guidance,
        repo_bundle_materialized: marker == SetupFileState::Current
            && skill == SetupFileState::Current
            && supporting_doc == SetupFileState::Current
            && guidance == SetupFileState::Current,
        backups_available,
        warnings,
        anti_oracle: "setup status is mechanical and does not report mission or native goal state",
    })
}

fn backups_list(args: SetupStatusArgs) -> Result<serde_json::Value> {
    let repo = resolve_repo(args.repo)?;
    let manifest = workspace::read_manifest(&repo)?;
    Ok(json!({
        "summary": "setup backups listed",
        "repo": repo,
        "backups": manifest.records,
    }))
}

fn backups_restore(args: SetupBackupRestoreArgs) -> Result<serde_json::Value> {
    if !args.force && !args.dry_run {
        return Err(Codex1Error::SetupRestore(
            "setup backups restore requires --force".into(),
        ));
    }
    let repo = resolve_repo(args.repo)?;
    let manifest = workspace::read_manifest(&repo)?;
    let record = manifest
        .records
        .iter()
        .find(|record| record.id == args.id)
        .ok_or_else(|| Codex1Error::SetupRestore(format!("backup not found: {}", args.id)))?;
    workspace::ensure_restore_target(&repo, &record.target_path)?;
    let mut plan = SetupPlan::new(args.dry_run);
    if record.existed {
        let backup_path = record.backup_path.as_ref().ok_or_else(|| {
            Codex1Error::SetupRestore(format!("backup record {} has no file", record.id))
        })?;
        workspace::ensure_backup_file(&repo, backup_path)?;
        plan.writes.push(record.target_path.clone());
        if !args.dry_run {
            workspace::copy_backup_to_target(&repo, backup_path, &record.target_path)?;
        }
    } else {
        plan.removes.push(record.target_path.clone());
        workspace::restore_absence(&repo, &record.target_path, args.dry_run)?;
    }
    Ok(json!({
        "summary": "setup backup restored",
        "repo": repo,
        "record": record,
        "plan": plan,
    }))
}

fn emit(cli_json: bool, data: serde_json::Value) -> Result<()> {
    if cli_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope::success(data)).unwrap()
        );
    } else if let Some(summary) = data.get("summary").and_then(|value| value.as_str()) {
        println!("{summary}");
    } else {
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
    Ok(())
}

fn resolve_repo(repo_arg: Option<PathBuf>) -> Result<PathBuf> {
    discover_repo_root(repo_arg)
}

fn materialize_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let guidance = workspace::setup_target(repo, catalog::BUNDLE_GUIDANCE)?;
    let marker = workspace::setup_target(repo, catalog::BUNDLE_MARKER)?;
    let marker_data = workspace::read_bundle_marker(&marker)?;
    if marker_data
        .as_ref()
        .is_some_and(|marker| !catalog::is_managed_bundle_marker(marker))
    {
        return Err(Codex1Error::SetupBundle(
            "invalid Codex1 setup bundle marker".into(),
        ));
    }

    for entry in catalog::owned_file_entries() {
        let path = workspace::setup_target(repo, entry.relative)?;
        workspace::ensure_owned_file_writable(
            &path,
            &entry.expected_body(),
            catalog::marker_allows_file_repair(marker_data.as_ref(), entry.relative),
        )?;
    }

    for entry in catalog::owned_file_entries() {
        let path = workspace::setup_target(repo, entry.relative)?;
        workspace::write_owned_file(
            repo,
            &path,
            &entry.expected_body(),
            catalog::marker_allows_file_repair(marker_data.as_ref(), entry.relative),
            entry.role.materialize_reason(),
            plan,
            dry_run,
        )?;
    }
    remove_legacy_bundle_files_not_current(repo, marker_data.as_ref(), plan, dry_run)?;
    workspace::write_guidance_file(repo, &guidance, plan, dry_run)?;
    workspace::write_owned_file(
        repo,
        &marker,
        &catalog::bundle_marker_body(),
        marker_data
            .as_ref()
            .is_some_and(catalog::is_managed_bundle_marker),
        "bundle marker",
        plan,
        dry_run,
    )?;
    Ok(())
}

fn remove_legacy_bundle_files_not_current(
    repo: &Path,
    marker: Option<&catalog::BundleMarker>,
    plan: &mut SetupPlan,
    dry_run: bool,
) -> Result<()> {
    let Some(marker) = marker.filter(|marker| catalog::is_managed_bundle_marker(marker)) else {
        return Ok(());
    };
    for relative in &marker.files {
        if catalog::is_current_bundle_file(relative) {
            continue;
        }
        let path = workspace::setup_target(repo, relative)?;
        match fs::read_to_string(&path) {
            Ok(text) if catalog::is_managed_restore_body(relative, &text) => {
                workspace::remove_file_with_backup(
                    repo,
                    &path,
                    "remove legacy managed setup file",
                    plan,
                    dry_run,
                )?
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Codex1Error::SetupBundle(format!(
                    "failed to read {}: {error}",
                    path.display()
                )))
            }
        }
    }
    Ok(())
}

fn remove_bundle(repo: &Path, plan: &mut SetupPlan, dry_run: bool) -> Result<()> {
    let marker = workspace::setup_target(repo, catalog::BUNDLE_MARKER)?;
    let (files, strict) = match workspace::read_bundle_marker(&marker)? {
        Some(marker_data) => {
            if !catalog::is_managed_bundle_marker(&marker_data) {
                return Err(Codex1Error::SetupBundle(
                    "invalid Codex1 setup bundle marker".into(),
                ));
            }
            (marker_data.files, true)
        }
        None => (catalog::current_bundle_files(), false),
    };
    for relative in files {
        let path = workspace::setup_target(repo, &relative)?;
        if relative == catalog::BUNDLE_GUIDANCE {
            workspace::remove_guidance_if_owned(repo, &path, strict, plan, dry_run)?;
        } else {
            workspace::remove_owned_file_if_managed(repo, &path, &relative, strict, plan, dry_run)?;
        }
    }
    workspace::remove_bundle_marker_file(repo, &marker, plan, dry_run)?;
    Ok(())
}

fn marker_state(repo: &Path) -> SetupFileState {
    let Ok(path) = workspace::setup_target(repo, catalog::BUNDLE_MARKER) else {
        return SetupFileState::Invalid;
    };
    match workspace::read_bundle_marker(&path) {
        Ok(Some(marker)) if catalog::is_current_marker(&marker) => SetupFileState::Current,
        Ok(Some(marker)) if catalog::is_managed_bundle_marker(&marker) => SetupFileState::Stale,
        Ok(Some(_)) => SetupFileState::Invalid,
        Ok(None) => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn owned_file_state(repo: &Path, relative: &str) -> SetupFileState {
    let Ok(path) = workspace::setup_target(repo, relative) else {
        return SetupFileState::Invalid;
    };
    let Some(expected) = catalog::expected_body(relative) else {
        return SetupFileState::Invalid;
    };
    match fs::read_to_string(&path) {
        Ok(text) if text == expected => SetupFileState::Current,
        Ok(_) => SetupFileState::Stale,
        Err(error) if error.kind() == ErrorKind::NotFound => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn guidance_state(repo: &Path) -> SetupFileState {
    let Ok(path) = workspace::setup_target(repo, catalog::BUNDLE_GUIDANCE) else {
        return SetupFileState::Invalid;
    };
    match fs::read_to_string(&path) {
        Ok(text) if text == guidance::body() || text.contains(&guidance::managed_block()) => {
            SetupFileState::Current
        }
        Ok(text) if guidance::has_managed_block(&text) => SetupFileState::Stale,
        Ok(_) => SetupFileState::Missing,
        Err(error) if error.kind() == ErrorKind::NotFound => SetupFileState::Missing,
        Err(_) => SetupFileState::Invalid,
    }
}

fn aggregate_states(states: impl IntoIterator<Item = SetupFileState>) -> SetupFileState {
    let states: Vec<_> = states.into_iter().collect();
    if states.iter().all(|state| *state == SetupFileState::Current) {
        SetupFileState::Current
    } else if states.contains(&SetupFileState::Invalid) {
        SetupFileState::Invalid
    } else if states.iter().all(|state| *state == SetupFileState::Missing) {
        SetupFileState::Missing
    } else {
        SetupFileState::Stale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialize_bundle_upgrades_pre_handoff_release() {
        let repo = tempfile::tempdir().unwrap();
        let marker = repo.path().join(catalog::BUNDLE_MARKER);
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::create_dir_all(repo.path().join(".agents/skills/clarify")).unwrap();
        fs::write(
            repo.path().join(".agents/skills/clarify/SKILL.md"),
            "# Old managed clarify\n",
        )
        .unwrap();
        fs::write(&marker, catalog::legacy_marker_body_for_test(11)).unwrap();

        let mut plan = SetupPlan::new(false);
        materialize_bundle(repo.path(), &mut plan, false).unwrap();

        let status = status(Some(repo.path().to_path_buf())).unwrap();
        assert!(status.repo_bundle_materialized);
        assert!(repo
            .path()
            .join(".agents/skills/handoff/SKILL.md")
            .is_file());
        assert!(repo
            .path()
            .join(".agents/skills/handoff/agents/openai.yaml")
            .is_file());
        let clarify =
            fs::read_to_string(repo.path().join(".agents/skills/clarify/SKILL.md")).unwrap();
        assert!(clarify.contains("Relentlessly clarify"));
    }

    #[test]
    fn materialize_bundle_removes_retired_managed_file_with_body_proof() {
        let repo = tempfile::tempdir().unwrap();
        let retired = ".agents/skills/plan/EXECUTION-PROMPT-FORMAT.md";
        let retired_path = repo.path().join(retired);
        let marker = repo.path().join(catalog::BUNDLE_MARKER);
        fs::create_dir_all(retired_path.parent().unwrap()).unwrap();
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(&retired_path, catalog::expected_body(retired).unwrap()).unwrap();
        fs::write(&marker, catalog::legacy_marker_body_for_test(4)).unwrap();

        let mut plan = SetupPlan::new(false);
        materialize_bundle(repo.path(), &mut plan, false).unwrap();

        assert!(!retired_path.exists());
    }
}
