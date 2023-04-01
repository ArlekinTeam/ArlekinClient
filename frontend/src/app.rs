use yew::prelude::*;
use yew_router::prelude::*;

use crate::{
    account::{friends_views::friends::Friends, login::Login},
    localization,
    route::{self, Route},
};

pub struct App {
    logged_in: bool,
}

pub enum Msg {
    Login,
    Logout,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self { logged_in: false }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Login => {
                self.logged_in = true;
                true
            }
            Msg::Logout => {
                self.logged_in = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let app_callback = ctx.link().callback(|m| m);
        if self.logged_in {
            html! {
                <>
                    <Friends app_callback={app_callback} />
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
