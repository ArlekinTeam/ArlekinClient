use std::sync::{Arc, Mutex};

use arc_cell::ArcCell;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    account::{friends_views::friends::Friends, login::Login},
    app_status_bar::AppStatusBar,
    channel_views::channel::Channel,
    common::UnsafeSync,
    direct_messages_views::direct_messages::DirectMessages,
    localization,
    route::{self, Route},
};

lazy_static! {
    static ref INSTANCE: ArcCell<Option<Mutex<UnsafeSync<Callback<Msg>>>>> = ArcCell::default();
    static ref USER_ID: ArcCell<i64> = ArcCell::default();
}

pub struct App {
    logged_in: bool,
    openned_channel: i64,
}

pub enum Msg {
    Login(i64),
    Logout,
    OpennedChannel(i64),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            logged_in: false,
            openned_channel: 0,
        };
        INSTANCE.set(Arc::new(Some(Mutex::new(
            ctx.link().callback(|m| m).into(),
        ))));
        s
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Login(user_id) => {
                USER_ID.set(Arc::new(user_id));
                self.logged_in = true;
            }
            Msg::Logout => self.logged_in = false,
            Msg::OpennedChannel(openned_channel) => self.openned_channel = openned_channel,
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="app-container">
                <AppStatusBar />
                {self.element_view(ctx)}
            </div>
        }
    }
}

impl App {
    pub fn user_id() -> i64 {
        *USER_ID.get()
    }

    pub fn logout() {
        if let Some(instance) = INSTANCE.get().as_ref() {
            instance.lock().unwrap().emit(Msg::Logout);
        }
    }

    fn element_view(&self, ctx: &Context<Self>) -> Html {
        let app_callback = ctx.link().callback(|m| m);
        if !self.logged_in {
            return html! { <Login {app_callback} /> };
        }

        let content = if self.openned_channel == 0 {
            html! { <Friends app_callback={app_callback.clone()} /> }
        } else {
            html! { <Channel app_callback={app_callback.clone()} channel_id={self.openned_channel} /> }
        };

        html! {
            <>
                <link rel="stylesheet" href="/static/css/app.css" />
                <link rel="stylesheet" href="/static/css/channel_views/channel.css" />
                <link rel="stylesheet" href="/static/css/account/friends.css" />

                <div class="app">
                    <div class="app-inner">
                        <div class="app-navigator">
                            <DirectMessages app_callback={app_callback} />
                        </div>
                        <div class="app-content">
                            {content}
                        </div>
                    </div>
                </div>
            </>
        }
    }
}

#[function_component(AppInit)]
pub fn app_init() -> Html {
    let fallback = html! {<div>{"Loading..."}</div>};
    html! {
        <Suspense {fallback}>
            <AppLoader />
        </Suspense>
    }
}

#[function_component(AppLoader)]
pub fn app_loader() -> HtmlResult {
    localization::init_language()?;

    Ok(html! {
        <BrowserRouter>
            <Switch<Route> render={route::switch} />
        </BrowserRouter>
    })
}
