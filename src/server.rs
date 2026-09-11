use std::path::PathBuf;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, tool, tool_router};

use crate::fixer;
use crate::lexer::detect_language;
use crate::path_guard::{PathError, validate_path};
use crate::report::{FixReport, ScanReport, paginate};
use crate::scanner;
use crate::writer;

pub struct DelimDoctorServer {
    pub workspace_root: PathBuf,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScanParams {
    /// Source file path, absolute or relative to --workspace-root
    pub path: String,
    /// Optional: "rust" or "generic". Defaults to extension-based detection.
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
    /// Optional: "rust" or "generic". Defaults to extension-based detection.
    pub language: Option<String>,
    /// Optional: when true (default) only report the fixes, do not write.
    pub dry_run: Option<bool>,
}

fn validate_language(language: &Option<String>) -> Result<(), ErrorData> {
    if let Some(lang) = language
        && lang != "rust"
        && lang != "generic"
    {
        return Err(ErrorData::invalid_params(
            format!(
                "invalid language '{}': only 'rust' and 'generic' are accepted",
                lang
            ),
            None,
        ));
    }
    Ok(())
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
        validate_language(&language)?;

        let canonical = match validate_path(&path, &self.workspace_root) {
            Ok(c) => c,
            Err(e) => return Ok(path_error_result(e)),
        };

        let src = match std::fs::read_to_string(&canonical) {
            Ok(s) => s,
            Err(e) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "delim_scan error: failed to read file: {}",
                    e
                ))]));
            }
        };

        let ext = canonical
            .extension()
            .map(|e| e.to_string_lossy().to_string());
        let lang = detect_language(ext.as_deref(), language.as_deref());

        let max = max_problems.unwrap_or(20);
        let ctx = context_lines.unwrap_or(1);

        let offset = match cursor.as_deref() {
            None | Some("") => 0usize,
            Some(s) => s.parse::<usize>().map_err(|_| {
                ErrorData::invalid_params(
                    format!("invalid cursor '{}': expected a non-negative integer", s),
                    None,
                )
            })?,
        };

        let all_problems = scanner::scan(&src, ctx, &lang);
        let (problems, total, truncated, next_cursor) = paginate(all_problems, offset, max);

        let report = ScanReport {
            schema_version: 1,
            file: canonical.to_string_lossy().to_string(),
            language: lang,
            total,
            truncated,
            next_cursor,
            problems,
        };

        let json_str = serde_json::to_string_pretty(&report).unwrap_or_default();
        let summary = report.text_summary();
        let json_value = serde_json::to_value(&report).unwrap_or_default();

        let mut result = CallToolResult::success(vec![
            ContentBlock::text(json_str),
            ContentBlock::text(summary),
        ]);
        result.structured_content = Some(json_value);
        Ok(result)
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
        validate_language(&language)?;

        let canonical = match validate_path(&path, &self.workspace_root) {
            Ok(c) => c,
            Err(e) => return Ok(path_error_result(e)),
        };

        let src = match std::fs::read_to_string(&canonical) {
            Ok(s) => s,
            Err(e) => {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "delim_fix error: failed to read file: {}",
                    e
                ))]));
            }
        };

        let ext = canonical
            .extension()
            .map(|e| e.to_string_lossy().to_string());
        let lang = detect_language(ext.as_deref(), language.as_deref());
        let dry = dry_run.unwrap_or(true);

        let problems = scanner::scan(&src, 0, &lang);
        let plan = fixer::plan_fixes(&problems);
        let changed = !plan.applied.is_empty();

        if !dry && changed {
            let fixed = fixer::apply_fixes(&src, &plan);
            if let Err(e) = writer::write_atomic_with_backup(&canonical, &fixed) {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "delim_fix error: failed to write file: {}",
                    e
                ))]));
            }
        }

        let report = FixReport {
            schema_version: 1,
            file: canonical.to_string_lossy().to_string(),
            language: lang,
            dry_run: dry,
            applied: plan.applied,
            skipped: plan.skipped,
            changed,
        };

        let json_str = serde_json::to_string_pretty(&report).unwrap_or_default();
        let summary = report.text_summary();
        let json_value = serde_json::to_value(&report).unwrap_or_default();

        let mut result = CallToolResult::success(vec![
            ContentBlock::text(json_str),
            ContentBlock::text(summary),
        ]);
        result.structured_content = Some(json_value);
        Ok(result)
    }
}

fn path_error_result(e: PathError) -> CallToolResult {
    let msg = format!("delim_scan error: {}", e);
    CallToolResult::error(vec![ContentBlock::text(msg)])
}
