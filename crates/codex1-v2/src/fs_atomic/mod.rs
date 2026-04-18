//! Atomic filesystem primitive used by `state` and `events`.
//!
//! Two capabilities:
//!
//! * [`LockedDir`] — acquires an exclusive fs2 file lock on a per-mission
//!   `.state.lock` so `StateStore::mutate` is the only in-process path that
//!   can mutate `STATE.json` and `events.jsonl` at the same time.
//! * [`atomic_write`] — writes bytes to a tempfile in the target's directory,
//!   fsyncs the tempfile, atomically renames it into place, then fsyncs the
//!   parent directory so the new inode survives a crash.

// Call sites for these helpers land in T4 (`state`) and T11 (`init`).
#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fs2::FileExt;
use tempfile::NamedTempFile;

/// Name of the lock file, created on demand inside the mission directory.
pub const LOCK_FILENAME: &str = ".state.lock";

/// Exclusive file-lock guard on a directory's `.state.lock`. Drop releases
/// the lock (fs2 unlocks on `File` close).
pub struct LockedDir {
    // Kept alive for the duration of the lock; never read directly.
    _file: File,
    path: PathBuf,
}

impl LockedDir {
    /// Acquire an exclusive fs2 lock on `<dir>/.state.lock`. Creates the lock
    /// file if it doesn't exist. Blocks the current thread until the lock is
    /// held.
    pub fn acquire(dir: &Path) -> io::Result<Self> {
        let lock_path = dir.join(LOCK_FILENAME);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        FileExt::lock_exclusive(&file)?;
        Ok(Self {
            _file: file,
            path: lock_path,
        })
    }

    /// Path to the lock file (useful for diagnostics).
    #[must_use]
    pub fn lock_path(&self) -> &Path {
        &self.path
    }
}

/// Atomically write `content` to `path`.
///
/// The tempfile is created in the same directory as the target so the
/// rename is a same-filesystem operation (atomic on macOS and Linux).
/// After rename, the parent directory is fsynced so the new inode is
/// durably visible.
pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    let dir_file = File::open(dir)?;
    dir_file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, LockedDir, LOCK_FILENAME};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn acquire_creates_lock_file() {
        let dir = tempdir().unwrap();
        let lock = LockedDir::acquire(dir.path()).unwrap();
        assert!(lock.lock_path().exists());
        assert!(lock.lock_path().ends_with(LOCK_FILENAME));
    }

    #[test]
    fn second_acquire_on_same_dir_is_exclusive_after_drop() {
        let dir = tempdir().unwrap();
        let first = LockedDir::acquire(dir.path()).unwrap();
        drop(first);
        let second = LockedDir::acquire(dir.path()).unwrap();
        assert!(second.lock_path().exists());
    }

    #[test]
    fn atomic_write_creates_file_with_content() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("out.txt");
        atomic_write(&target, b"hello").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"hello");
    }

    #[test]
    fn atomic_write_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("out.txt");
        atomic_write(&target, b"first").unwrap();
        atomic_write(&target, b"second").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
    }

    #[test]
    fn atomic_write_leaves_no_tempfiles() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("out.txt");
        atomic_write(&target, b"data").unwrap();
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries, vec!["out.txt".to_string()]);
    }

    #[test]
    fn atomic_write_creates_missing_parent_dirs() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("nested/deeper/out.txt");
        atomic_write(&target, b"ok").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"ok");
    }
}
