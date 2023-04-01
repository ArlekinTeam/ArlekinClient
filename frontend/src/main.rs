pub mod account;
pub mod api;
pub mod app;
pub mod helpers;
pub mod localization;
pub mod route;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::AppInit>::new().render();
}
