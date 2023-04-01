use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, User},
    api::{self, ApiResponse},
    app,
};

pub struct FriendsList {
    props: Props,
    data: Option<FriendsListLoadResponseData>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

pub enum Msg {
    Load(FriendsListLoadResponseData),
    Reload,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsListLoadResponseData {
    friends: Vec<i64>,
}

impl Component for FriendsList {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            props: ctx.props().clone(),
            data: None,
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => self.load(ctx),
            Msg::Load(data) => self.data = Some(data),
        };
        true
    }

    fn view(&self, _: &Context<Self>) -> Html {
        let data = match &self.data {
            Some(data) => {
                let mut vec = Vec::new();

                for e in &data.friends {
                    let user_id = *e;
                    vec.push(html! {
                        <div class="friends-profile-container">
                            <LoadUser
                                app_callback={self.props.app_callback.clone()}
                                user_id={user_id}
                                view={Callback::from(process_user_view)}
                            />
                        </div>
                    })
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        html! { <>
            {data}
        </> }
    }
}

impl FriendsList {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("accounts/friends").send(
            self.props.app_callback.clone(),
            move |r: ApiResponse<FriendsListLoadResponseData>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            },
        );
    }
}

fn process_user_view(user: Option<User>) -> Html {
    if user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = user.unwrap();

    html! {
        <div class="friends-profile">
            <img class="friends-avatar" src={user.avatar_url} alt={"avatar"} />
            <div class="select">
                <label class="friends-name">{user.name}</label>
                <br/>
                <span class="friends-username">{"@"}{user.username}</span>
            </div>
        </div>
    }
}
