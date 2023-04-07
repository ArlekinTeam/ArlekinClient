use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use yew::prelude::*;

use crate::{
    api::{self, ApiResponse},
    app,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization, notifier,
    route::{Route, Router},
};

pub struct Login {
    props: Props,
    status: Html,
}

pub enum Msg {
    SetStatus(Html),
    Submit,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseData {
    refresh_token: Uuid,
    message_encryption_salt: i64,
}

impl Component for Login {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            props: ctx.props().clone(),
            status: Default::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetStatus(status) => self.status = status,
            Msg::Submit => {
                self.submit(ctx);
                return false;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        html! {
            <>
                <Router route={Route::Login} />
                <link rel="stylesheet" href="/static/css/account/login.css" />
                <div class="login-container">
                    <div id="login-items-main">
                        <div id="login-items">
                            <h1 id="login-header">{lang.get("viewAccountLoginTitle")}</h1>
                            <h3 id="welcome-text">{lang.get("viewAccountLoginWelcomeText")}</h3>
                            <input placeholder={lang.get("viewAccountLoginEmail")} name="email" id="email" type="email" />
                            <br/>
                            <input placeholder={lang.get("viewAccountLoginPassword")} name="password" id="password" type="password" />
                            <br/><br/>
                            <button onclick={ctx.link().callback(|_| Msg::Submit)}>{lang.get("viewAccountLoginSubmit")}</button>
                            {self.status.clone()}
                        </div>
                    </div>
                </div>
            </>
        }
    }
}

impl Login {
    fn submit(&self, ctx: &Context<Self>) {
        let email = Input::by_id("email").value();
        let password = Input::by_id("password").value();

        if email.is_empty() || password.is_empty() {
            return;
        }

        let mut password_hash = [0u8; 32];
        Argon2::default()
            .hash_password_into(
                password.as_bytes(),
                format!("arlekin{}login", email).as_bytes(),
                &mut password_hash,
            )
            .unwrap();

        let app_callback = self.props.app_callback.clone();
        let status = ctx.link().callback(Msg::SetStatus);

        api::post("accounts/auth/login")
            .body(&json!({
                "email": email,
                "passwordHash": general_purpose::STANDARD.encode(password_hash)
            }))
            .send(
                app_callback.clone(),
                move |r: ApiResponse<LoginResponseData>| match r {
                    ApiResponse::Ok(r) => {
                        let mut message_encryption_hash = [0u8; 128];
                        Argon2::new(
                            Algorithm::default(),
                            Version::default(),
                            Params::new(65536, 3, 3, None).unwrap(),
                        )
                        .hash_password_into(
                            password.as_bytes(),
                            format!(
                                "arlekin{}message",
                                r.message_encryption_salt
                                    .to_le_bytes()
                                    .iter()
                                    .map(|&x| x as char)
                                    .collect::<String>()
                            )
                            .as_bytes(),
                            &mut message_encryption_hash,
                        )
                        .unwrap();

                        let b = app_callback.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let a = message_encryption_hash;
                            encryption::init(b.clone(), &a).await;

                            // TODO: remove this. Move to registration.
                            encryption::put_new_encryption_block(0).await;

                            // TODO: remove this. Move to app.
                            notifier::connect(b);
                        });
                        app_callback.emit(app::Msg::Login);
                    }
                    ApiResponse::BadRequest(err) => {
                        status.emit(Status::with_err(err));
                    }
                },
            );
    }
}
