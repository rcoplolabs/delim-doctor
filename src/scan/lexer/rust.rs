use super::{BaseLexer, Lexer};

pub struct RustLexer<'a> {
    base: BaseLexer<'a>,
}

impl<'a> RustLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            base: BaseLexer::new(src),
        }
    }

    fn handle_r_prefix(&mut self) {
        if self.base.peek_char() == Some('"') {
            self.base.consume_char();
            self.base.skip_raw_string(0);
        } else if self.base.peek_char() == Some('#') {
            self.try_raw_string();
        }
    }

    fn handle_b_prefix(&mut self) {
        match self.base.peek_char() {
            Some('"') => {
                self.base.consume_char();
                self.base.skip_string('"');
            }
            Some('\'') => {
                self.base.consume_char();
                self.base.skip_string('\'');
            }
            Some('r') => {
                self.base.consume_char();
                if self.base.peek_char() == Some('"') {
                    self.base.consume_char();
                    self.base.skip_raw_string(0);
                } else if self.base.peek_char() == Some('#') {
                    self.try_raw_string();
                }
            }
            _ => {}
        }
    }

    fn try_raw_string(&mut self) -> bool {
        let mut hash_count = 0;
        while self.base.peek_char() == Some('#') {
            self.base.consume_char();
            hash_count += 1;
        }
        if self.base.peek_char() == Some('"') {
            self.base.consume_char();
            self.base.skip_raw_string(hash_count);
            true
        } else {
            false
        }
    }

    fn skip_lifetime_or_char(&mut self) {
        match self.base.peek_char() {
            Some('\\') => {
                self.base.skip_string('\'');
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                self.base.consume_char();
                if self.base.peek_char() == Some('\'') {
                    self.base.consume_char();
                } else {
                    while let Some(c) = self.base.peek_char() {
                        if c.is_alphanumeric() || c == '_' {
                            self.base.consume_char();
                        } else {
                            break;
                        }
                    }
                }
            }
            _ => {
                self.base.skip_string('\'');
            }
        }
    }

    fn skip_nested_block_comment(&mut self) {
        let mut depth = 1u32;
        let mut prev = '\0';
        while let Some((_, c)) = self.base.consume_char() {
            match (prev, c) {
                ('/', '*') => depth += 1,
                ('*', '/') => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
            prev = c;
        }
    }

    fn skip_attribute(&mut self) {
        let mut depth = 1u32;
        while let Some((_, c)) = self.base.consume_char() {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                '"' => self.base.skip_string('"'),
                '\'' => self.skip_lifetime_or_char(),
                '/' if self.base.peek_char() == Some('*') => {
                    self.base.consume_char();
                    self.skip_nested_block_comment();
                }
                '/' if self.base.peek_char() == Some('/') => {
                    self.base.consume_char();
                    self.base.skip_line_comment();
                }
                // Handle raw/byte string prefixes so `]` inside them is
                // not mistaken for the attribute's closing bracket.
                'r' if matches!(self.base.peek_char(), Some('"') | Some('#')) => {
                    self.handle_r_prefix();
                }
                'b' if matches!(self.base.peek_char(), Some('"') | Some('\'') | Some('r')) => {
                    self.handle_b_prefix();
                }
                _ => {}
            }
        }
    }
}

