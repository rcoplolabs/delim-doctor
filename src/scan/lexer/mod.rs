pub mod generic;
pub mod javascript;
pub mod python;
pub mod rust;

use generic::GenericLexer;
use javascript::JavaScriptLexer;
use python::PythonLexer;
use rust::RustLexer;
use std::iter::Peekable;
use std::str::CharIndices;

pub struct DelimEvent {
    pub ch: char,
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}

pub struct BaseLexer<'a> {
    pub chars: Peekable<CharIndices<'a>>,
    pub line: usize,
    pub col: usize,
}

impl<'a> BaseLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.char_indices().peekable(),
            line: 1,
            col: 1,
        }
    }

    pub fn consume_char(&mut self) -> Option<(usize, char)> {
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

    pub fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    pub fn skip_line_comment(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.consume_char();
        }
    }

    pub fn skip_block_comment(&mut self) {
        let mut prev = '\0';
        while let Some((_, c)) = self.consume_char() {
            if prev == '*' && c == '/' {
                break;
            }
            prev = c;
        }
    }

    pub fn skip_string(&mut self, quote: char) {
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

    pub fn skip_raw_string(&mut self, hash_count: usize) {
        while let Some((_, c)) = self.consume_char() {
            if c == '"' && hash_count > 0 {
                let mut matched = true;
                for _ in 0..hash_count {
                    if self.peek_char() != Some('#') {
                        matched = false;
                        break;
                    }
                    self.consume_char();
                }
                if matched {
                    return;
                }
            } else if c == '"' && hash_count == 0 {
                return;
            }
        }
    }
}

pub trait Lexer<'a> {
    fn base(&mut self) -> &mut BaseLexer<'a>;

    fn skip_block_comment(&mut self) {
        self.base().skip_block_comment();
    }

    fn handle_single_quote(&mut self) {
        self.base().skip_string('\'');
    }

    fn handle_double_quote(&mut self) {
        self.base().skip_string('"');
    }

    fn handle_backtick(&mut self) {
        self.base().skip_string('`');
    }

    fn handle_hash(&mut self) {
        self.base().skip_line_comment();
    }

    fn try_handle_prefix(&mut self, _ch: char) -> bool {
        false
    }

    fn next_delim(&mut self) -> Option<DelimEvent> {
        loop {
            let (line, col, byte_offset, ch) = {
                let base = self.base();
                let &(byte_offset, ch) = base.chars.peek()?;
                let line = base.line;
                let col = base.col;
                base.consume_char();
                (line, col, byte_offset, ch)
            };

            match ch {
                '/' if self.base().peek_char() == Some('/') => {
                    self.base().consume_char();
                    self.base().skip_line_comment();
                }
                '/' if self.base().peek_char() == Some('*') => {
                    self.base().consume_char();
                    self.skip_block_comment();
                }
                '#' => self.handle_hash(),
                '"' => self.handle_double_quote(),
                '\'' => self.handle_single_quote(),
                '`' => self.handle_backtick(),
                '\r' | '\n' | '\u{FEFF}' => {}
                '(' | '[' | '{' | ')' | ']' | '}' => {
                    return Some(DelimEvent {
                        ch,
                        line,
                        col,
                        byte_offset,
                    });
                }
                _ if self.try_handle_prefix(ch) => {}
                _ => {}
            }
        }
    }
}

pub fn detect_language(ext: Option<&str>, language: Option<&str>) -> String {
    if let Some(lang) = language {
        return lang.to_string();
    }
    match ext.map(|e| e.to_lowercase()).as_deref() {
        Some("rs") => "rust".to_string(),
        Some("py") | Some("pyw") => "python".to_string(),
        Some("js") | Some("mjs") | Some("cjs") | Some("jsx") => "javascript".to_string(),
        Some("ts") | Some("mts") | Some("cts") | Some("tsx") => "typescript".to_string(),
        _ => "generic".to_string(),
    }
}

pub fn is_valid_language(lang: &str) -> bool {
    matches!(
        lang,
        "rust" | "python" | "javascript" | "js" | "typescript" | "ts" | "generic"
    )
}

pub fn make_lexer<'a>(src: &'a str, language: &str) -> Box<dyn Lexer<'a> + 'a> {
    match language {
        "rust" => Box::new(RustLexer::new(src)),
        "python" => Box::new(PythonLexer::new(src)),
        "javascript" | "js" | "typescript" | "ts" => Box::new(JavaScriptLexer::new(src)),
        _ => Box::new(GenericLexer::new(src)),
    }
}
