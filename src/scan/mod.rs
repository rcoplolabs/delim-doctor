mod fixer;
mod lexer;
mod path_guard;
mod report;
mod scanner;

use std::path::PathBuf;

use lexer::{detect_language, is_valid_language};
use path_guard::{PathError, validate_path};
use report::paginate;
use scanner::scan;

pub use report::{FixReport, ScanReport};

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error("failed to read file: {0}")]
    ReadFailed(String),
    #[error("invalid cursor '{0}': expected a non-negative integer")]
    InvalidCursor(String),
    #[error(
        "invalid language '{0}': accepted values are 'rust', 'python', 'javascript'/'js', 'typescript'/'ts', or 'generic'"
    )]
    UnsupportedLanguage(String),
}

pub struct ScanOptions {
    pub language: Option<String>,
    pub max_problems: usize,
    pub context_lines: usize,
    pub cursor: Option<String>,
    pub mode: ScanMode,
}

pub enum ScanMode {
    Scan,
    Fix { dry_run: bool },
}

pub enum ScanResult {
    Scanned(ScanReport),
    Fixed {
        report: FixReport,
        content: Option<String>,
    },
}

pub struct Scan {
    workspace_root: PathBuf,
}

impl Scan {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn run(&self, path: &str, opts: ScanOptions) -> Result<ScanResult, ScanError> {
        if let Some(ref lang) = opts.language
            && !is_valid_language(lang)
        {
            return Err(ScanError::UnsupportedLanguage(lang.clone()));
        }

        let canonical = validate_path(path, &self.workspace_root)?;

        let src = std::fs::read_to_string(&canonical)
            .map_err(|e| ScanError::ReadFailed(e.to_string()))?;

        let ext = canonical
            .extension()
            .map(|e| e.to_string_lossy().to_string());
        let lang = detect_language(ext.as_deref(), opts.language.as_deref());

