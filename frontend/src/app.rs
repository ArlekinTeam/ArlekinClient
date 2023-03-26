use yew::prelude::*;
use yew_router::prelude::*;

use crate::{account::login::Login, localization, route::{Route, self}};

pub struct App {
    logged_in: bool
}

pub enum Msg {
    Login,
    Logout
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
            },
            Msg::Logout => {
                self.logged_in = false;
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.logged_in {
            html! {
                <>
                    <route::Router route={Route::Home} />
                    <h1>{"Logged in"}</h1>
                </>
            }
        } else {
            let app_callback = ctx.link().callback(|m| m);
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
