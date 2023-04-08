pub mod account;
pub mod api;
pub mod app;
pub mod app_status_bar;
pub mod channel_views;
pub mod common;
pub mod direct_messages_views;
pub mod helpers;
pub mod localization;
pub mod notifier;
pub mod route;

#[macro_use]
extern crate lazy_static;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<app::AppInit>::new().render();
}
