use std::collections::HashSet;

use super::report::{AppliedFix, DelimProblem, FixActionKind, ProblemKind, SkippedFix};

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
/// ASCII closing delimiters (`)`, `]`, `}`); EOF inserts are appended in plan
/// order (innermost first).
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
