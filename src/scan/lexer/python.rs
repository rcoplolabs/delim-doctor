use super::{BaseLexer, Lexer};

pub struct PythonLexer<'a> {
    base: BaseLexer<'a>,
}

impl<'a> PythonLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            base: BaseLexer::new(src),
        }
    }

    fn skip_triple(&mut self, quote: char) {
        let mut consecutive = 0u32;
        while let Some((_, c)) = self.base.consume_char() {
            if c == '\\' {
                self.base.consume_char();
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
        match self.base.peek_char() {
            Some('\'') => {
                self.base.consume_char();
                if let Some('\'') = self.base.peek_char() {
                    self.base.consume_char();
                    self.skip_triple('\'');
                }
            }
            _ => self.base.skip_string('\''),
        }
    }

    fn handle_double_quote(&mut self) {
        match self.base.peek_char() {
            Some('"') => {
                self.base.consume_char();
                if let Some('"') = self.base.peek_char() {
                    self.base.consume_char();
                    self.skip_triple('"');
                }
            }
            _ => self.base.skip_string('"'),
        }
    }

    fn try_handle_prefix(&mut self, ch: char) -> bool {
        let cl = ch.to_ascii_lowercase();
        if !matches!(cl, 'r' | 'b' | 'f' | 'u') {
            return false;
        }
        match self.base.peek_char() {
            Some(q) if q == '\'' || q == '"' => true,
            Some(n) => {
                let nl = n.to_ascii_lowercase();
                let valid = matches!((cl, nl), ('r', 'b') | ('b', 'r') | ('f', 'r') | ('r', 'f'));
                if valid {
                    self.base.consume_char();
                    matches!(self.base.peek_char(), Some(q) if q == '\'' || q == '"')
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
}
