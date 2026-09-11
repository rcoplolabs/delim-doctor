use maud::{html, Markup};

fn render_page(title: &str, items: &[&str]) -> Markup {
    html! {
        div class="page" {
            header {
                h1 { (title) }
                nav {
                    a href="https://example.com/{title}" { "Home" }
                    a href="/about" { "About" }
                }
            }
            main {
                ul {
                    @for (i, item) in items.iter().enumerate() {
                        li { (format!("Item {}: {}", i, item)) }
                    }
                }
                // conditional rendering
                @if items.is_empty() {
                    p { "No items found." }
                } else {
                    p { (items.len()) " items found." }
                }
            }
            footer {
                // escaped braces in string
                p { "{{not interpolated}}" }
            }
        }
    }
}

fn render_card(content: &str) -> Markup {
    html! {
        div class="card" {
            div class="card-body" {
                (content)
            }
            div class="card-footer" {
                small { "Footer" }
            }
        }
    }
}

fn main() {
    let items = vec!["foo", "bar", "baz"];
    let _markup = render_page("Test Page", &items);
}
