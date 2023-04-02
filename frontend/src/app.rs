use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    account::{friends_views::friends::Friends, login::Login},
    channel_views::channel::Channel,
    direct_messages_views::direct_messages::DirectMessages,
    localization,
    route::{self, Route},
};

pub struct App {
    logged_in: bool,
    openned_channel: i64,
}

pub enum Msg {
    Login,
    Logout,
    OpennedChannel(i64),
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            logged_in: false,
            openned_channel: 0,
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Login => self.logged_in = true,
            Msg::Logout => self.logged_in = false,
            Msg::OpennedChannel(openned_channel) => self.openned_channel = openned_channel,
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let app_callback = ctx.link().callback(|m| m);
        if self.logged_in {
            let content = if self.openned_channel == 0 {
                html! { <Friends app_callback={app_callback.clone()} /> }
            } else {
                html! { <Channel app_callback={app_callback.clone()} /> }
            };

            html! {
                <>
                    <DirectMessages app_callback={app_callback} />
                    {content}
                </>
            }
        } else {
            html! { <Login {app_callback} /> }
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
