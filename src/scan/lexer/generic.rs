use super::{BaseLexer, Lexer};

pub struct GenericLexer<'a> {
    base: BaseLexer<'a>,
}

impl<'a> GenericLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            base: BaseLexer::new(src),
        }
    }
}

impl<'a> Lexer<'a> for GenericLexer<'a> {
    fn base(&mut self) -> &mut BaseLexer<'a> {
        &mut self.base
    }
}
