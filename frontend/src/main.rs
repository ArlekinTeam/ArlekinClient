use account::login::Login;
use yew::prelude::*;

pub mod api;
pub mod account;
pub mod helpers;
pub mod localization;

#[function_component(App)]
fn app() -> Html {
    let fallback = html! {<div>{"Loading..."}</div>};

    html! {
        <Suspense {fallback}>
            <Login />
        </Suspense>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
