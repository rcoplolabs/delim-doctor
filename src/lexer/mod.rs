pub mod generic;
pub mod rust;

pub struct DelimEvent {
    pub ch: char,
    pub line: usize,
    pub col: usize,
    pub byte_offset: usize,
}

pub trait Lexer {
    fn next_delim(&mut self) -> Option<DelimEvent>;
}

pub fn detect_language(ext: Option<&str>, language: Option<&str>) -> String {
    if let Some(lang) = language {
        return lang.to_string();
    }
    match ext.map(|e| e.to_lowercase()).as_deref() {
        Some("rs") => "rust".to_string(),
        _ => "generic".to_string(),
    }
}
