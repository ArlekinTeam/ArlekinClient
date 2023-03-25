use yew::prelude::*;

use crate::localization;

#[function_component(Login)]
pub fn login() -> HtmlResult {
    let lang = localization::get_language()?;

    Ok(html! {
        <>
            <h1>{lang.get("viewAccountLoginTitle")}</h1>
            <label for="email">{lang.get("viewAccountLoginEmail")}</label>
            <br/>
            <input name="email" type="email" />
            <br/>
            <label for="password">{lang.get("viewAccountLoginPassword")}</label>
            <br/>
            <input name="password" type="password" />
            <br/><br/>
            <button>{lang.get("viewAccountLoginSubmit")}</button>
        </>
    })
}
