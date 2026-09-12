use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemKind {
    UnexpectedClose,
    MissingClose,
}

#[derive(Debug, Clone, Serialize)]
pub struct DelimProblem {
    pub kind: ProblemKind,
    pub ch: char,
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
    pub expected: Option<char>,
    pub snippet: String,
    pub depth_at: usize,
    /// True only for MissingClose problems emitted at end of scan (stack still
    /// non-empty). These have an unambiguous insertion point at end-of-file.
    pub at_eof: bool,
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub schema_version: u32,
    pub file: String,
    pub language: String,
    pub total: usize,
    pub truncated: bool,
    pub next_cursor: Option<String>,
    pub problems: Vec<DelimProblem>,
}

impl ScanReport {
    pub fn text_summary(&self) -> String {
        if self.total == 0 {
            return "No issues found".to_string();
        }

        let missing = self
            .problems
            .iter()
            .filter(|p| p.kind == ProblemKind::MissingClose)
            .count();
        let unexpected = self
            .problems
            .iter()
            .filter(|p| p.kind == ProblemKind::UnexpectedClose)
            .count();

        let pagination_note = if self.truncated {
            format!(" [showing {} of {}]", self.problems.len(), self.total)
        } else if self.problems.len() < self.total {
            format!(" [page: {} of {} total]", self.problems.len(), self.total)
        } else {
            String::new()
        };

        format!(
            "{} issue{} found ({} MissingClose, {} UnexpectedClose){}",
            self.total,
            if self.total == 1 { "" } else { "s" },
            missing,
            unexpected,
            pagination_note
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FixActionKind {
    Delete,
    Insert,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedFix {
    pub action: FixActionKind,
    pub ch: char,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkippedFix {
    pub kind: ProblemKind,
    pub ch: char,
    pub line: usize,
    pub col: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixReport {
    pub schema_version: u32,
    pub file: String,
    pub language: String,
    pub dry_run: bool,
    pub applied: Vec<AppliedFix>,
    pub skipped: Vec<SkippedFix>,
    /// True when at least one fix was applied (in dry-run: would be applied).
    pub changed: bool,
}

impl FixReport {
    pub fn text_summary(&self) -> String {
        if !self.changed {
            if self.skipped.is_empty() {
                return "No fixes needed".to_string();
            }
            return format!("No fixes applied ({} skipped)", self.skipped.len());
        }

        let deletes = self
            .applied
            .iter()
            .filter(|f| f.action == FixActionKind::Delete)
            .count();
        let inserts = self
            .applied
            .iter()
            .filter(|f| f.action == FixActionKind::Insert)
            .count();

        let verb = if self.dry_run {
            "would apply"
        } else {
            "applied"
        };
        let skipped_note = if self.skipped.is_empty() {
            String::new()
        } else {
            format!(", {} skipped (ambiguous)", self.skipped.len())
        };

        format!(
            "{} {} fix{} ({} delete, {} insert){}",
            verb,
            self.applied.len(),
            if self.applied.len() == 1 { "" } else { "es" },
            deletes,
            inserts,
            skipped_note
        )
    }
}

/// Paginate a problem list by offset and max.
///
/// Returns `(page, total, truncated, next_cursor)`.
/// - `total` is always the full count of the input.
/// - `truncated` is true when more problems remain after this page.
/// - `next_cursor` is `Some(offset + returned)` when truncated, else `None`.
pub fn paginate(
    problems: Vec<DelimProblem>,
    offset: usize,
    max: usize,
) -> (Vec<DelimProblem>, usize, bool, Option<String>) {
    let total = problems.len();

    let page: Vec<DelimProblem> = if offset >= total {
        Vec::new()
    } else {
        let end = (offset + max).min(total);
        problems[offset..end].to_vec()
    };

    let returned = page.len();
    let truncated = offset + returned < total;
    let next_cursor = if truncated {
        Some((offset + returned).to_string())
    } else {
        None
    };

    (page, total, truncated, next_cursor)
}

pub fn generate_snippet(
    lines: &[&str],
    problem_line: usize,
    problem_col: usize,
    context_lines: usize,
    kind: ProblemKind,
    expected: Option<char>,
) -> String {
    if lines.is_empty() {
        return String::new();
    }
    // When the file has fewer lines than context_lines would suggest
    // (e.g. a single-line file), start/end are clamped to [1, lines.len()],
    // so fewer than 2*context_lines+1 lines are shown. This is intentional —
    // we only show lines that exist.
    let start = problem_line.saturating_sub(context_lines).max(1);
    let end = (problem_line + context_lines).min(lines.len());

    let width = format!("{}", end).len().max(3);
    let mut result = Vec::new();

    for ln in start..=end {
        let idx = ln - 1;
        let content = lines.get(idx).copied().unwrap_or("");
        result.push(format!("{:>w$} | {}", ln, content, w = width));

        if ln == problem_line {
            let col_indent = " ".repeat(problem_col.saturating_sub(1));
            let desc = match (kind, expected) {
                (ProblemKind::MissingClose, Some(c)) => format!("missing {}", c),
                (ProblemKind::UnexpectedClose, Some(c)) => format!("unexpected {}", c),
                _ => String::new(),
            };
            result.push(format!("{:>w$} | {}^ {}", "", col_indent, desc, w = width));
        }
    }

    result.join("\n")
}