impl<'a> Lexer<'a> for RustLexer<'a> {
    fn base(&mut self) -> &mut BaseLexer<'a> {
        &mut self.base
    }

    fn skip_block_comment(&mut self) {
        self.skip_nested_block_comment();
    }

    fn handle_single_quote(&mut self) {
        self.skip_lifetime_or_char();
    }

    fn handle_hash(&mut self) {
        match self.base.peek_char() {
            Some('[') => {
                self.base.consume_char();
                self.skip_attribute();
            }
            Some('!') => {
                self.base.consume_char();
                if self.base.peek_char() == Some('[') {
                    self.base.consume_char();
                    self.skip_attribute();
                }
            }
            _ => {}
        }
    }

    fn try_handle_prefix(&mut self, ch: char) -> bool {
        match ch {
            'r' => {
                self.handle_r_prefix();
                true
            }
            'b' => {
                self.handle_b_prefix();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_delims(src: &str) -> Vec<(char, usize, usize)> {
        let mut lexer = RustLexer::new(src);
        let mut result = Vec::new();
        while let Some(ev) = lexer.next_delim() {
            result.push((ev.ch, ev.line, ev.col));
        }
        result
    }

    #[test]
    fn test_raw_string_with_hash() {
        let src = "let s = r#\"{[()]}\"#;";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_raw_string_no_hash() {
        let src = "let s = r\"()\";";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_raw_string_multiple_hashes() {
        let src = "let s = r##\"\"#\"##;";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_raw_string_with_quotes_inside() {
        let src = "let s = r#\"he said \"hello\"\"#;";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_byte_raw_string() {
        let src = "let s = br#\"{}\"#;";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_nested_block_comment() {
        let src = "/* { /* } */ { */ x";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_nested_block_comment_deep() {
        let src = "/* a /* b /* c */ d */ e */ f ()";
        let delims = collect_delims(src);
        assert_eq!(delims, vec![('(', 1, 31), (')', 1, 32)]);
    }

    #[test]
    fn test_lifetime_not_char() {
        let src = "fn foo<'a>(x: &'a str) {}";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 4);
    }

    #[test]
    fn test_char_literal_skipped() {
        let src = "let c = 'a';";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_char_literal_escape() {
        let src = "let c = '\\n';";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_static_lifetime() {
        let src = "let s: &'static str = \"hello\";";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_bom_skipped() {
        let src = "\u{FEFF}fn main() {}";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 4);
        assert_eq!(delims[0], ('(', 1, 8));
        assert_eq!(delims[3], ('}', 1, 12));
    }

    #[test]
    fn test_byte_string() {
        let src = "let b = b\"{}\";";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_byte_literal() {
        let src = "let b = b'(';";
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_attribute_skipped() {
        let src = "#[derive(Debug)]\nstruct Foo {}\n";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2, "only struct braces should be delims");
    }

    #[test]
    fn test_attribute_inner() {
        let src = "#![allow(dead_code)]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "fn main() parens + braces; inner attr skipped"
        );
    }

    #[test]
    fn test_attribute_delims_inside() {
        let src = "#[cfg(feature = \"foo\")]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims,
            vec![('(', 2, 8), (')', 2, 9), ('{', 2, 11), ('}', 2, 12)]
        );
    }

    #[test]
    fn test_attribute_then_code_same_line() {
        let src = "#[derive(Debug)] fn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "same-line code after attribute must still be scanned"
        );
    }

    #[test]
    fn test_attribute_nested_brackets() {
        let src = "#[foo(bar[1])]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 4, "nested attribute brackets skipped");
    }

    #[test]
    fn test_attribute_close_bracket_in_string() {
        // ] inside a string inside an attribute must not terminate the attribute.
        let src = "#[cfg(feature = \"x]\")]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "attribute with ] in string should skip the whole attribute"
        );
    }

    #[test]
    fn test_attribute_brackets_in_doc_string() {
        // Markdown-style [link] inside a doc string must not affect depth.
        let src = "#[doc = \"see [link](url)\"]\nfn foo() {}\n";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 4, "doc string brackets must not leak");
    }

    #[test]
    fn test_attribute_close_bracket_in_block_comment() {
        // ] inside a block comment inside an attribute must not terminate it.
        let src = "#[foo /* ] */]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "attribute with ] in block comment should not produce false positive"
        );
    }

    #[test]
    fn test_attribute_close_bracket_in_char_literal() {
        // ] inside a char literal inside an attribute must not terminate it.
        let src = "#[foo(']')]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "attribute with ] in char literal should not produce false positive"
        );
    }

    #[test]
    fn test_attribute_close_bracket_in_raw_string() {
        // ] inside a raw string inside an attribute must not terminate it.
        let src = "#[doc = r#\"see link]\"#]\nfn foo() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "attribute with ] in raw string should not produce false positive"
        );
    }

    #[test]
    fn test_attribute_line_comment_with_close_bracket() {
        // ] inside a line comment inside an attribute (multi-line attribute).
        let src = "#[foo // ]\nbar()]\nfn main() {}\n";
        let delims = collect_delims(src);
        assert_eq!(
            delims.len(),
            4,
            "attribute with ] in line comment should not produce false positive"
        );
    }
}
