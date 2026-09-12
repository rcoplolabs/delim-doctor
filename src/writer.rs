use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter ensuring each concurrent `write_atomic_with_backup` call
/// uses a unique temp file name, even when targeting files in the same
/// directory from different tokio worker threads.
static FIX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Backup file path: `<path>.bak`.
pub fn backup_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".bak");
    PathBuf::from(s)
}

/// Back up `path`, then replace its contents atomically where the platform
/// allows it (temp file + rename, same directory).
///
/// On Windows, `rename` cannot overwrite an existing file, so it falls back to
/// remove-then-rename. The backup taken beforehand preserves the original.
/// If the final rename still fails, the backup is restored to avoid data loss.
///
/// The `.bak` file is intentionally left on disk after a successful write as
/// an undo mechanism — the user can restore the original if the fix is wrong.
/// The temp file is cleaned up on all error paths.
pub fn write_atomic_with_backup(path: &Path, content: &str) -> std::io::Result<()> {
    let backup = backup_path(path);
    fs::copy(path, &backup)?;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let seq = FIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".delim-doctor-{}-{}.tmp", std::process::id(), seq));

    // Write to the temp file. If any step fails, clean up the temp file
    // before propagating the error.
    let write_result: std::io::Result<()> = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
        return write_result;
    }

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::remove_file(path)?;
            match fs::rename(&tmp, path) {
                Ok(()) => Ok(()),
                Err(_) => {
                    // Both rename attempts failed. Restore original from backup.
                    let _ = fs::remove_file(&tmp);
                    fs::copy(&backup, path)?;
                    Err(std::io::Error::other(
                        "atomic write failed; original restored from backup",
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("delim_doctor_writer_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_backup_path() {
        let p = Path::new("/tmp/foo.rs");
        assert_eq!(backup_path(p), PathBuf::from("/tmp/foo.rs.bak"));
    }

    #[test]
    fn test_write_and_read_back() {
        let dir = make_temp_dir();
        let file = dir.join("test_write.rs");
        fs::write(&file, "original content").unwrap();

        write_atomic_with_backup(&file, "new content").unwrap();

        let result = fs::read_to_string(&file).unwrap();
        assert_eq!(result, "new content");

        // Backup should exist with original content.
        let bak = backup_path(&file);
        assert!(bak.exists(), "backup file should exist");
        assert_eq!(fs::read_to_string(&bak).unwrap(), "original content");

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_two_writes_same_directory() {
        // Verify that writing two different files in the same directory
        // (the scenario that was broken by the shared temp file name) works
        // correctly — each file gets its own content.
        let dir = make_temp_dir();
        let file_a = dir.join("a.rs");
        let file_b = dir.join("b.rs");
        fs::write(&file_a, "content A original").unwrap();
        fs::write(&file_b, "content B original").unwrap();

        write_atomic_with_backup(&file_a, "content A fixed").unwrap();
        write_atomic_with_backup(&file_b, "content B fixed").unwrap();

        assert_eq!(fs::read_to_string(&file_a).unwrap(), "content A fixed");
        assert_eq!(fs::read_to_string(&file_b).unwrap(), "content B fixed");

        for f in [&file_a, &file_b] {
            let _ = fs::remove_file(f);
            let _ = fs::remove_file(backup_path(f));
        }
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_write_creates_backup_before_replacing() {
        let dir = make_temp_dir();
        let file = dir.join("backup_test.rs");
        fs::write(&file, "original").unwrap();

        write_atomic_with_backup(&file, "replaced").unwrap();

        // The backup should contain the original content, not the new content.
        let bak = backup_path(&file);
        assert!(bak.exists());
        assert_eq!(fs::read_to_string(&bak).unwrap(), "original");
        assert_eq!(fs::read_to_string(&file).unwrap(), "replaced");

        let _ = fs::remove_file(&file);
        let _ = fs::remove_file(&bak);
        let _ = fs::remove_dir(&dir);
    }
}
