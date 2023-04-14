use yew::prelude::*;
use yew_icons::{Icon, IconId};

use crate::{
    app::App,
    localization,
    route::{self, Route},
};

#[function_component(Settings)]
pub fn settings() -> Html {
    let lang = localization::get_language();
    html! { <>
        <route::Router route={Route::Settings} />

        <div class="settings-container">
            <div class="settings-inner">
                <div class="settings-navigator">
                    <button onclick={Callback::from(|_| App::logout())}>{lang.get("viewSettingsLogoutButton")}</button>
                </div>
                <div class="settings-content">

                </div>
                <div class="settings-exit">
                    <Icon onclick={Callback::from(|_| App::display_settings(false))} icon_id={IconId::BootstrapXCircle}/>
                </div>
            </div>
        </div>
    </> }
}
