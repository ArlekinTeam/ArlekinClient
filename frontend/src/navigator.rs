use std::sync::{Arc, Mutex};

use arc_cell::ArcCell;
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    api::{self, ApiResponse},
    app,
    common::UnsafeSync,
    direct_messages_views::direct_channels::{self, DirectChannelsLoadResponseData},
};

lazy_static! {
    pub(crate) static ref CACHED_DATA: ArcCell<Mutex<NavigatorCache>> =
        ArcCell::new(Arc::new(Mutex::new(NavigatorCache {
            direct_channels: None,
        })));
    static ref INSTANCE: ArcCell<Option<UnsafeSync<Callback<Msg>>>> = ArcCell::default();
}

pub fn update_activity(direct_channel_id: i64) {
    let cache = CACHED_DATA.get();
    let mut lock = cache.lock().unwrap();
    let data = match &mut lock.direct_channels {
        Some(data) => data,
        None => return,
    };

    let mut found_index = None;
    for (i, element) in data.direct_channels.iter_mut().enumerate() {
        if element.direct_channel_id == direct_channel_id {
            found_index = Some(i);
            break;
        }
    }

    update_activity_worker(found_index, data);
    refresh();
}

pub fn add_pings(channel_id: i64, pings_to_add: i64, new_last_read_message_id: i64) {
    let cache = CACHED_DATA.get();
    let mut lock = cache.lock().unwrap();
    let data = match &mut lock.direct_channels {
        Some(data) => data,
        None => return,
    };

    let mut found_index = None;
    for (i, element) in data.direct_channels.iter_mut().enumerate() {
        if element.direct_channel_id == channel_id {
            element.unread_count += pings_to_add;
            if element.unread_count < 0 {
                element.unread_count = 0;
            }
            found_index = Some(i);

            if pings_to_add < 0 {
                element.last_read_direct_message_id = new_last_read_message_id;
            }
            break;
        }
    }

    if pings_to_add > 0 {
        update_activity_worker(found_index, data);
    }
    refresh();
}

fn update_activity_worker(found_index: Option<usize>, data: &mut DirectChannelsLoadResponseData) {
    if let Some(index) = found_index {
        let element = data.direct_channels.remove(index);
        data.direct_channels.insert(0, element);
    }
}

fn refresh() {
    if let Some(instance) = &*INSTANCE.get() {
        instance.0.emit(Msg::Refresh);
    }
    if let Some(instance) = &*direct_channels::INSTANCE.get() {
        instance.0.emit(direct_channels::Msg::Refresh);
    }
}

pub struct Navigator {
    cache: Arc<Mutex<NavigatorCache>>,
}

pub enum Msg {
    Refresh,
    Reload,
}

pub(crate) struct NavigatorCache {
    pub direct_channels: Option<DirectChannelsLoadResponseData>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

impl Component for Navigator {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            cache: CACHED_DATA.get(),
        };
        s.load(ctx);
        INSTANCE.set(Arc::new(Some(UnsafeSync(ctx.link().callback(|m| m)))));
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
            Msg::Reload => {
                self.load(ctx);
                return false;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lock = self.cache.lock().unwrap();
        let data = match &lock.direct_channels {
            Some(data) => {
                let mut vec = Vec::new();

                for data in &data.direct_channels {
                    if data.unread_count == 0 {
                        continue;
                    }

                    let avatar_element = if data.is_group {
                        todo!();
                    } else {
                        html! {
                            <LoadUser<()>
                                props={()}
                                user_id={data.user_id}
                                view={Callback::from(process_user_channel_view)}
                                with_status={false}
                                refresh={false}
                            />
                        }
                    };

                    let a = ctx.props().app_callback.clone();
                    let channel_id = data.direct_channel_id;
                    vec.push(html! {
                        <div class="app-navigator-button-container">
                            <div
                                onclick={Callback::from(move |_| a.emit(app::Msg::OpennedChannel(channel_id)))}
                                class="app-navigator-button"
                            >
                                {avatar_element}
                            </div>
                            <div class="navigator-notification"><span>{data.unread_count}</span></div>
                        </div>
                    })
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        html! {
            <nav class="noselect app-navigator">
                <div class="app-navigator-inner">
                    {data}
                </div>
            </nav>
        }
    }

    fn destroy(&mut self, _: &Context<Self>) {
        INSTANCE.set(Arc::new(None));
    }
}

impl Navigator {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(|_: ()| Msg::Refresh);

        api::get("channels/direct").send(move |r: ApiResponse<DirectChannelsLoadResponseData>| {
            match r {
                ApiResponse::Ok(mut r) => {
                    r.direct_channels
                        .sort_by(|a, b| b.recent_activity.cmp(&a.recent_activity));

                    let cache = CACHED_DATA.get();
                    let mut lock = cache.lock().unwrap();
                    lock.direct_channels = Some(r);

                    callback.emit(());
                    if let Some(instance) = &*direct_channels::INSTANCE.get() {
                        instance.0.emit(direct_channels::Msg::Refresh);
                    }
                }
                ApiResponse::BadRequest(_) => todo!(),
            }
        });
    }
}

fn process_user_channel_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <img class="app-navigator-image" src={user.avatar_url.to_owned()} alt={"avatar"} />
    }
}
