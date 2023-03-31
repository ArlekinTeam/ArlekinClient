use serde::{Serialize, Deserialize};
use yew::prelude::*;

use crate::{app, api::{self, ApiResponse}};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub avatar_url: String
}


pub struct LoadUser {
    props: Props,
    user: Option<User>
}

pub enum Msg {
    Reload,
    Load(User)
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
    pub user_id: i64,
    pub view: Callback<Option<User>, Html>
}

impl Component for LoadUser {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            props: ctx.props().clone(),
            user: None
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => {
                self.load(ctx);
                return false;
            },
            Msg::Load(user) => self.user = Some(user)
        };
        true
    }

    fn view(&self, _: &Context<Self>) -> Html {
        self.props.view.emit(self.user.clone())
    }
}

impl LoadUser {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("accounts/user").query([("id", self.props.user_id.to_string())]).send(
            self.props.app_callback.clone(),
            move |r: ApiResponse<User>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            }
        );
    }
}
