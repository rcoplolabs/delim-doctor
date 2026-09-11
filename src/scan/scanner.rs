use super::lexer::{Lexer, make_lexer};
use super::report::{DelimProblem, ProblemKind, generate_snippet};

struct OpenDelim {
    ch: char,
    line: usize,
    col: usize,
    byte_offset: usize,
    depth_at: usize,
}

fn matching_close(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

/// Map a delimiter char to an index 0–2 for the per-type counter.
fn delim_index(ch: char) -> Option<usize> {
    match ch {
        '(' | ')' => Some(0),
        '[' | ']' => Some(1),
        '{' | '}' => Some(2),
        _ => None,
    }
}

fn scan_inner<'a>(
    lexer: &mut dyn Lexer<'a>,
    lines: &[&str],
    context_lines: usize,
) -> Vec<DelimProblem> {
    let mut stack: Vec<OpenDelim> = Vec::new();
    let mut problems: Vec<DelimProblem> = Vec::new();
    let mut counts: [usize; 3] = [0, 0, 0];

    while let Some(ev) = lexer.next_delim() {
        if matches!(ev.ch, '(' | '[' | '{') {
            let depth_at = stack.len() + 1;
            if let Some(i) = delim_index(ev.ch) {
                counts[i] += 1;
            }
            stack.push(OpenDelim {
                ch: ev.ch,
                line: ev.line,
                col: ev.col,
                byte_offset: ev.byte_offset,
                depth_at,
            });
        } else {
            process_close(
                ev.ch,
                ev.line,
                ev.col,
                ev.byte_offset,
                &mut stack,
                &mut problems,
                &mut counts,
            );
        }
    }

    for open in stack.iter().rev() {
        problems.push(DelimProblem {
            kind: ProblemKind::MissingClose,
            ch: open.ch,
            line: open.line,
            col: open.col,
            byte_offset: open.byte_offset,
            expected: matching_close(open.ch),
            snippet: String::new(),
            depth_at: open.depth_at,
            at_eof: true,
        });
    }

    for p in &mut problems {
        p.snippet = generate_snippet(lines, p.line, p.col, context_lines, p.kind, p.expected);
    }

    problems
}

pub fn scan(src: &str, context_lines: usize, language: &str) -> Vec<DelimProblem> {
    let lines: Vec<&str> = src.lines().collect();
    let mut lexer = make_lexer(src, language);
    scan_inner(&mut *lexer, &lines, context_lines)
}

