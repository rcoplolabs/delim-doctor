use super::{BaseLexer, Lexer};

/// JavaScript/TypeScript lexer.
///
/// Uses generic defaults for comments (`//`, `/* */`), strings (`"..."`, `'...'`),
/// and template literals (`` `...` `` skipped as opaque string).
///
/// `#` is NOT treated as a line comment (unlike the generic base) because in
/// modern JavaScript/TypeScript (ES2022+) `#` denotes private class fields
/// (`#field`, `#method()`), not comments. Only `//` and `/* */` are comments.
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

    /// In JS/TS, `#` is a private class field marker (ES2022+), not a line
    /// comment. Do nothing — the `#` is already consumed by `next_delim`'s
    /// `consume_char`, and subsequent characters are scanned normally.
    fn handle_hash(&mut self) {}
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

    #[test]
    fn test_private_field_not_treated_as_comment() {
        // # is a private field marker, not a line comment.
        // Delimiters on the same line must be scanned.
        let src = "class C { #arr = [1, 2]; }";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 4);
        assert_eq!(delims[0].ch, '{');
        assert_eq!(delims[1].ch, '[');
        assert_eq!(delims[2].ch, ']');
        assert_eq!(delims[3].ch, '}');
    }

    #[test]
    fn test_private_field_access_with_parens() {
        let src = "this.#method(arg)";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
        assert_eq!(delims[0].ch, '(');
        assert_eq!(delims[1].ch, ')');
    }

    #[test]
    fn test_private_field_in_class_body() {
        let src = r#"class Counter {
    #items = [1, 2, 3];
    get() {
        return this.#items.length;
    }
}"#;
        let delims = collect_delims(src);
        // { [ ] { ( ) } }
        assert_eq!(delims.len(), 8);
    }

    #[test]
    fn test_hash_not_skipping_rest_of_line() {
        // Previously, # would skip the entire line as a comment.
        // Now, # is a regular character and delimiters after it are found.
        let src = "x = 1; #field()";
        let delims = collect_delims(src);
        assert_eq!(delims.len(), 2);
        assert_eq!(delims[0].ch, '(');
        assert_eq!(delims[1].ch, ')');
    }
}
