//! `codex1 doctor` — health report. Never crashes on missing auth.

use serde_json::json;

use crate::cli::Ctx;
use crate::core::config::default_config_path;
use crate::core::envelope::JsonOk;
use crate::core::error::CliResult;

pub fn run(_ctx: &Ctx) -> CliResult<()> {
    let version = env!("CARGO_PKG_VERSION");
    let config_path = default_config_path();
    let config_exists = config_path.as_ref().is_some_and(|p| p.is_file());
    let installed_path = which_codex1();
    let home_local_bin =
        std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local").join("bin"));
    let local_bin_writable = home_local_bin
        .as_ref()
        .is_some_and(|p| p.is_dir() && writable(p));
    let cwd = std::env::current_dir().ok();

    let env = JsonOk::global(json!({
        "version": version,
        "config": {
            "path": config_path,
            "exists": config_exists,
        },
        "install": {
            "codex1_on_path": installed_path,
            "home_local_bin": home_local_bin,
            "home_local_bin_writable": local_bin_writable,
        },
        "auth": {
            "required": false,
            "notes": "Codex1 is a local mission harness; no auth is required by default.",
        },
        "cwd": cwd,
        "warnings": warnings(),
    }));
    println!("{}", env.to_pretty());
    Ok(())
}

fn warnings() -> Vec<String> {
    let mut warnings = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        if is_network_mount(&cwd) {
            warnings.push(
                "Current directory appears to live on a network filesystem; \
                 fs2 locks may behave unexpectedly. Prefer local disk for mission state."
                    .to_string(),
            );
        }
    }
    warnings
}

fn is_network_mount(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy().to_ascii_lowercase();
    s.contains("/mnt/") || s.contains("/net/") || s.contains("//") || s.starts_with("//")
}

fn which_codex1() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("codex1");
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        meta.is_file() && (meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn writable(path: &std::path::Path) -> bool {
    let probe = path.join(".codex1-doctor-probe");
    match std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}
