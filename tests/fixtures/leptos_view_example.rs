use leptos::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);

    view! {
        <div class="counter-app">
            <h1>{ "Leptos Counter" }</h1>
            <p>{ move || format!("Count: {}", count.get()) }</p>
            <button on:click=move |_| set_count.set(0)>{ "Reset" }</button>
            <button on:click=move |_| set_count.update(|c| *c += 1)>{ "Increment" }</button>
            <a href="https://example.com/{count}" target="_blank">
                { "Link with braces in attribute" }
            </a>
            // nested view! macro
            {view! {
                <span class="nested">
                    { "nested content" }
                    {move || if count.get() > 10 {
                        view! { <b>{ "High!" }</b> }.into_any()
                    } else {
                        view! { <i>{ "Low" }</i> }.into_any()
                    }}
                </span>
            }}
            // list rendering with .collect_view()
            <ul>
                {move || {
                    (0..count.get())
                        .map(|i| {
                            view! { <li>{ format!("Item {}", i) }</li> }
                        })
                        .collect_view()
                }}
            </ul>
            // escaped braces in string
            <div>{ "{{not interpolated}}" }</div>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(Counter);
}
