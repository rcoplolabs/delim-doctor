use sycamore::prelude::*;

#[component]
fn Counter<G: Html>(cx: Scope) -> View<G> {
    let count = create_signal(cx, 0i32);

    view! {
        cx,
        div(class="counter-app") {
            h1 { "Sycamore Counter" }
            p { "Count: " (count.get()) }
            button(on:click=|_| count.set(count.get() + 1)) { "Increment" }
            button(on:click=|_| count.set(0)) { "Reset" }
            a(href="https://example.com/{count.get()}") { "Link" }
            // nested view!
            (view! { cx, span(class="nested") { "nested" } })
            // list rendering
            ul {
                Keyed(
                    iterable=IndexedProp(
                        iterable=|| (0..count.get()).collect::<Vec<_>>(),
                        view=|cx, i| view! { cx, li { "Item " (i) } },
                    )
                )
            }
            // string with braces
            div { "{{not interpolated}}" }
        }
    }
}

fn main() {
    sycamore::render(Counter);
}
