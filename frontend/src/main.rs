pub mod account;
pub mod api;
pub mod app;
pub mod channel_views;
pub mod direct_messages_views;
pub mod helpers;
pub mod localization;
pub mod route;
pub mod common;

#[macro_use]
extern crate lazy_static;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::AppInit>::new().render();
}
