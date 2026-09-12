use super::{BaseLexer, Lexer};

pub struct PythonLexer<'a> {
    base: BaseLexer<'a>,
    /// Tracks whether the string prefix just consumed was raw (`r`, `rb`,
    /// `br`, `rf`, `fr`). Set by `try_handle_prefix`, consumed by
    /// `handle_single_quote`/`handle_double_quote`.
    last_prefix_raw: bool,
}

impl<'a> PythonLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            base: BaseLexer::new(src),
            last_prefix_raw: false,
        }
    }

    fn skip_triple(&mut self, quote: char, raw: bool) {
        let mut consecutive = 0u32;
        while let Some((_, c)) = self.base.consume_char() {
            if !raw && c == '\\' {
                self.base.consume_char();
                consecutive = 0;
            } else if c == quote {
                consecutive += 1;
                if consecutive == 3 {
                    return;
                }
            } else {
                consecutive = 0;
            }
        }
    }
}

impl<'a> Lexer<'a> for PythonLexer<'a> {
    fn base(&mut self) -> &mut BaseLexer<'a> {
        &mut self.base
    }

    fn handle_single_quote(&mut self) {
        let raw = self.last_prefix_raw;
        self.last_prefix_raw = false;
        match self.base.peek_char() {
            Some('\'') => {
                self.base.consume_char();
                if let Some('\'') = self.base.peek_char() {
                    self.base.consume_char();
                    self.skip_triple('\'', raw);
                }
            }
            _ => {
                if raw {
                    self.base.skip_raw_simple('\'');
                } else {
                    self.base.skip_string('\'');
                }
            }
        }
    }

    fn handle_double_quote(&mut self) {
        let raw = self.last_prefix_raw;
        self.last_prefix_raw = false;
        match self.base.peek_char() {
            Some('"') => {
                self.base.consume_char();
                if let Some('"') = self.base.peek_char() {
                    self.base.consume_char();
                    self.skip_triple('"', raw);
                }
            }
            _ => {
                if raw {
                    self.base.skip_raw_simple('"');
                } else {
                    self.base.skip_string('"');
                }
            }
        }
    }

    fn try_handle_prefix(&mut self, ch: char) -> bool {
        let cl = ch.to_ascii_lowercase();
        if !matches!(cl, 'r' | 'b' | 'f' | 'u') {
            return false;
        }
        let is_raw = cl == 'r';
        match self.base.peek_char() {
            Some(q) if q == '\'' || q == '"' => {
                self.last_prefix_raw = is_raw;
                true
            }
            Some(n) => {
                let nl = n.to_ascii_lowercase();
                let valid = matches!((cl, nl), ('r', 'b') | ('b', 'r') | ('f', 'r') | ('r', 'f'));
                if valid {
                    self.base.consume_char();
                    let raw = is_raw || nl == 'r';
                    match self.base.peek_char() {
                        Some(q) if q == '\'' || q == '"' => {
                            self.last_prefix_raw = raw;
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::DelimEvent;
    use super::*;

    fn collect_delims(src: &str) -> Vec<DelimEvent> {
        let mut lexer = PythonLexer::new(src);
        let mut result = Vec::new();
        while let Some(ev) = lexer.next_delim() {
            result.push(ev);
        }
        result
    }

    #[test]
    fn test_simple_string() {
        let src = "x = \"hello\"";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_single_quote_string() {
        let src = "x = 'hello'";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_double_quote() {
        let src = "x = \"\"\"hello {world}\"\"\"";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_single_quote() {
        let src = "x = '''hello {world}'''";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_quote_with_newlines() {
        let src = "x = \"\"\"line1\nline2\nline3\"\"\"";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_quote_with_escaped_quote() {
        let src = r#"x = """hello \"world\"""""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_raw_string() {
        let src = r#"x = r"hello \n world""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_f_string() {
        let src = r#"x = f"hello {name}""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_bytes_string() {
        let src = r#"x = b"hello""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_prefix_combo() {
        let src = r#"x = rf"hello""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_balanced_parens() {
        let src = "x = [1, 2, 3]";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
        assert_eq!(delims[0].ch, '[');
        assert_eq!(delims[1].ch, ']');
    }

    #[test]
    fn test_comment_with_delims() {
        let src = "# ( [ { ) ] }\nx = 1";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_string_with_delims() {
        let src = r#"x = "( [ { ) ] }""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_empty_string() {
        let src = r#"x = "" y = ''"#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_quote_escaped_double_quote_at_end() {
        let src = r#"x = """hello \" world""""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_quote_escaped_quote_followed_by_more_quotes() {
        let src = r#"x = """"\'"""""#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_triple_quote_escaped_backslash() {
        let src = r#"x = """a \\\nb""""#;
        assert!(collect_delims(src).is_empty());
    }

    // --- Raw triple-quoted string tests ---

    #[test]
    fn test_raw_triple_double_quote() {
        // In raw strings, \ is literal, so \""" should close the string.
        let src = "x = r\"\"\"hello\\\"\"\"";
        assert!(
            collect_delims(src).is_empty(),
            "raw triple string should close at \\\"\"\""
        );
    }

    #[test]
    fn test_raw_triple_single_quote() {
        let src = "x = r'''hello\\'''";
        assert!(
            collect_delims(src).is_empty(),
            "raw triple single should close at \\'''"
        );
    }

    #[test]
    fn test_raw_triple_quote_backslash_literal() {
        // Backslash before quotes in raw mode is literal — string closes at """.
        let src = "x = r\"\"\"a\\nb\\\"\"\"";
        assert!(
            collect_delims(src).is_empty(),
            "got: {:?}",
            collect_delims(src)
        );
    }

    #[test]
    fn test_raw_regular_string_backslash_quote() {
        // In a raw regular string r"...\", the " after \ is the closing quote.
        let src = "r\"hello\\\" world()";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
        assert_eq!(delims[0].ch, '(');
        assert_eq!(delims[1].ch, ')');
    }

    #[test]
    fn test_rb_triple_quote_raw() {
        // rb prefix: raw + bytes, should treat \ as literal in triple.
        let src = "x = rb\"\"\"hello\\\"\"\"";
        assert!(
            collect_delims(src).is_empty(),
            "rb triple should close at \\\"\"\""
        );
    }

    #[test]
    fn test_rf_triple_quote_raw() {
        // rf prefix: raw + f-string, should treat \ as literal in triple.
        let src = "x = rf\"\"\"hello\\\"\"\"";
        assert!(
            collect_delims(src).is_empty(),
            "rf triple should close at \\\"\"\""
        );
    }

    #[test]
    fn test_non_raw_triple_still_handles_escapes() {
        // Non-raw triple: \ before """ should escape the first quote.
        let src = r#"x = """hello\"""world""""#;
        assert!(
            collect_delims(src).is_empty(),
            "non-raw triple should handle escapes"
        );
    }
}
