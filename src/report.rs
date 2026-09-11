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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_problem(line: usize) -> DelimProblem {
        DelimProblem {
            kind: ProblemKind::MissingClose,
            ch: '{',
            line,
            col: 1,
            byte_offset: 0,
            expected: Some('}'),
            snippet: String::new(),
            depth_at: 1,
            at_eof: true,
        }
    }

    fn make_problems(n: usize) -> Vec<DelimProblem> {
        (0..n).map(|i| make_problem(i + 1)).collect()
    }

    // --- paginate tests ---

    #[test]
    fn test_paginate_no_truncation_small() {
        let problems = make_problems(5);
        let (page, total, truncated, cursor) = paginate(problems, 0, 20);
        assert_eq!(page.len(), 5);
        assert_eq!(total, 5);
        assert!(!truncated);
        assert!(cursor.is_none());
    }

    #[test]
    fn test_paginate_no_truncation_exact() {
        let problems = make_problems(20);
        let (page, total, truncated, cursor) = paginate(problems, 0, 20);
        assert_eq!(page.len(), 20);
        assert_eq!(total, 20);
        assert!(!truncated);
        assert!(cursor.is_none());
    }

    #[test]
    fn test_paginate_truncated() {
        let problems = make_problems(30);
        let (page, total, truncated, cursor) = paginate(problems, 0, 20);
        assert_eq!(page.len(), 20);
        assert_eq!(total, 30);
        assert!(truncated);
        assert_eq!(cursor, Some("20".to_string()));
    }

    #[test]
    fn test_paginate_second_page_full() {
        let problems = make_problems(50);
        let (page, total, truncated, cursor) = paginate(problems, 20, 20);
        assert_eq!(page.len(), 20);
        assert_eq!(total, 50);
        assert!(truncated);
        assert_eq!(cursor, Some("40".to_string()));
    }

    #[test]
    fn test_paginate_second_page_partial() {
        let problems = make_problems(30);
        let (page, total, truncated, cursor) = paginate(problems, 20, 20);
        assert_eq!(page.len(), 10);
        assert_eq!(total, 30);
        assert!(!truncated);
        assert!(cursor.is_none());
    }

    #[test]
    fn test_paginate_offset_beyond_total() {
        let problems = make_problems(5);
        let (page, total, truncated, cursor) = paginate(problems, 10, 20);
        assert_eq!(page.len(), 0);
        assert_eq!(total, 5);
        assert!(!truncated);
        assert!(cursor.is_none());
    }

    #[test]
    fn test_paginate_multiple_pages() {
        let problems = make_problems(50);

        // Page 1: offset=0
        let (p1, _, t1, c1) = paginate(problems.clone(), 0, 20);
        assert_eq!(p1.len(), 20);
        assert!(t1);
        assert_eq!(c1, Some("20".to_string()));

        // Page 2: offset=20
        let offset2: usize = c1.unwrap().parse().unwrap();
        let (p2, _, t2, c2) = paginate(problems.clone(), offset2, 20);
        assert_eq!(p2.len(), 20);
        assert!(t2);
        assert_eq!(c2, Some("40".to_string()));

        // Page 3: offset=40
        let offset3: usize = c2.unwrap().parse().unwrap();
        let (p3, _, t3, c3) = paginate(problems, offset3, 20);
        assert_eq!(p3.len(), 10);
        assert!(!t3);
        assert!(c3.is_none());
    }

    #[test]
    fn test_paginate_first_line_correct() {
        let problems = make_problems(30);
        let (page, _, _, _) = paginate(problems, 20, 20);
        // Page 2 should start at line 21
        assert_eq!(page[0].line, 21);
    }

    // --- text_summary tests ---

    #[test]
    fn test_text_summary_no_issues() {
        let report = ScanReport {
            schema_version: 1,
            file: "test.rs".into(),
            language: "rust".into(),
            total: 0,
            truncated: false,
            next_cursor: None,
            problems: vec![],
        };
        assert_eq!(report.text_summary(), "No issues found");
    }

    #[test]
    fn test_text_summary_single_issue() {
        let report = ScanReport {
            schema_version: 1,
            file: "test.rs".into(),
            language: "rust".into(),
            total: 1,
            truncated: false,
            next_cursor: None,
            problems: vec![make_problem(1)],
        };
        assert_eq!(
            report.text_summary(),
            "1 issue found (1 MissingClose, 0 UnexpectedClose)"
        );
    }

    #[test]
    fn test_text_summary_truncated() {
        let report = ScanReport {
            schema_version: 1,
            file: "test.rs".into(),
            language: "rust".into(),
            total: 30,
            truncated: true,
            next_cursor: Some("20".into()),
            problems: make_problems(20),
        };
        let summary = report.text_summary();
        assert!(summary.contains("30 issues found"));
        assert!(summary.contains("[showing 20 of 30]"));
    }

    #[test]
    fn test_text_summary_paginated_last_page() {
        let report = ScanReport {
            schema_version: 1,
            file: "test.rs".into(),
            language: "rust".into(),
            total: 30,
            truncated: false,
            next_cursor: None,
            problems: make_problems(10),
        };
        let summary = report.text_summary();
        assert!(summary.contains("30 issues found"));
        assert!(summary.contains("[page: 10 of 30 total]"));
    }

    #[test]
    fn test_text_summary_all_shown_no_note() {
        let report = ScanReport {
            schema_version: 1,
            file: "test.rs".into(),
            language: "rust".into(),
            total: 5,
            truncated: false,
            next_cursor: None,
            problems: make_problems(5),
        };
        let summary = report.text_summary();
        assert!(summary.contains("5 issues found"));
        assert!(!summary.contains("["));
    }
}
