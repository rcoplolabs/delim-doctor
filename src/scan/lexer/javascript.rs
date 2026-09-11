use super::{BaseLexer, Lexer};

/// JavaScript/TypeScript lexer.
///
/// Uses generic defaults for comments (`//`, `/* */`), strings (`"..."`, `'...'`),
/// and template literals (`` `...` `` skipped as opaque string).
///
/// Known limitations:
/// - Template literal expressions (`${...}`) are skipped as string content;
///   bracket imbalances inside `${}` are not detected.
/// - Regex literals (`/pattern/`) are not distinguished from division;
///   brackets inside regex are reported as delimiters (false positives).
pub struct JavaScriptLexer<'a> {
    base: BaseLexer<'a>,
}

impl<'a> JavaScriptLexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            base: BaseLexer::new(src),
        }
    }
}

impl<'a> Lexer<'a> for JavaScriptLexer<'a> {
    fn base(&mut self) -> &mut BaseLexer<'a> {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_delims(src: &str) -> Vec<super::super::DelimEvent> {
        let mut lexer = JavaScriptLexer::new(src);
        let mut result = Vec::new();
        while let Some(ev) = lexer.next_delim() {
            result.push(ev);
        }
        result
    }

    #[test]
    fn test_template_literal_no_delims() {
        let src = r#"let x = `hello ${name} world`;"#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_template_literal_with_quotes() {
        let src = r#"let x = `he said "hello"`;"#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_template_literal_with_escapes() {
        let src = r#"let x = `hello \`world\``;"#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_nested_template() {
        let src = r#"let x = `outer ${`inner`}`;"#;
        assert!(collect_delims(src).is_empty());
    }

    #[test]
    fn test_template_does_not_leak_quotes() {
        let src = r#"let x = `a " b`; foo();"#;
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
        assert_eq!(delims[0].ch, '(');
        assert_eq!(delims[1].ch, ')');
    }

    #[test]
    fn test_block_comment() {
        let src = "/* ( [ { ) ] } */ x()";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
    }

    #[test]
    fn test_line_comment() {
        let src = "// ( [ { ) ] } \nx()";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
    }

    #[test]
    fn test_regex_literal_false_positives() {
        let src = r#"var re = /[a-z(]/;"#;
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 3);
        assert_eq!(delims[0].ch, '[');
        assert_eq!(delims[1].ch, '(');
        assert_eq!(delims[2].ch, ']');
    }
}
