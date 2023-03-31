use yew_router::prelude::*;
use yew::prelude::*;

use crate::app::App;

#[derive(Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/friends")]
    Friends
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub route: Route
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
