use dioxus::prelude::*;

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let country = "es";
    let coordinates = (42, 0);
    let text = "Dioxus";

    rsx! {
        div {
            class: "country-{country}",
            "position": "{coordinates:?}",
            h1 { "Glorious Counter" }
            p { "Count: {count}" }
            button { onclick: move |_| count += 1, "Increment" }
            button { onclick: move |_| count -= 1, "Decrement" }
            // arbitrary expressions are allowed
            div {
                {text.to_uppercase()}
                {(0..10).map(|i| rsx! {
                    div { "{i}" }
                })}
            }
            // {} can be escaped with {{}}
            div {
                "{{}}"
            }
            // for loop
            for i in 0..3 {
                div { "{i}" }
            }
            // if statement
            if true {
                div { "true" }
            }
            // match
            {match count() {
                0 => rsx! { p { "zero" } },
                _ => rsx! { p { "non-zero" } },
            }}
        }
    }
}

fn Clickable(cx: &Scope) -> Element {
    cx.render(rsx!(
        a {
            href: "https://example.com",
            class: "fancy-button",
            &cx.props.children
        }
    ))
}

fn main() {
    dioxus::launch(app);
}
