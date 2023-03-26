use argon2::Argon2;
use serde::{Serialize, Deserialize};
use serde_json::json;
use uuid::Uuid;
use yew::prelude::*;

use crate::{localization, helpers::prelude::*, api::{self, ApiResponse}, app, route::{Router, Route}};

pub struct Login {
    props: Props
}

pub enum Msg {
    Submit
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseData {
    refresh_token: Uuid,
    message_encryption_salt: i64
}

impl Component for Login {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self { props: ctx.props().clone() }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Submit => {
                self.submit();
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        html! {
            <>
                <Router route={Route::Login} />

                <h1>{lang.get("viewAccountLoginTitle")}</h1>
                <label for="email">{lang.get("viewAccountLoginEmail")}</label>
                <br/>
                <input name="email" id="email" type="email" />
                <br/>
                <label for="password">{lang.get("viewAccountLoginPassword")}</label>
                <br/>
                <input name="password" id="password" type="password" />
                <br/><br/>
                <button onclick={ctx.link().callback(|_| Msg::Submit)}>{lang.get("viewAccountLoginSubmit")}</button>

                <p id="loginStatus"></p>
            </>
        }
    }
}

impl Login {
    fn submit(&self) {
        let email = Input::by_id("email").value();
        let password = Input::by_id("password").value();

        if email.is_empty() || password.is_empty() {
            return;
        }

        let mut password_hash = [0u8; 32];
        Argon2::default().hash_password_into(
            password.as_bytes(), format!("arlekin{}login", email).as_bytes(), &mut password_hash
        ).unwrap();
        let password_hash_string = password_hash.iter().map(|&x| x as char).collect::<String>();

        let app_callback = self.props.app_callback.clone();

        api::post("accounts/auth/login").body(&json!({
            "email": email,
            "passwordHash": password_hash_string
        })).send(app_callback.clone(), move |r: ApiResponse<LoginResponseData>| match r {
            ApiResponse::Ok(r) => {
                let mut message_encryption_hash = [0u8; 512];
                Argon2::default().hash_password_into(password.as_bytes(), format!(
                    "arlekin{}message",
                    r.message_encryption_salt.to_le_bytes().iter().map(|&x| x as char).collect::<String>()
                ).as_bytes(), &mut message_encryption_hash).unwrap();

                Element::by_id("loginStatus").set_inner_html(
                    &message_encryption_hash.iter().map(|&x| x as char).collect::<String>()
                );

                app_callback.emit(app::Msg::Login);
            },
            ApiResponse::BadRequest(err) => {
                let mut message = String::new();
                for (key, value) in err {
                    message.push_str(&key);
                    message.push_str(": ");
                    message.push_str(&value.translation_key);
                    message.push_str("<br>");
                }

                Element::by_id("loginStatus").set_inner_html(&message);
            },
        });
    }
}
