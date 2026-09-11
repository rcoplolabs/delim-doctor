use std::iter::Peekable;
use std::str::CharIndices;

use super::{DelimEvent, Lexer};

pub struct RustLexer<'a> {
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    col: usize,
}

impl<'a> RustLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.char_indices().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn consume_char(&mut self) -> Option<(usize, char)> {
        let item = self.chars.next()?;
        match item.1 {
            '\n' => {
                self.line += 1;
                self.col = 1;
            }
            '\r' | '\u{FEFF}' => {}
            _ => self.col += 1,
        }
        Some(item)
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.consume_char();
        }
    }

    fn skip_nested_block_comment(&mut self) {
        let mut depth = 1u32;
        let mut prev = '\0';
        while let Some((_, c)) = self.consume_char() {
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

    fn skip_string(&mut self, quote: char) {
        let mut escaped = false;
        while let Some((_, c)) = self.consume_char() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == quote {
                break;
            }
        }
    }

    fn skip_raw_string(&mut self, hash_count: usize) {
        while let Some((_, c)) = self.consume_char() {
            if c == '"' {
                let mut found = 0;
                while found < hash_count && self.peek_char() == Some('#') {
                    self.consume_char();
                    found += 1;
                }
                if found == hash_count {
                    return;
                }
            }
        }
    }

    fn skip_lifetime_or_char(&mut self) {
        match self.peek_char() {
            Some('\\') => {
                self.skip_string('\'');
            }
            Some(c) if c.is_alphabetic() || c == '_' => {
                self.consume_char();
                if self.peek_char() == Some('\'') {
                    self.consume_char();
                } else {
                    while let Some(c) = self.peek_char() {
                        if c.is_alphanumeric() || c == '_' {
                            self.consume_char();
                        } else {
                            break;
                        }
                    }
                }
            }
            _ => {
                self.skip_string('\'');
            }
        }
    }

    fn try_raw_string(&mut self) -> bool {
        let mut hash_count = 0;
        while self.peek_char() == Some('#') {
            self.consume_char();
            hash_count += 1;
        }
        if self.peek_char() == Some('"') {
            self.consume_char();
            self.skip_raw_string(hash_count);
            true
        } else {
            false
        }
    }
}

impl<'a> Lexer for RustLexer<'a> {
    fn next_delim(&mut self) -> Option<DelimEvent> {
        while let Some(&(byte_offset, ch)) = self.chars.peek() {
            let line = self.line;
            let col = self.col;

            self.consume_char();

            match ch {
                '/' if self.peek_char() == Some('/') => {
                    self.consume_char();
                    self.skip_line_comment();
                }
                '/' if self.peek_char() == Some('*') => {
                    self.consume_char();
                    self.skip_nested_block_comment();
                }
                'r' if self.peek_char() == Some('"') => {
                    self.consume_char();
                    self.skip_raw_string(0);
                }
                'r' if self.peek_char() == Some('#') => {
                    self.try_raw_string();
                }
                'b' if self.peek_char() == Some('"') => {
                    self.consume_char();
                    self.skip_string('"');
                }
                'b' if self.peek_char() == Some('\'') => {
                    self.consume_char();
                    self.skip_string('\'');
                }
                'b' if self.peek_char() == Some('r') => {
                    self.consume_char();
                    if self.peek_char() == Some('"') {
                        self.consume_char();
                        self.skip_raw_string(0);
                    } else if self.peek_char() == Some('#') {
                        self.try_raw_string();
                    }
                }
                '"' => {
                    self.skip_string('"');
                }
                '\'' => {
                    self.skip_lifetime_or_char();
                }
                '\r' | '\n' | '\u{FEFF}' => {}
                '(' | '[' | '{' | ')' | ']' | '}' => {
                    return Some(DelimEvent {
                        ch,
                        line,
                        col,
                        byte_offset,
                    });
                }
                _ => {}
            }
        }
        None
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
}
