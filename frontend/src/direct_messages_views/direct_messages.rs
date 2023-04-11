use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    api::{self, ApiResponse},
    app,
};

pub struct DirectMessages {
    props: Props,
    data: Option<DirectMessagesLoadResponseData>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

pub enum Msg {
    Load(DirectMessagesLoadResponseData),
    Reload,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectMessagesLoadResponseData {
    direct_channels: Vec<DirectChannelResponseData>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectChannelResponseData {
    direct_channel_id: i64,
    is_group: bool,
    user_id: i64,
    group_data: Option<GroupResponseData>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupResponseData {
    name: String,
    avatar_url: String,
    user_count: i32,
}

impl Component for DirectMessages {
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

                for data in &data.direct_channels {
                    if data.is_group {
                        todo!();
                    }

                    let a = self.props.app_callback.clone();
                    let channel_id = data.direct_channel_id;
                    vec.push(html! {
                        <div
                            onclick={Callback::from(move |_| a.emit(app::Msg::OpennedChannel(channel_id)))}
                            class="user-profile-container"
                        >
                            <LoadUser<()>
                                props={()}
                                user_id={data.user_id}
                                view={Callback::from(process_user_channel_view)}
                            />
                        </div>
                    })
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        let a = self.props.app_callback.clone();
        html! { <>
            <p onclick={Callback::from(move |_| a.emit(app::Msg::OpennedChannel(0)))}>{"Friends"}</p>

            {data}
        </> }
    }
}

impl DirectMessages {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("channels/direct/todo2").send(
            self.props.app_callback.clone(),
            move |r: ApiResponse<DirectMessagesLoadResponseData>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            },
        );
    }
}

fn process_user_channel_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    process_channel_view(&user.name, &user.avatar_url)
}

fn process_channel_view(name: &str, avatar_url: &str) -> Html {
    html! {
        <div class="user-profile">
            <img class="user-avatar noselect" src={avatar_url.to_owned()} alt={"avatar"} />
            <span class="user-name">{name.to_owned()}</span>
        </div>
    }
}
