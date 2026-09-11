use yew::prelude::*;

#[derive(Properties, PartialEq)]
struct CounterProps {
    initial: u32,
}

#[function_component(Counter)]
fn counter(props: &CounterProps) -> Html {
    let state = use_state(|| props.initial);
    let onclick = Callback::from(Callback::from(move |_| {
        state.set(*state + 1);
    }));

    html! {
        <div class="counter-app">
            <h1>{ "Dioxus Counter" }</h1>
            <p>{ format!("Count: {}", *state) }</p>
            <button {onclick}>{ "Increment" }</button>
            <a href="https://example.com/{state}" target="_blank">
                { "Link with braces in attribute" }
            </a>
            // nested html! macro
            {html! {
                <span class="nested">
                    { "nested content" }
                    { if *state > 10 {
                        html! { <b>{ "High!" }</b> }
                    } else {
                        html! { <i>{ "Low" }</i> }
                    }}
                </span>
            }}
            // list rendering
            <ul>
                { for (0..*state).map(|i| html! {
                    <li key={i}>{ format!("Item {}", i) }</li>
                })}
            </ul>
            // escaped braces in string
            <div>{ "{{not interpolated}}" }</div>
        </div>
    }
}

fn main() {
    yew::Renderer::<Counter>::new().render();
}
