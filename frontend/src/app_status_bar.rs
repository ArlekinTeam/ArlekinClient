use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

use arc_cell::ArcCell;
use yew::prelude::*;

use crate::{common::UnsafeSync, localization};

lazy_static! {
    static ref INSTANCE: ArcCell<Option<Mutex<UnsafeSync<Callback<Msg>>>>> =
        ArcCell::default();
    static ref IS_CONNECTED: AtomicBool = AtomicBool::new(true);
}

pub struct AppStatusBar {
}

pub enum Msg {
    UpdateConnection
}

impl Component for AppStatusBar {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self { };
        INSTANCE.set(Arc::new(Some(Mutex::new(ctx.link().callback(|m| m).into()))));
        s
    }

    fn update(&mut self, _: &Context<Self>, _: Self::Message) -> bool {
        true
    }

    fn view(&self, _: &Context<Self>) -> Html {
        if IS_CONNECTED.load(Ordering::Relaxed) {
            return html! {}
        }

        let lang = localization::get_language();
        html! { <div>
            <p>{lang.get("viewAppStatusBarDisconnected")}</p>
        </div> }
    }
}

impl AppStatusBar {
    pub fn set_connection(is_connected: bool) {
        if IS_CONNECTED.swap(is_connected, Ordering::Relaxed) == is_connected {
            return;
        }

        if let Some(instance) = INSTANCE.get().as_ref() {
            instance.lock().unwrap().emit(Msg::UpdateConnection);
        }
    }
}
