use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

const ALLOWED_EXTENSIONS: &[&str] = &[
    ".rs", ".go", ".js", ".ts", ".jsx", ".tsx", ".py", ".rb", ".java", ".c", ".cpp", ".h", ".hpp",
    ".cs", ".swift", ".kt", ".lua", ".sh", ".json", ".toml", ".yaml", ".yml",
];

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("path contains null bytes")]
    NullBytes,
    #[error("file extension not allowed: {0}")]
    BadExtension(String),
    #[error("path outside workspace root")]
    OutsideWorkspace,
    #[error("path points to a directory, not a file")]
    IsDirectory,
    #[error("path points to a device file")]
    DeviceFile,
    #[error("file too large: {0} bytes (max {MAX_FILE_SIZE})")]
    FileTooLarge(u64),
    #[error("failed to canonicalize path: {0}")]
    CanonicalizeFailed(String),
    #[error("failed to read file metadata: {0}")]
    MetadataFailed(String),
}

pub fn validate_path(path: &str, workspace_root: &Path) -> Result<PathBuf, PathError> {
    if path.contains('\0') {
        return Err(PathError::NullBytes);
    }

    let p = Path::new(path);
    if !has_allowed_extension(p) {
        return Err(PathError::BadExtension(
            p.extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default(),
        ));
    }

    if is_device_file(p) {
        return Err(PathError::DeviceFile);
    }

    let abs_path = if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace_root.join(p)
    };

    let canonical = abs_path
        .canonicalize()
        .map_err(|e| PathError::CanonicalizeFailed(e.to_string()))?;

    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|e| PathError::CanonicalizeFailed(e.to_string()))?;

    if !canonical.starts_with(&canonical_root) {
        return Err(PathError::OutsideWorkspace);
    }

    let metadata =
        std::fs::metadata(&canonical).map_err(|e| PathError::MetadataFailed(e.to_string()))?;

    if metadata.is_dir() {
        return Err(PathError::IsDirectory);
    }

    if is_device_file(&canonical) {
        return Err(PathError::DeviceFile);
    }

    if metadata.len() > MAX_FILE_SIZE {
        return Err(PathError::FileTooLarge(metadata.len()));
    }

    Ok(canonical)
}

fn has_allowed_extension(path: &Path) -> bool {
    let ext = match path.extension() {
        Some(e) => e.to_string_lossy().to_lowercase(),
        None => return false,
    };
    ALLOWED_EXTENSIONS.contains(&format!(".{}", ext).as_str())
}

fn is_device_file(path: &Path) -> bool {
    let stem = match path.file_stem() {
        Some(s) => s.to_string_lossy().to_uppercase(),
        None => return false,
    };

    #[cfg(target_os = "windows")]
    {
        matches!(
            stem.as_str(),
            "NUL"
                | "CON"
                | "PRN"
                | "AUX"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let path_str = path.to_string_lossy();
        path_str.starts_with("/dev/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn test_dir() -> PathBuf {
        let dir = env::temp_dir().join("delim_doctor_test");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_allowed_extension() {
        assert!(has_allowed_extension(Path::new("foo.rs")));
        assert!(has_allowed_extension(Path::new("foo.JS")));
        assert!(has_allowed_extension(Path::new("foo.TOML")));
        assert!(!has_allowed_extension(Path::new("foo.exe")));
        assert!(!has_allowed_extension(Path::new("foo")));
        assert!(!has_allowed_extension(Path::new("foo.txt")));
    }

    #[test]
    fn test_null_bytes_rejected() {
        let root = test_dir();
        let result = validate_path("foo\0.rs", &root);
        assert!(matches!(result, Err(PathError::NullBytes)));
    }

    #[test]
    fn test_bad_extension_rejected() {
        let root = test_dir();
        let result = validate_path("foo.exe", &root);
        assert!(matches!(result, Err(PathError::BadExtension(_))));
    }

    #[test]
    fn test_directory_rejected() {
        let root = test_dir();
        let subdir = root.join("subdir.rs");
        fs::create_dir_all(&subdir).unwrap();
        let result = validate_path(subdir.to_str().unwrap(), &root);
        // subdir.rs looks like a file by extension but is a dir
        // canonicalize will succeed, metadata.is_dir will catch it
        assert!(matches!(result, Err(PathError::IsDirectory)));
        fs::remove_dir(&subdir).ok();
    }

    #[test]
    fn test_outside_workspace_rejected() {
        let root = test_dir();
        let outside = env::temp_dir().join("delim_doctor_outside.rs");
        fs::write(&outside, "fn main() {}").unwrap();
        let result = validate_path(outside.to_str().unwrap(), &root);
        assert!(matches!(result, Err(PathError::OutsideWorkspace)));
        fs::remove_file(&outside).ok();
    }

    #[test]
    fn test_valid_file_accepted() {
        let root = test_dir();
        let file = root.join("valid.rs");
        fs::write(&file, "fn main() {}").unwrap();
        let result = validate_path(file.to_str().unwrap(), &root);
        assert!(result.is_ok());
        fs::remove_file(&file).ok();
    }

    #[test]
    fn test_relative_path_in_workspace() {
        let root = test_dir();
        let file = root.join("relative.rs");
        fs::write(&file, "fn main() {}").unwrap();
        let result = validate_path("relative.rs", &root);
        assert!(result.is_ok());
        fs::remove_file(&file).ok();
    }

    #[test]
    fn test_path_traversal_rejected() {
        let root = test_dir();
        let result = validate_path("../../etc/passwd.rs", &root);
        assert!(result.is_err());
    }
}