fn process_close(
    close_ch: char,
    line: usize,
    col: usize,
    byte_offset: usize,
    stack: &mut Vec<OpenDelim>,
    problems: &mut Vec<DelimProblem>,
    counts: &mut [usize; 3],
) {
    if stack.is_empty() {
        problems.push(DelimProblem {
            kind: ProblemKind::UnexpectedClose,
            ch: close_ch,
            line,
            col,
            byte_offset,
            expected: Some(close_ch),
            snippet: String::new(),
            depth_at: 0,
            at_eof: false,
        });
        return;
    }

    if matching_close(stack.last().unwrap().ch) == Some(close_ch) {
        let open = stack.pop().unwrap();
        if let Some(i) = delim_index(open.ch) {
            counts[i] -= 1;
        }
        return;
    }

    // O(1) fast path: if no matching open type exists anywhere on the stack,
    // report unexpected close without scanning the entire stack.
    let ci = delim_index(close_ch).unwrap();
    if counts[ci] == 0 {
        problems.push(DelimProblem {
            kind: ProblemKind::UnexpectedClose,
            ch: close_ch,
            line,
            col,
            byte_offset,
            expected: Some(close_ch),
            snippet: String::new(),
            depth_at: stack.len(),
            at_eof: false,
        });
        return;
    }

    let match_idx = stack
        .iter()
        .rposition(|o| matching_close(o.ch) == Some(close_ch));

    match match_idx {
        Some(idx) => {
            while stack.len() > idx + 1 {
                let open = stack.pop().unwrap();
                if let Some(i) = delim_index(open.ch) {
                    counts[i] -= 1;
                }
                problems.push(DelimProblem {
                    kind: ProblemKind::MissingClose,
                    ch: open.ch,
                    line: open.line,
                    col: open.col,
                    byte_offset: open.byte_offset,
                    expected: matching_close(open.ch),
                    snippet: String::new(),
                    depth_at: open.depth_at,
                    at_eof: false,
                });
            }
            let open = stack.pop().unwrap();
            if let Some(i) = delim_index(open.ch) {
                counts[i] -= 1;
            }
        }
        None => {
            problems.push(DelimProblem {
                kind: ProblemKind::UnexpectedClose,
                ch: close_ch,
                line,
                col,
                byte_offset,
                expected: Some(close_ch),
                snippet: String::new(),
                depth_at: stack.len(),
                at_eof: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::report::ProblemKind;
    use super::*;

    fn scan_src(src: &str) -> Vec<DelimProblem> {
        scan(src, 0, "generic")
    }

    fn scan_rust(src: &str) -> Vec<DelimProblem> {
        scan(src, 0, "rust")
    }

    #[test]
    fn test_balanced_no_problems() {
        let src = "fn main() {\n    let x = [1, 2, 3];\n}\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_missing_close_brace() {
        let src = "fn foo() {\n    let x = 1;\n";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ProblemKind::MissingClose);
        assert_eq!(problems[0].ch, '{');
        assert_eq!(problems[0].line, 1);
        assert_eq!(problems[0].col, 10);
        assert_eq!(problems[0].expected, Some('}'));
        assert_eq!(problems[0].depth_at, 1);
    }

    #[test]
    fn test_unexpected_close_empty_stack() {
        let src = "extra )\n";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ProblemKind::UnexpectedClose);
        assert_eq!(problems[0].ch, ')');
        assert_eq!(problems[0].line, 1);
        assert_eq!(problems[0].col, 7);
        assert_eq!(problems[0].depth_at, 0);
    }

    #[test]
    fn test_mismatch_crossed_opens() {
        let src = "{ [ }";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ProblemKind::MissingClose);
        assert_eq!(problems[0].ch, '[');
        assert_eq!(problems[0].expected, Some(']'));
    }

    #[test]
    fn test_mismatch_no_matching_open() {
        let src = "( ]";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 2);
        assert_eq!(problems[0].kind, ProblemKind::UnexpectedClose);
        assert_eq!(problems[0].ch, ']');
        assert_eq!(problems[0].depth_at, 1);
        assert_eq!(problems[1].kind, ProblemKind::MissingClose);
        assert_eq!(problems[1].ch, '(');
    }

    #[test]
    fn test_nested_depth_at() {
        let src = "{ [ (";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 3);
        assert_eq!(problems[0].ch, '(');
        assert_eq!(problems[0].depth_at, 3);
        assert_eq!(problems[1].ch, '[');
        assert_eq!(problems[1].depth_at, 2);
        assert_eq!(problems[2].ch, '{');
        assert_eq!(problems[2].depth_at, 1);
    }

    #[test]
    fn test_multiple_errors() {
        let src = "{ [ } )";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 2);
        assert_eq!(problems[0].kind, ProblemKind::MissingClose);
        assert_eq!(problems[0].ch, '[');
        assert_eq!(problems[1].kind, ProblemKind::UnexpectedClose);
        assert_eq!(problems[1].ch, ')');
    }

    #[test]
    fn test_brackets_in_string_skipped() {
        let src = "let s = \"(not a delim)\";\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_brackets_in_line_comment_skipped() {
        let src = "// { not a delim\nfn main() {}\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_brackets_in_block_comment_skipped() {
        let src = "/* { [ ( */\nfn main() {}\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_brackets_in_hash_comment_skipped() {
        let src = "# { not a delim }\nx = 1\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_single_quote_string_skipped() {
        let src = "let c = '(';\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_escaped_quote_in_string() {
        let src = "let s = \"it's \\\"fine\\\" {}\";\n";
        let problems = scan_src(src);
        assert!(
            problems.is_empty(),
            "expected no problems, got {:?}",
            problems
        );
    }

    #[test]
    fn test_utf8_column_tracking() {
        let src = "\u{00e5}{";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].ch, '{');
        assert_eq!(problems[0].line, 1);
        assert_eq!(problems[0].col, 2);
    }

    #[test]
    fn test_multiline_string() {
        let src = "let s = \"hello\nworld\";\n{";
        let problems = scan_src(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].ch, '{');
        assert_eq!(problems[0].line, 3);
    }

    #[test]
    fn test_snippet_generation() {
        let src = "line1\nline2\nfn foo() {\n    let x = 1;\nline5\n";
        let problems = scan(src, 1, "generic");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].snippet.contains("fn foo()"));
        assert!(problems[0].snippet.contains("^ missing }"));
    }

    #[test]
    fn test_pass_criteria_missing_brace() {
        let src = "fn outer() {\n    fn inner() {\n        let x = 1;\n    // missing close for inner\n}\n";
        let problems = scan_src(src);
        assert!(!problems.is_empty());
        let outer = problems.iter().find(|p| p.line == 1);
        assert!(
            outer.is_some(),
            "should find MissingClose at line 1 (outer's {{)"
        );
        assert_eq!(outer.unwrap().ch, '{');
        assert_eq!(outer.unwrap().expected, Some('}'));
    }

    #[test]
    fn test_rust_raw_string_brackets_skipped() {
        let src = "let s = r#\"{[()]}\"#;\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_raw_string_no_hash() {
        let src = "let s = r\"()\";\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_nested_block_comment() {
        let src = "/* { /* } */ { */ fn main() {}\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_lifetime_not_char() {
        let src = "fn foo<'a>(x: &'a str) {}\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_static_lifetime() {
        let src = "let s: &'static str = \"hello\";\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_char_literal_skipped() {
        let src = "let c = '(';\nlet d = '\\n';\nlet e = 'a';\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_bom_skipped() {
        let src = "\u{FEFF}fn main() {}\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_byte_string_skipped() {
        let src = "let b = b\"{}\";\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_byte_raw_string() {
        let src = "let s = br#\"{}\"#;\n";
        let problems = scan_rust(src);
        assert!(problems.is_empty(), "got {:?}", problems);
    }

    #[test]
    fn test_rust_missing_brace_with_lifetimes() {
        let src = "fn foo<'a>(x: &'a str) {\n    let y = 1;\n";
        let problems = scan_rust(src);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].kind, ProblemKind::MissingClose);
        assert_eq!(problems[0].ch, '{');
        assert_eq!(problems[0].line, 1);
    }

    #[test]
    fn test_many_unmatched_opens_no_quadratic() {
        // Each line pushes { [ but ) never matches any open on the stack.
        // Before the fix this was O(n²) due to rposition scanning the full
        // stack on every non-matching close.
        let line = "fn foo() { let x = [1, 2, 3); bar(x); \n";
        let src = line.repeat(10_000);
        let problems = scan_src(&src);
        // 1 UnexpectedClose per line + 2 MissingClose per line at EOF.
        assert_eq!(problems.len(), 30_000);
    }
}
