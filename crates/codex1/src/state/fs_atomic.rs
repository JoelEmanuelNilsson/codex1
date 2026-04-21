//! Atomic-write helper.
//!
//! Writes `data` into a temp file in the same directory as `target`,
//! then renames. Same-filesystem rename on Unix is atomic; on Windows
//! it is effectively so via `MoveFileEx`. The temp file is orphaned if
//! we crash mid-write; `target` is unchanged.

use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::core::error::CliError;

/// Atomically write `data` to `target`. Creates parent directories as
/// needed. Preserves nothing of the previous file's mode; files under
/// `PLANS/` are treated as plain user-owned mission artifacts.
///
/// After `persist`, the parent directory is fsynced so the rename entry
/// is durable on Linux/ext4/xfs; on macOS `fsync` on a directory is a
/// no-op returning Ok, so this does not regress darwin behavior.
pub fn atomic_write(target: &Path, data: &[u8]) -> Result<(), CliError> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let dir = target.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(data)?;
    tmp.as_file_mut().sync_data()?;
    tmp.persist(target).map_err(|e| CliError::Io(e.error))?;
    // Fsync the containing directory so the rename entry survives a
    // power loss that follows `persist`'s return.
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }
    Ok(())
}
