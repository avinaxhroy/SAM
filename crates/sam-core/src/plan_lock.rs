//! Advisory plan lock (§4.6 step 1, Phase 4).
//!
//! Serializes cooperating SAM processes against each other using advisory locking.
//! Writers create the `.sam/` bookkeeping layout; readers lock only when one exists,
//! allowing read-only plan roots to remain functional (§4.1).
//!
//! Bookkeeping lives under `.sam/` — ignored by `check_layout` and excluded
//! from canonical exports (§4.6).
//!
//! Implemented via `std::fs::File::try_lock` (`flock` on Unix, `LockFileEx` on Windows),
//! avoiding external locking crates (§9).

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::transaction::{TransactionError, ensure_layout};

#[derive(Debug)]
pub enum PlanLockError {
    /// Another SAM process holds the lock → CLI exit 3.
    Busy(String),
    /// Cannot open or create the lock file → CLI exit 4.
    Io(String),
}

impl std::fmt::Display for PlanLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanLockError::Busy(message) | PlanLockError::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PlanLockError {}

/// A held lock. `has_domain == false` is the read path on a plan root that has
/// never been written: no `.sam/`, so no transaction can be torn, and the lock
/// is a no-op.
pub struct PlanLock {
    pub root: PathBuf,
    pub has_domain: bool,
    file: Option<File>,
}

impl PlanLock {
    fn held(root: &Path, file: Option<File>, has_domain: bool) -> Self {
        Self {
            root: root.to_path_buf(),
            has_domain,
            file,
        }
    }

    /// `.sam/` exists, so a transaction has run here at least once.
    pub fn domain_exists(root: &Path) -> bool {
        root.join(".sam").is_dir()
    }

    /// Writers: create `.sam/`, open the lock read-write, take it exclusively.
    /// Retries briefly so a just-finishing writer yields a deterministic
    /// revision conflict instead of a lock race; then exit 3 (§4.9).
    pub fn acquire_write(root: &Path, retry: Duration) -> Result<Self, PlanLockError> {
        ensure_layout(root).map_err(|error| PlanLockError::Io(error.to_string()))?;
        Self::acquire(root, true, retry)
    }

    /// Readers: if a `.sam/` domain exists, take the same exclusive lock —
    /// recovery may mutate, and one snapshot publishes only after commit
    /// (§4.6 step 5). Without `.sam/` reads stay lock-free.
    pub fn acquire_read(root: &Path, retry: Duration) -> Result<Self, PlanLockError> {
        if !Self::domain_exists(root) {
            return Ok(Self::held(root, None, false));
        }
        Self::acquire(root, false, retry)
    }

    fn acquire(root: &Path, write: bool, retry: Duration) -> Result<Self, PlanLockError> {
        let path = root.join(".sam/lock");
        let mut options = OpenOptions::new();
        options.read(true);
        if write {
            options.write(true).create(true);
        }
        let file = options.open(&path).map_err(|error| {
            PlanLockError::Io(format!("cannot open {} ({error})", path.display()))
        })?;

        let deadline = Instant::now() + retry;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self::held(root, Some(file), true)),
                Err(TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(PlanLockError::Busy(
                            "another SAM process holds the plan lock — retry, or exit 3".into(),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(TryLockError::Error(error)) => {
                    return Err(PlanLockError::Io(format!("flock failed ({error})")));
                }
            }
        }
    }
}

impl Drop for PlanLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = file.unlock();
        }
    }
}

/// The writer's retry window, and the reader's. Readers wait longer because a
/// reader arriving mid-commit should see the committed state, not a conflict.
pub const WRITE_RETRY: Duration = Duration::from_millis(250);
pub const READ_RETRY: Duration = Duration::from_millis(2000);

/// One recovery hop for callers that hold a lock and need the engine's
/// `TransactionError` shape rather than the lock's.
pub fn recover_or_report(root: &Path) -> Result<(), PlanLockError> {
    crate::transaction::recover(root)
        .map(|_| ())
        .map_err(|error| match error {
            TransactionError::Io(message) => PlanLockError::Io(message),
            TransactionError::ExternalConflict(message) => PlanLockError::Io(message),
            TransactionError::RecoveryConflict(conflicts) => PlanLockError::Io(
                conflicts
                    .into_iter()
                    .map(|conflict| format!("tx {}: {}", conflict.txid, conflict.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        // `make_txid` serialises a process-wide counter, so two tests that want
        // the same `name` still stage into different directories.
        let dir = std::env::temp_dir().join(format!(
            "sam-lock-{name}-{}",
            crate::transaction::make_txid()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_second_writer_is_refused_while_the_first_holds() {
        let root = temp_dir("second-writer");
        let first = PlanLock::acquire_write(&root, Duration::from_millis(10))
            .expect("the first writer takes the lock");
        let second = PlanLock::acquire_write(&root, Duration::from_millis(10));
        assert!(
            matches!(second, Err(PlanLockError::Busy(_))),
            "a cooperating writer never proceeds on a held lock"
        );
        drop(first);
        assert!(
            PlanLock::acquire_write(&root, Duration::from_millis(10)).is_ok(),
            "the lock is released when the holder drops"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_read_on_a_plan_without_dot_sam_is_a_no_op() {
        let root = temp_dir("read-noop");
        let lock = PlanLock::acquire_read(&root, Duration::from_millis(10)).expect("read lock");
        assert!(!lock.has_domain, "no .sam/ means no transaction ever ran");
        assert!(
            !root.join(".sam").exists(),
            "a read never creates bookkeeping"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_reader_waits_for_a_writer_and_then_proceeds() {
        let root = temp_dir("reader-waits");
        let writer = PlanLock::acquire_write(&root, Duration::from_millis(10)).expect("write lock");
        let released = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            drop(writer);
        });
        let reader = PlanLock::acquire_read(&root, Duration::from_secs(2));
        assert!(reader.is_ok(), "the reader waits rather than failing");
        released.join().expect("the writer thread finishes");
        let _ = std::fs::remove_dir_all(&root);
    }
}