        match opts.mode {
            ScanMode::Scan => {
                let offset = match opts.cursor.as_deref() {
                    None | Some("") => 0usize,
                    Some(s) => s
                        .parse::<usize>()
                        .map_err(|_| ScanError::InvalidCursor(s.to_string()))?,
                };

                let all_problems = scan(&src, opts.context_lines, &lang);
                let (problems, total, truncated, next_cursor) =
                    paginate(all_problems, offset, opts.max_problems);

                let report = ScanReport {
                    schema_version: 1,
                    file: canonical.to_string_lossy().to_string(),
                    language: lang,
                    total,
                    truncated,
                    next_cursor,
                    problems,
                };

                Ok(ScanResult::Scanned(report))
            }
            ScanMode::Fix { dry_run } => {
                let problems = scan(&src, 0, &lang);
                let plan = fixer::plan_fixes(&problems);
                let changed = !plan.applied.is_empty();

                let content = if !dry_run && changed {
                    Some(fixer::apply_fixes(&src, &plan))
                } else {
                    None
                };

                let report = FixReport {
                    schema_version: 1,
                    file: canonical.to_string_lossy().to_string(),
                    language: lang,
                    dry_run,
                    applied: plan.applied,
                    skipped: plan.skipped,
                    changed,
                };

                Ok(ScanResult::Fixed { report, content })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn make_workspace() -> PathBuf {
        let dir = std::env::temp_dir().join("delim_doctor_scan_test");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_file(workspace: &Path, name: &str, content: &str) -> PathBuf {
        let path = workspace.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn cleanup(path: &Path) {
        fs::remove_file(path).ok();
    }

    fn scan_opts(language: Option<&str>) -> ScanOptions {
        ScanOptions {
            language: language.map(String::from),
            max_problems: 20,
            context_lines: 1,
            cursor: None,
            mode: ScanMode::Scan,
        }
    }

    #[test]
    fn scan_finds_missing_brace() {
        let ws = make_workspace();
        let file = make_file(&ws, "missing_brace.rs", "fn main() {\n    let x = 1;\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(None));
        assert!(result.is_ok());
        let ScanResult::Scanned(report) = result.unwrap() else {
            panic!("expected Scanned")
        };
        assert_eq!(report.total, 1);
        assert_eq!(report.problems[0].ch, '{');
        assert_eq!(report.language, "rust");
        cleanup(&file);
    }

    #[test]
    fn scan_balanced_no_problems() {
        let ws = make_workspace();
        let file = make_file(&ws, "balanced_ok.rs", "fn main() {}\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(None));
        assert!(result.is_ok());
        let ScanResult::Scanned(report) = result.unwrap() else {
            panic!("expected Scanned")
        };
        assert_eq!(report.total, 0);
        cleanup(&file);
    }

    #[test]
    fn scan_path_outside_workspace_rejected() {
        let ws = make_workspace();
        let outside = std::env::temp_dir().join("delim_doctor_outside_test.rs");
        fs::write(&outside, "fn main() {}").unwrap();
        let scan = Scan::new(ws.clone());
        let result = scan.run(outside.to_str().unwrap(), scan_opts(None));
        assert!(matches!(result, Err(ScanError::Path(_))));
        fs::remove_file(&outside).ok();
    }

    #[test]
    fn scan_directory_rejected() {
        let ws = make_workspace();
        let dir = ws.join("dir.rs");
        fs::create_dir_all(&dir).unwrap();
        let scan = Scan::new(ws.clone());
        let result = scan.run(dir.to_str().unwrap(), scan_opts(None));
        assert!(matches!(result, Err(ScanError::Path(_))));
        fs::remove_dir(&dir).ok();
    }

    #[test]
    fn scan_bad_extension_rejected() {
        let ws = make_workspace();
        let file = make_file(&ws, "bad.txt", "content");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(None));
        assert!(matches!(result, Err(ScanError::Path(_))));
        cleanup(&file);
    }

    #[test]
    fn scan_unsupported_language_rejected() {
        let ws = make_workspace();
        let file = make_file(&ws, "bad_lang.rs", "fn main() {}");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(Some("ruby")));
        assert!(matches!(result, Err(ScanError::UnsupportedLanguage(_))));
        cleanup(&file);
    }

    #[test]
    fn scan_invalid_cursor_rejected() {
        let ws = make_workspace();
        let file = make_file(&ws, "bad_cursor.rs", "fn main() {}");
        let scan = Scan::new(ws.clone());
        let result = scan.run(
            file.to_str().unwrap(),
            ScanOptions {
                language: None,
                max_problems: 20,
                context_lines: 1,
                cursor: Some("abc".to_string()),
                mode: ScanMode::Scan,
            },
        );
        assert!(matches!(result, Err(ScanError::InvalidCursor(_))));
        cleanup(&file);
    }

    #[test]
    fn scan_pagination_works() {
        let ws = make_workspace();
        let src = "{\n{\n{\n".repeat(10);
        let file = make_file(&ws, "many.rs", &src);
        let scan = Scan::new(ws.clone());
        let result = scan.run(
            file.to_str().unwrap(),
            ScanOptions {
                language: Some("generic".to_string()),
                max_problems: 5,
                context_lines: 0,
                cursor: None,
                mode: ScanMode::Scan,
            },
        );
        let ScanResult::Scanned(report) = result.unwrap() else {
            panic!("expected Scanned")
        };
        assert_eq!(report.problems.len(), 5);
        assert!(report.truncated);
        assert!(report.next_cursor.is_some());
        cleanup(&file);
    }

    #[test]
    fn fix_dry_run_returns_no_content() {
        let ws = make_workspace();
        let file = make_file(&ws, "fix_dry.rs", "fn main() {\n    let x = 1;\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(
            file.to_str().unwrap(),
            ScanOptions {
                language: Some("rust".to_string()),
                max_problems: 0,
                context_lines: 0,
                cursor: None,
                mode: ScanMode::Fix { dry_run: true },
            },
        );
        let ScanResult::Fixed { report, content } = result.unwrap() else {
            panic!("expected Fixed")
        };
        assert!(content.is_none());
        assert!(report.changed);
        assert_eq!(report.applied.len(), 1);
        let original = fs::read_to_string(&file).unwrap();
        assert!(original.contains("let x = 1"));
        cleanup(&file);
    }

    #[test]
    fn fix_non_dry_run_returns_content() {
        let ws = make_workspace();
        let file = make_file(&ws, "fix_write.rs", "fn main() {\n    let x = 1;\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(
            file.to_str().unwrap(),
            ScanOptions {
                language: Some("rust".to_string()),
                max_problems: 0,
                context_lines: 0,
                cursor: None,
                mode: ScanMode::Fix { dry_run: false },
            },
        );
        let ScanResult::Fixed { report, content } = result.unwrap() else {
            panic!("expected Fixed")
        };
        assert!(content.is_some());
        assert!(report.changed);
        let fixed = content.unwrap();
        assert!(fixed.ends_with('}'));
        assert!(!fixed.ends_with("}\n"));
        cleanup(&file);
    }

    #[test]
    fn fix_balanced_source_no_changes() {
        let ws = make_workspace();
        let file = make_file(&ws, "fix_balanced.rs", "fn main() {}\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(
            file.to_str().unwrap(),
            ScanOptions {
                language: Some("rust".to_string()),
                max_problems: 0,
                context_lines: 0,
                cursor: None,
                mode: ScanMode::Fix { dry_run: false },
            },
        );
        let ScanResult::Fixed { report, content } = result.unwrap() else {
            panic!("expected Fixed")
        };
        assert!(!report.changed);
        assert!(content.is_none());
        assert!(report.applied.is_empty());
        cleanup(&file);
    }

    #[test]
    fn scan_language_auto_detect_from_extension() {
        let ws = make_workspace();
        let file = make_file(&ws, "detect_py.py", "x = [1, 2, 3\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(None));
        let ScanResult::Scanned(report) = result.unwrap() else {
            panic!("expected Scanned")
        };
        assert_eq!(report.language, "python");
        assert_eq!(report.total, 1);
        cleanup(&file);
    }

    #[test]
    fn scan_explicit_language_overrides_extension() {
        let ws = make_workspace();
        let file = make_file(&ws, "override_py.py", "fn main() {\n");
        let scan = Scan::new(ws.clone());
        let result = scan.run(file.to_str().unwrap(), scan_opts(Some("rust")));
        let ScanResult::Scanned(report) = result.unwrap() else {
            panic!("expected Scanned")
        };
        assert_eq!(report.language, "rust");
        cleanup(&file);
    }
}
