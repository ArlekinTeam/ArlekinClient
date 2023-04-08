use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::App;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/friends")]
    Friends,
    #[at("/direct/:id")]
    Direct { id: i64 },
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub route: Route,
}

pub fn switch(_: Route) -> Html {
    html! { <App /> }
}

#[function_component(Router)]
pub fn router(props: &Props) -> Html {
    let navigator = use_navigator().unwrap();
    navigator.push(&props.route);

    html! {}
}
