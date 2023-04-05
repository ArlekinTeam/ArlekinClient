use serde::{Deserialize, Serialize};
use serde_json::json;
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    api::{self, ApiResponse},
    app, localization,
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
    OpenChannel(i64),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendsListLoadResponseData {
    friends: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChannelIdFromUserIdResponseData {
    channel_id: i64,
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
            Msg::Reload => {
                self.load(ctx);
                return false;
            }
            Msg::Load(data) => self.data = Some(data),
            Msg::OpenChannel(friend_user_id) => {
                self.open_channel(friend_user_id);
                return false;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        let data = match &self.data {
            Some(data) => {
                let mut vec = Vec::new();

                for e in &data.friends {
                    let user_id = *e;
                    vec.push(html! {
                        <div class="friends-profile-container">
                            <LoadUser<()>
                                props={()}
                                app_callback={self.props.app_callback.clone()}
                                user_id={user_id}
                                view={Callback::from(process_user_view)}
                            />

                            <button onclick={ctx.link().callback(move |_| Msg::OpenChannel(user_id))}>{
                                lang.get("viewAccountFriendsOpenChat")
                            }</button>
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

    fn open_channel(&self, friend_user_id: i64) {
        let callback = self.props.app_callback.clone();

        api::post("channels/direct/getchannelidfromuserid")
            .body(&json!({ "userId": friend_user_id }))
            .send(
                self.props.app_callback.clone(),
                move |r: ApiResponse<GetChannelIdFromUserIdResponseData>| match r {
                    ApiResponse::Ok(r) => callback.emit(app::Msg::OpennedChannel(r.channel_id)),
                    ApiResponse::BadRequest(_) => todo!(),
                },
            );
    }
}

fn process_user_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="friends-profile">
            <img class="friends-avatar" src={user.avatar_url.clone()} alt={"avatar"} />
            <div class="select">
                <label class="friends-name">{user.name.clone()}</label>
                <br/>
                <span class="friends-username">{"@"}{user.username.clone()}</span>
            </div>
        </div>
    }
}
