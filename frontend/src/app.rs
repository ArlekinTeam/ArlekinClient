use std::sync::{Arc, Mutex};

use arc_cell::ArcCell;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    account::{friends_views::friends::Friends, login::Login},
    app_status_bar::AppStatusBar,
    channel_views::channel::Channel,
    common::UnsafeSync,
    direct_messages_views::{direct_messages::DirectMessages, encryption},
    localization,
    route::{self, Route}, app_me::AppMe, settings_views::settings::Settings, api::{self, ApiResponse},
    helpers::prelude::WebPage, notifier,
};

lazy_static! {
    static ref INSTANCE: ArcCell<Option<Mutex<UnsafeSync<Callback<Msg>>>>> = ArcCell::default();
    static ref USER_ID: ArcCell<i64> = ArcCell::default();
}

pub struct App {
    logged_in: bool,
    is_settings_displayed: bool,
    openned_channel: i64,
}

pub enum Msg {
    Login(i64),
    Logout,
    DisplaySettings(bool),
    OpennedChannel(i64),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            logged_in: false,
            is_settings_displayed: false,
            openned_channel: 0,
        };
        INSTANCE.set(Arc::new(Some(Mutex::new(
            ctx.link().callback(|m| m).into(),
        ))));

        let user_id = WebPage::local_storage().get_item("user_id").unwrap();
        if let Some(user_id) = user_id {
            let user_id = user_id.parse::<i64>().unwrap();
            if api::try_load() && encryption::try_load() {
                ctx.link().callback(move |_| Msg::Login(user_id)).emit(());
                return s;
            }
        }

        Self::remove_session();
        s
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Login(user_id) => {
                self.logged_in = true;

                USER_ID.set(Arc::new(user_id));
                WebPage::local_storage().set_item("user_id", &user_id.to_string())
                    .expect("Unable to set user_id to session storage.");

                notifier::connect();
            }
            Msg::Logout => {
                self.logged_in = false;
                self.is_settings_displayed = false;
                self.openned_channel = 0;
            },
            Msg::DisplaySettings(display) => self.is_settings_displayed = display,
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

    pub fn display_settings(display: bool) {
        if let Some(instance) = INSTANCE.get().as_ref() {
            instance.lock().unwrap().emit(Msg::DisplaySettings(display));
        }
    }

    pub fn logout() {
        api::get("accounts/auth/logout").send_without_ok(
            move |r: ApiResponse<()>| match r {
                ApiResponse::Ok(_) => (),
                ApiResponse::BadRequest(_) => {
                    log::error!("Failed to logout.");
                }
            },
        );
        Self::logout_without_api();
    }

    pub(crate) fn logout_without_api() {
        Self::remove_session();
        if let Some(instance) = INSTANCE.get().as_ref() {
            instance.lock().unwrap().emit(Msg::Logout);
        }
    }

    fn remove_session() {
        let storage = WebPage::local_storage();
        storage.remove_item("user_id").unwrap();
        storage.remove_item("refresh_token").unwrap();
        storage.remove_item("encryption_block_hash").unwrap();
    }

    fn element_view(&self, ctx: &Context<Self>) -> Html {
        let app_callback = ctx.link().callback(|m| m);
        if !self.logged_in {
            return html! { <Login {app_callback} /> };
        }

        let styles = html! { <>
            <link rel="stylesheet" href="/static/css/app.css" />
            <link rel="stylesheet" href="/static/css/channel_views/channel.css" />
            <link rel="stylesheet" href="/static/css/account/friends.css" />
            <link rel="stylesheet" href="/static/css/settings_views/settings.css" />
        </> };

        if self.is_settings_displayed {
            return html! { <>
                {styles}
                <Settings />
            </> }
        }

        let content = if self.openned_channel == 0 {
            html! { <Friends app_callback={app_callback.clone()} /> }
        } else {
            html! { <Channel app_callback={app_callback.clone()} channel_id={self.openned_channel} /> }
        };

        html! {
            <>
                {styles}

                <div class="app">
                    <div class="app-inner">
                        <div class="app-navigator">
                            <div class="app-navigator-content">
                                <div class="app-navigator-content-inner">
                                    <DirectMessages app_callback={app_callback} />
                                </div>
                            </div>

                            <AppMe />
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

/*fn app_loader_async() -> SuspensionResult<()> {
    let suspension = Suspension::from_future(async {
        localization::get_language_async().await;
    });

    if suspension.resumed() {
        Ok(())
    } else {
        Err(suspension)
    }
}*/
