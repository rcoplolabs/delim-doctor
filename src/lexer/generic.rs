use std::iter::Peekable;
use std::str::CharIndices;

use super::{DelimEvent, Lexer};

pub struct GenericLexer<'a> {
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    col: usize,
}

impl<'a> GenericLexer<'a> {
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
            '\r' => {}
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

    fn skip_block_comment(&mut self) {
        let mut prev = '\0';
        while let Some((_, c)) = self.consume_char() {
            if prev == '*' && c == '/' {
                break;
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
}

impl<'a> Lexer for GenericLexer<'a> {
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
                    self.skip_block_comment();
                }
                '#' => {
                    self.skip_line_comment();
                }
                '"' => {
                    self.skip_string('"');
                }
                '\'' => {
                    self.skip_string('\'');
                }
                '\r' | '\n' => {}
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
