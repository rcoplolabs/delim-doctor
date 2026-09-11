#![allow(dead_code)]

#[path = "../src/scan/lexer/mod.rs"]
mod lexer;
#[path = "../src/scan/report.rs"]
mod report;
#[path = "../src/scan/scanner.rs"]
mod scanner;

use std::fs;

fn scan_rust(src: &str) -> Vec<report::DelimProblem> {
    scanner::scan(src, 1, "rust")
}

// ---------- Yew html! ----------

#[test]
fn yew_fixture_is_balanced() {
    let src = fs::read_to_string("tests/fixtures/yew_html_example.rs").unwrap();
    let problems = scan_rust(&src);
    assert!(
        problems.is_empty(),
        "expected balanced yew html! code, got: {:?}",
        problems
    );
}

#[test]
fn yew_missing_brace_is_detected() {
    let src = fs::read_to_string("tests/fixtures/yew_html_example.rs").unwrap();
    let broken = src.replace("fn main() {", "fn main() ");
    let problems = scan_rust(&broken);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem after deleting an opening brace"
    );
}

#[test]
fn yew_nested_html_is_balanced() {
    let src = r#"
fn item() -> Html {
    html! {
        <div>
            {html! { <span>{ "nested" }</span> }}
            {if true {
                html! { <b>{ "yes" }</b> }
            } else {
                html! { <i>{ "no" }</i> }
            }}
        </div>
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}

// ---------- Leptos view! ----------

#[test]
fn leptos_fixture_is_balanced() {
    let src = fs::read_to_string("tests/fixtures/leptos_view_example.rs").unwrap();
    let problems = scan_rust(&src);
    assert!(
        problems.is_empty(),
        "expected balanced leptos view! code, got: {:?}",
        problems
    );
}

#[test]
fn leptos_missing_brace_is_detected() {
    let src = fs::read_to_string("tests/fixtures/leptos_view_example.rs").unwrap();
    let broken = src.replace("fn main() {", "fn main() ");
    let problems = scan_rust(&broken);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem after deleting an opening brace"
    );
}

#[test]
fn leptos_closures_in_view_are_balanced() {
    let src = r#"
#[component]
fn Foo() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <div>
            <button on:click=move |_| { set_count.set(count.get() + 1); }>
                { "Click" }
            </button>
            {move || {
                let c = count.get();
                if c > 10 {
                    view! { <b>{ "High" }</b> }.into_any()
                } else {
                    view! { <i>{ "Low" }</i> }.into_any()
                }
            }}
        </div>
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}

// ---------- Maud html! ----------

#[test]
fn maud_fixture_is_balanced() {
    let src = fs::read_to_string("tests/fixtures/maud_html_example.rs").unwrap();
    let problems = scan_rust(&src);
    assert!(
        problems.is_empty(),
        "expected balanced maud html! code, got: {:?}",
        problems
    );
}

#[test]
fn maud_missing_brace_is_detected() {
    let src = fs::read_to_string("tests/fixtures/maud_html_example.rs").unwrap();
    let broken = src.replace("fn main() {", "fn main() ");
    let problems = scan_rust(&broken);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem after deleting an opening brace"
    );
}

#[test]
fn maud_control_directives_are_balanced() {
    let src = r#"
fn render(items: &[&str]) -> Markup {
    html! {
        ul {
            @for item in items {
                li { (item) }
            }
            @if items.is_empty() {
                p { "empty" }
            }
            @match items.len() {
                0 => { p { "zero" } },
                _ => { p { "many" } },
            }
        }
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}

// ---------- Sycamore view! ----------

#[test]
fn sycamore_fixture_is_balanced() {
    let src = fs::read_to_string("tests/fixtures/sycamore_view_example.rs").unwrap();
    let problems = scan_rust(&src);
    assert!(
        problems.is_empty(),
        "expected balanced sycamore view! code, got: {:?}",
        problems
    );
}

#[test]
fn sycamore_missing_brace_is_detected() {
    let src = fs::read_to_string("tests/fixtures/sycamore_view_example.rs").unwrap();
    let broken = src.replace("fn main() {", "fn main() ");
    let problems = scan_rust(&broken);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem after deleting an opening brace"
    );
}

// ---------- Cross-framework edge cases ----------

#[test]
fn raw_string_in_macro_attribute_is_balanced() {
    let src = r##"
fn view() -> Html {
    html! {
        <div class=r#"data-{value}"#>
            { "content" }
        </div>
    }
}
"##;
    assert!(scan_rust(src).is_empty());
}

#[test]
fn char_literal_delims_in_macro_are_balanced() {
    let src = r#"
fn view() -> Html {
    html! {
        <button onclick=move |_| handler('(')>
            { "(" }
        </button>
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}

#[test]
fn nested_macro_mismatch_is_detected() {
    // Missing closing ) for html!()
    let src = r#"
fn view() -> Html {
    html!(
        <div>{ "content" }</div>
    }
}
"#;
    let problems = scan_rust(src);
    assert!(
        !problems.is_empty(),
        "expected a missing-close problem for unclosed html!()"
    );
}

#[test]
fn format_string_braces_are_balanced() {
    let src = r#"
fn view() -> Html {
    let count = 5;
    html! {
        <div>
            { format!("Count: {}", count) }
            { format!("Nested: {{inner}}") }
            { format!("Multi: {} {} {}", count, count, count) }
        </div>
    }
}
"#;
    assert!(scan_rust(src).is_empty());
}
