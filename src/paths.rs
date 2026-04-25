use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::error::{Codex1Error, IoContext, Result};

pub fn validate_mission_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(Codex1Error::MissionPath(
            "mission id must not be empty".into(),
        ));
    }
    if id.starts_with('.') {
        return Err(Codex1Error::MissionPath(
            "mission id must not start with a dot".into(),
        ));
    }
    if id.contains('\0') || id.contains('/') || id.contains('\\') {
        return Err(Codex1Error::MissionPath(
            "mission id must not contain path separators or NUL bytes".into(),
        ));
    }
    if id == "." || id == ".." || id.contains("..") {
        return Err(Codex1Error::MissionPath(
            "mission id must not contain dot segments".into(),
        ));
    }
    let valid = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if !valid {
        return Err(Codex1Error::MissionPath(
            "mission id may only contain ASCII letters, digits, '-' and '_'".into(),
        ));
    }
    Ok(())
}

pub fn discover_repo_root(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return absolutize_existing_or_parent(&path);
    }

    let current = env::current_dir().io_context("failed to read current directory")?;
    discover_repo_root_from(&current)
}

pub fn discover_repo_root_from(start: &Path) -> Result<PathBuf> {
    let original = if start.exists() {
        fs::canonicalize(start).io_context(format!("failed to canonicalize {}", start.display()))?
    } else {
        absolutize_existing_or_parent(start)?
    };
    let mut current = original.clone();
    loop {
        if current.join(".git").exists() || current.join("Cargo.toml").exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Ok(original)
}

fn absolutize_existing_or_parent(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path)
            .io_context(format!("failed to canonicalize {}", path.display()));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().unwrap_or_else(|| OsStr::new(""));
    let parent = fs::canonicalize(parent).io_context(format!(
        "failed to canonicalize parent {}",
        parent.display()
    ))?;
    Ok(parent.join(name))
}

pub fn safe_join(base: &Path, relative: impl AsRef<Path>) -> Result<PathBuf> {
    let relative = relative.as_ref();
    if relative.is_absolute() {
        return Err(Codex1Error::MissionPath(format!(
            "absolute path is not allowed: {}",
            relative.display()
        )));
    }
    for component in relative.components() {
        match component {
            Component::Normal(part) if part != OsStr::new("") => {}
            _ => {
                return Err(Codex1Error::MissionPath(format!(
                    "unsafe relative path: {}",
                    relative.display()
                )))
            }
        }
    }
    let joined = base.join(relative);
    ensure_contained_for_write(base, &joined)?;
    Ok(joined)
}

pub fn create_dir_all_contained(base: &Path, relative: impl AsRef<Path>) -> Result<PathBuf> {
    let base_real =
        fs::canonicalize(base).io_context(format!("failed to canonicalize {}", base.display()))?;
    let relative = relative.as_ref();
    if relative.as_os_str().is_empty() {
        return Ok(base_real);
    }
    if relative.is_absolute() {
        return Err(Codex1Error::MissionPath(format!(
            "absolute path is not allowed: {}",
            relative.display()
        )));
    }

    let mut current = base_real.clone();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(Codex1Error::MissionPath(format!(
                "unsafe directory path: {}",
                relative.display()
            )));
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(Codex1Error::MissionPath(format!(
                        "directory path contains symlink: {}",
                        current.display()
                    )));
                }
                if !metadata.is_dir() {
                    return Err(Codex1Error::MissionPath(format!(
                        "directory path is not a directory: {}",
                        current.display()
                    )));
                }
                let current_real = fs::canonicalize(&current)
                    .io_context(format!("failed to canonicalize {}", current.display()))?;
                if !current_real.starts_with(&base_real) {
                    return Err(Codex1Error::MissionPath(format!(
                        "directory escapes base: {}",
                        current.display()
                    )));
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .io_context(format!("failed to create {}", current.display()))?;
            }
            Err(error) => {
                return Err(Codex1Error::Io {
                    context: format!("failed to inspect {}", current.display()),
                    source: error,
                });
            }
        }
    }
    Ok(current)
}

pub fn ensure_existing_contained(base: &Path, path: &Path) -> Result<()> {
    let base_real =
        fs::canonicalize(base).io_context(format!("failed to canonicalize {}", base.display()))?;
    let metadata =
        fs::symlink_metadata(path).io_context(format!("failed to inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(Codex1Error::MissionPath(format!(
            "path must not be a symlink: {}",
            path.display()
        )));
    }
    let real =
        fs::canonicalize(path).io_context(format!("failed to canonicalize {}", path.display()))?;
    if !real.starts_with(&base_real) {
        return Err(Codex1Error::MissionPath(format!(
            "path escapes base: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn ensure_contained_for_write(base: &Path, target: &Path) -> Result<()> {
    let base_real = fs::canonicalize(base)
        .io_context(format!("failed to canonicalize base {}", base.display()))?;
    let parent = target
        .parent()
        .ok_or_else(|| Codex1Error::MissionPath("target path has no parent".into()))?;
    let parent_real = if parent.exists() {
        fs::canonicalize(parent).io_context(format!(
            "failed to canonicalize parent {}",
            parent.display()
        ))?
    } else {
        let mut existing = parent;
        let mut missing = Vec::new();
        while !existing.exists() {
            let name = existing
                .file_name()
                .ok_or_else(|| Codex1Error::MissionPath("unsafe missing path".into()))?;
            missing.push(name.to_owned());
            existing = existing
                .parent()
                .ok_or_else(|| Codex1Error::MissionPath("unsafe missing path".into()))?;
        }
        let mut real = fs::canonicalize(existing)
            .io_context(format!("failed to canonicalize {}", existing.display()))?;
        for item in missing.iter().rev() {
            real.push(item);
        }
        real
    };
    if !parent_real.starts_with(&base_real) {
        return Err(Codex1Error::MissionPath(format!(
            "target escapes mission directory: {}",
            target.display()
        )));
    }
    if target.exists() {
        let target_real = fs::canonicalize(target).io_context(format!(
            "failed to canonicalize target {}",
            target.display()
        ))?;
        if !target_real.starts_with(&base_real) {
            return Err(Codex1Error::MissionPath(format!(
                "target escapes mission directory: {}",
                target.display()
            )));
        }
    }
    Ok(())
}

pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for c in input.chars() {
        let next = if c.is_ascii_alphanumeric() {
            Some(c.to_ascii_lowercase())
        } else if matches!(c, '-' | '_' | ' ' | '.' | ':') {
            Some('-')
        } else {
            None
        };
        if let Some(c) = next {
            if c == '-' {
                if !last_dash && !out.is_empty() {
                    out.push(c);
                    last_dash = true;
                }
            } else {
                out.push(c);
                last_dash = false;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "artifact".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_mission_ids() {
        for id in [
            "",
            ".",
            "..",
            ".hidden",
            "../x",
            "x/y",
            "x\\y",
            "a..b",
            "hello world",
        ] {
            assert!(validate_mission_id(id).is_err(), "{id}");
        }
    }

    #[test]
    fn accepts_boring_mission_ids() {
        for id in ["mission", "mission-1", "mission_1", "M1"] {
            validate_mission_id(id).unwrap();
        }
    }
}
