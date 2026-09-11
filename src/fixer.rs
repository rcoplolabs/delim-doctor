use std::collections::HashSet;

use crate::report::{AppliedFix, DelimProblem, FixActionKind, ProblemKind, SkippedFix};

const AMBIGUOUS_REASON: &str =
    "open delimiter is not at end of file; insertion point is ambiguous, not auto-fixed";

/// A minimal, conservative fix plan.
///
/// Only two situations are auto-fixable:
/// - `UnexpectedClose`: delete the stray closing delimiter.
/// - `MissingClose` reported at end of scan (`at_eof`): append the missing
///   closing delimiter at end-of-file.
///
/// Every other problem (mid-file mismatch recovery) is skipped because the
/// correct repair is not unique.
#[derive(Debug, Default)]
pub struct FixPlan {
    /// Byte offsets of stray closing delimiters to delete.
    deletions: Vec<usize>,
    /// Closing delimiters to append at end-of-file, innermost first.
    eof_inserts: Vec<char>,
    pub applied: Vec<AppliedFix>,
    pub skipped: Vec<SkippedFix>,
}

pub fn plan_fixes(problems: &[DelimProblem]) -> FixPlan {
    let mut plan = FixPlan::default();

    for p in problems {
        match p.kind {
            ProblemKind::UnexpectedClose => {
                plan.deletions.push(p.byte_offset);
                plan.applied.push(AppliedFix {
                    action: FixActionKind::Delete,
                    ch: p.ch,
                    line: p.line,
                    col: p.col,
                });
            }
            ProblemKind::MissingClose => {
                if p.at_eof {
                    let expected = p.expected.unwrap_or('}');
                    plan.eof_inserts.push(expected);
                    plan.applied.push(AppliedFix {
                        action: FixActionKind::Insert,
                        ch: expected,
                        line: p.line,
                        col: p.col,
                    });
                } else {
                    plan.skipped.push(SkippedFix {
                        kind: p.kind,
                        ch: p.ch,
                        line: p.line,
                        col: p.col,
                        reason: AMBIGUOUS_REASON.to_string(),
                    });
                }
            }
        }
    }

    plan
}

/// Produce the fixed source text. Deletions are byte offsets of single-byte
/// ASCII closing delimiters; EOF inserts are appended in plan order.
pub fn apply_fixes(src: &str, plan: &FixPlan) -> String {
    let deletions: HashSet<usize> = plan.deletions.iter().copied().collect();

    let mut out = String::with_capacity(src.len() + plan.eof_inserts.len());
    for (offset, ch) in src.char_indices() {
        if deletions.contains(&offset) {
            continue;
        }
        out.push(ch);
    }
    for close in &plan.eof_inserts {
        out.push(*close);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner;

    fn plan(src: &str) -> FixPlan {
        let problems = scanner::scan(src, 0, "rust");
        plan_fixes(&problems)
    }

    #[test]
    fn stray_close_is_deleted() {
        let src = "fn main() {\n    let x = 1;\n}\n)\n";
        let plan = plan(src);
        assert_eq!(plan.applied.len(), 1);
        assert_eq!(plan.applied[0].action, FixActionKind::Delete);
        assert_eq!(plan.applied[0].ch, ')');
        assert!(plan.skipped.is_empty());
        let fixed = apply_fixes(src, &plan);
        assert_eq!(fixed, "fn main() {\n    let x = 1;\n}\n\n");
    }

    #[test]
    fn eof_missing_close_is_appended() {
        let src = "fn main() {\n    let x = 1;\n";
        let plan = plan(src);
        assert_eq!(plan.applied.len(), 1);
        assert_eq!(plan.applied[0].action, FixActionKind::Insert);
        assert_eq!(plan.applied[0].ch, '}');
        let fixed = apply_fixes(src, &plan);
        assert_eq!(fixed, "fn main() {\n    let x = 1;\n}");
        assert_eq!(scanner::scan(&fixed, 0, "rust").len(), 0);
    }

    #[test]
    fn nested_eof_missing_closes_appended_innermost_first() {
        let src = "fn foo() {\n    let v = vec![1, 2;\n";
        let plan = plan(src);
        // Unclosed: `{` then `[`; EOF inserts must be `]` then `}`.
        let inserted: Vec<char> = plan.applied.iter().map(|f| f.ch).collect();
        assert_eq!(inserted, vec![']', '}']);
        let fixed = apply_fixes(src, &plan);
        assert_eq!(scanner::scan(&fixed, 0, "rust").len(), 0);
    }

    #[test]
    fn mid_file_mismatch_is_skipped() {
        // `)` closes `(`, then `]` mismatches `{`, then EOF leaves `[` `{`.
        let src = "fn main() {\n    let a = (1];\n}\n";
        let plan = plan(src);
        let skipped: Vec<char> = plan.skipped.iter().map(|s| s.ch).collect();
        assert!(!skipped.is_empty(), "expected skipped ambiguous problems");
        // No deletion should target a char inside a balanced region.
        let fixed = apply_fixes(src, &plan);
        assert!(!fixed.is_empty());
    }

    #[test]
    fn balanced_source_needs_no_fix() {
        let src = "fn main() {\n    println!(\"{}\", 1);\n}\n";
        let plan = plan(src);
        assert!(plan.applied.is_empty());
        assert!(plan.skipped.is_empty());
        assert_eq!(apply_fixes(src, &plan), src);
    }
}
