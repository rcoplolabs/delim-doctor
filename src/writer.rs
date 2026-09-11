use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
pub fn write_atomic_with_backup(path: &Path, content: &str) -> std::io::Result<()> {
    let backup = backup_path(path);
    fs::copy(path, &backup)?;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".delim-doctor-{}.tmp", std::process::id()));

    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::remove_file(path)?;
            fs::rename(&tmp, path)
        }
    }
}
