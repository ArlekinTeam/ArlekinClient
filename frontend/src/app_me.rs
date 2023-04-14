use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    app::App,
};

#[function_component(AppMe)]
pub fn app_me() -> Html {
    html! {
        <div class="noselect app-me-container">
            <div class="app-me-profile user-profile-container">
                <LoadUser<()>
                    props={()}
                    user_id={App::user_id()}
                    view={Callback::from(process_user_view)}
                    with_status={true}
                    refresh={true}
                />
            </div>
            <div class="app-me-buttons">
                <Icon onclick={Callback::from(|_| App::display_settings(true))} icon_id={IconId::FontAwesomeSolidGear}/>
            </div>
        </div>
    }
}

fn process_user_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="user-profile">
            <img class="user-avatar" src={user.avatar_url.clone()} alt={"avatar"} />
            {user.status.as_ref().unwrap().icon_html()}
            <div class="user-content">
                <p class="app-me-user-name user-name">{user.name.clone()}</p>
                <p class="app-me-user-info user-info">{"@"}{user.username.clone()}</p>
            </div>
        </div>
    }
}
