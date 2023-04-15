use std::sync::{Arc, Mutex};

use arc_cell::ArcCell;
use serde::{Deserialize, Serialize};
use yew::prelude::*;

lazy_static! {
    pub(crate) static ref INSTANCE: ArcCell<Option<UnsafeSync<Callback<Msg>>>> = ArcCell::default();
}

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    app,
    common::UnsafeSync,
    navigator::{self, NavigatorCache},
};

pub struct DirectChannels {
    cache: Arc<Mutex<NavigatorCache>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

pub enum Msg {
    Refresh,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectChannelsLoadResponseData {
    pub direct_channels: Vec<DirectChannelResponseData>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DirectChannelResponseData {
    pub unread_count: i64,
    pub recent_activity: i64,
    pub is_group: bool,
    pub user_id: i64,
    pub group_data: Option<GroupResponseData>,
    pub direct_channel_id: i64,
    pub last_read_direct_message_id: i64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponseData {
    pub name: String,
    pub avatar_url: String,
    user_count: i32,
}

impl Component for DirectChannels {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            cache: navigator::CACHED_DATA.get(),
        };
        INSTANCE.set(Arc::new(Some(UnsafeSync(ctx.link().callback(|m| m)))));
        s
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lock = self.cache.lock().unwrap();
        let data = match &lock.direct_channels {
            Some(data) => {
                let mut vec = Vec::new();

                for data in &data.direct_channels {
                    if data.is_group {
                        todo!();
                    }

                    let a = ctx.props().app_callback.clone();
                    let channel_id = data.direct_channel_id;

                    let class = format!(
                        "user-profile-container channel-{}",
                        match data.unread_count == 0 {
                            true => "read",
                            false => "unread",
                        }
                    );

                    vec.push(html! {
                        <div
                            onclick={Callback::from(move |_| a.emit(app::Msg::OpennedChannel(channel_id)))}
                            class={class}
                        >
                            <LoadUser<()>
                                props={()}
                                user_id={data.user_id}
                                view={Callback::from(process_user_channel_view)}
                                with_status={true}
                                refresh={false}
                            />
                        </div>
                    })
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        let a = ctx.props().app_callback.clone();
        html! { <>
            <p onclick={Callback::from(move |_| a.emit(app::Msg::OpennedChannel(0)))}>{"Friends"}</p>

            {data}
        </> }
    }

    fn destroy(&mut self, _: &Context<Self>) {
        INSTANCE.set(Arc::new(None));
    }
}

fn process_user_channel_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="user-profile">
            <img class="user-avatar noselect" src={user.avatar_url.to_owned()} alt={"avatar"} />
            {user.status.as_ref().unwrap().icon_html()}
            <div class="user-content">
                <span class="user-name">{user.name.to_owned()}</span>
            </div>
        </div>
    }
}
