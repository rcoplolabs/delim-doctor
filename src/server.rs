use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, tool, tool_router};

use crate::scan::{Scan, ScanError, ScanMode, ScanOptions, ScanResult};
use crate::writer;

pub struct DelimDoctorServer {
    pub workspace_root: PathBuf,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScanParams {
    /// Source file path, absolute or relative to --workspace-root
    pub path: String,
    /// Optional: "rust", "python", "javascript"/"js", "typescript"/"ts", or "generic".
    /// Defaults to extension-based detection.
    pub language: Option<String>,
    /// Optional: max number of problems to report. Default 20.
    pub max_problems: Option<usize>,
    /// Optional: context lines around each problem. Default 1.
    pub context_lines: Option<usize>,
    /// Optional: pagination cursor from a previous response's next_cursor.
    pub cursor: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FixParams {
    /// Source file path, absolute or relative to --workspace-root
    pub path: String,
    /// Optional: "rust", "python", "javascript"/"js", "typescript"/"ts", or "generic".
    /// Defaults to extension-based detection.
    pub language: Option<String>,
    /// Optional: when true (default) only report the fixes, do not write.
    pub dry_run: Option<bool>,
}

#[tool_router(server_handler)]
impl DelimDoctorServer {
    #[tool(
        name = "delim_scan",
        description = "Scan source file for bracket/delimiter imbalance, pinpointing line:col. Returns JSON structured data + text summary."
    )]
    fn delim_scan(
        &self,
        Parameters(ScanParams {
            path,
            language,
            max_problems,
            context_lines,
            cursor,
        }): Parameters<ScanParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let scan = Scan::new(self.workspace_root.clone());
        let result = scan
            .run(
                &path,
                ScanOptions {
                    language,
                    max_problems: max_problems.unwrap_or(20),
                    context_lines: context_lines.unwrap_or(1),
                    cursor,
                    mode: ScanMode::Scan,
                },
            )
            .map_err(scan_error_to_error_data)?;

        let ScanResult::Scanned(report) = result else {
            unreachable!("ScanMode::Scan must return Scanned");
        };
        Ok(report_to_result(&report, report.text_summary()))
    }

    #[tool(
        name = "delim_fix",
        description = "Conservative delimiter fix. Deletes stray closing delimiters and appends missing closers at end-of-file. Mid-file mismatches are reported as skipped, never auto-fixed. dry_run defaults to true (report only)."
    )]
    fn delim_fix(
        &self,
        Parameters(FixParams {
            path,
            language,
            dry_run,
        }): Parameters<FixParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let scan = Scan::new(self.workspace_root.clone());
        let result = scan
            .run(
                &path,
                ScanOptions {
                    language,
                    max_problems: 0,
                    context_lines: 0,
                    cursor: None,
                    mode: ScanMode::Fix {
                        dry_run: dry_run.unwrap_or(true),
                    },
                },
            )
            .map_err(scan_error_to_error_data)?;

        let ScanResult::Fixed { report, content } = result else {
            unreachable!("ScanMode::Fix must return Fixed");
        };

        if let Some(fixed) = content {
            let file_path = PathBuf::from(&report.file);
            writer::write_atomic_with_backup(&file_path, &fixed).map_err(|e| {
                ErrorData::internal_error(format!("failed to write file: {}", e), None)
            })?;
        }

        Ok(report_to_result(&report, report.text_summary()))
    }
}

fn scan_error_to_error_data(e: ScanError) -> ErrorData {
    match e {
        ScanError::UnsupportedLanguage(_) | ScanError::InvalidCursor(_) => {
            ErrorData::invalid_params(e.to_string(), None)
        }
        _ => ErrorData::internal_error(e.to_string(), None),
    }
}

fn report_to_result(report: &impl serde::Serialize, summary: String) -> CallToolResult {
    // unwrap_or_default is safe here: the report types contain only basic
    // serializable types (String, usize, bool, char, Option<char>) that
    // cannot fail to serialize. If a future field type breaks this
    // invariant, the empty fallback prevents a server-wide panic.
    let json_str = serde_json::to_string_pretty(report).unwrap_or_default();
    let json_value = serde_json::to_value(report).unwrap_or_default();

    let mut result = CallToolResult::success(vec![
        ContentBlock::text(json_str),
        ContentBlock::text(summary),
    ]);
    result.structured_content = Some(json_value);
    result
}
