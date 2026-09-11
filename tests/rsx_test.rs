#![allow(dead_code)]

#[path = "../src/lexer/mod.rs"]
mod lexer;
#[path = "../src/report.rs"]
mod report;
#[path = "../src/scanner.rs"]
mod scanner;

use std::fs;

fn scan_rust(src: &str) -> Vec<report::DelimProblem> {
    scanner::scan(src, 1, "rust")
}

#[test]
fn rsx_fixture_is_balanced() {
    let src = fs::read_to_string("tests/fixtures/rsx_example.rs").unwrap();
    let problems = scan_rust(&src);
    assert!(
        problems.is_empty(),
        "expected balanced rsx! code, got: {:?}",
        problems
    );
}

#[test]
fn rsx_paren_form_is_balanced() {
    let src = r#"
fn app() -> Element {
    cx.render(rsx!(
        div {
            h1 { "Title" }
            p { "Content with {interpolation}" }
        }
    ))
}
"#;
    assert!(scan_rust(src).is_empty());
}

#[test]
fn rsx_string_escapes_are_balanced() {
    let src = r#"
fn app() -> Element {
    rsx! {
        div { class: "customAttribute", "{escaped}" }
        div { "literal {{{{braces}}}}" }
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}

#[test]
fn missing_brace_is_detected() {
    let src = fs::read_to_string("tests/fixtures/rsx_example.rs").unwrap();
    let broken = src.replace("fn main() {", "fn main() ");
    let problems = scan_rust(&broken);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem after deleting an opening brace"
    );
}
