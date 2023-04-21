use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use arc_cell::ArcCell;
use gloo_timers::callback::Timeout;
use lru::LruCache;
use serde_json::json;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    api::{self, ApiResponse},
    common::UnsafeSync,
    direct_messages_views::encryption,
    helpers::prelude::*,
    navigator,
};

use super::channel_message::ChannelMessage;

lazy_static! {
    static ref OPENED_CHANNEL: ArcCell<Option<(i64, UnsafeSync<Callback<Msg>>)>> =
        ArcCell::default();
    static ref CACHED_CHANNELS: Mutex<LruCache<i64, Arc<Mutex<ChannelCache>>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap()));
}

pub fn notify_message(channel_id: i64, message: ChannelMessage) {
    if let Some(lock) = CACHED_CHANNELS.lock().unwrap().get(&channel_id) {
        let mut lock = lock.lock().unwrap();
        for m in lock.messages.iter_mut() {
            if m.1.message_id == message.message_id {
                return;
            }
        }

        lock.messages.push((message.message_id, message));
    }

    if !refresh_channel(channel_id) || !WebPage::is_focused() {
        navigator::add_pings(channel_id, 1, 0);
    } else {
        navigator::update_activity(channel_id);
    }
}

pub fn edit_message(channel_id: i64, message_id: i64, message: ChannelMessage) {
    if let Some(cache) = CACHED_CHANNELS.lock().unwrap().get(&channel_id) {
        let mut lock = cache.lock().unwrap();

        let id = message.message_id;
        for i in 0..lock.messages.len() {
            if lock.messages[i].1.message_id == message_id {
                lock.messages[i].1 = message;
                break;
            }
        }

        if message_id != id {
            lock.messages.dedup_by(
                |a, b| a.1.message_id == b.1.message_id
            );
        }
    }
    refresh_channel(channel_id);
}

pub fn set_scroll(channel_id: i64, scroll: i32) {
    if let Some(cache) = CACHED_CHANNELS.lock().unwrap().get(&channel_id) {
        let mut lock = cache.lock().unwrap();
        lock.scroll_y = scroll;
    }
    refresh_channel(channel_id);
}

fn refresh_channel(channel_id: i64) -> bool {
    let opened_channel = OPENED_CHANNEL.get();
    if let Some((id, callback)) = opened_channel.as_ref() {
        if channel_id == *id {
            callback.0.emit(Msg::Refresh);
            return true;
        }
    }
    false
}

pub struct ChannelContent {
    cache: Option<Arc<Mutex<ChannelCache>>>,
    scroll_event: Closure<dyn FnMut()>,
    latest_before: i64,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub channel_id: i64,
}

pub enum Msg {
    Refresh,
    Reload,
    Load(Vec<ChannelMessage>),
    ChangeChannel,
    SetScroll(i32),
    LoadUp,
}

struct ChannelCache {
    messages: Vec<(i64, ChannelMessage)>,
    is_scrolled_to_top: bool,
    scroll_y: i32,
}

impl Component for ChannelContent {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let callback = ctx.link().callback(|m| m);
        let scroll_event = Closure::new(move || {
            let scroll = Element::by_id("channel-content-scroll");
            callback.emit(Msg::SetScroll(
                scroll.scroll_height() - scroll.scroll_top() - scroll.client_height(),
            ));

            if scroll.scroll_top() < 500 {
                callback.emit(Msg::LoadUp);
            }
        });

        let mut s = Self {
            cache: None,
            scroll_event,
            latest_before: 0,
        };
        s.change_channel(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
            Msg::Reload => {
                self.load(ctx, 0);
                return false;
            }
            Msg::Load(messages) => self.load_set(ctx, messages),
            Msg::ChangeChannel => self.change_channel(ctx),
            Msg::SetScroll(scroll) => self.set_scroll(ctx, scroll),
            Msg::LoadUp => {
                self.load_up(ctx);
                return false;
            }
        };
        true
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        if ctx.props().channel_id != old_props.channel_id {
            ctx.link().callback(|_| Msg::ChangeChannel).emit(());
        }
        false
    }

    fn view(&self, _: &Context<Self>) -> Html {
        let content = match &self.cache {
            Some(arc) => {
                let cache = arc.lock().unwrap();
                let mut vec = Vec::with_capacity(cache.messages.len() + 1);

                if cache.is_scrolled_to_top {
                    vec.push(html! {
                        <h2>{"This is the beginning of the chat"}</h2>
                    });
                }

                let mut last_author = 0;
                let mut count = 0;

                for message in cache.messages.iter() {
                    let html = if last_author == message.1.author_user_id && count < 10 {
                        count += 1;
                        html! {
                            <div class="channel-message">
                                <label class="message-without-avatar">{message.1.get_html().clone()}</label>
                            </div>
                        }
                    } else {
                        last_author = message.1.author_user_id;
                        count = 1;
                        html! {
                            <LoadUser<ChannelMessage>
                                props={message.1.clone()}
                                user_id={message.1.author_user_id}
                                view={Callback::from(process_message_view)}
                                with_status={false}
                                refresh={false}
                            />
                        }
                    };
                    vec.push(html! {
                        <div key={message.0}>
                            {html}
                        </div>
                    })
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        html! {
            <div class="channel-content" id="channel-content-scroll">
                <div class="channel-content-inner">
                    {content}
                </div>
            </div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        let scroll = Element::by_id("channel-content-scroll");
        scroll
            .add_event_listener_with_callback("scroll", self.scroll_event.as_ref().unchecked_ref())
            .unwrap();

        if let Some(cache) = &self.cache {
            let lock = cache.lock().unwrap();
            scroll.set_scroll_top(scroll.scroll_height() - lock.scroll_y - scroll.client_height());
        }
    }
}

impl ChannelContent {
    fn load(&self, ctx: &Context<Self>, before: i64) {
        let callback = ctx.link().callback(Msg::Load);
        let channel_id = ctx.props().channel_id;

        Timeout::new(0, move || {
            wasm_bindgen_futures::spawn_local(async move {
                callback.emit(encryption::get_messages(channel_id, before).await);
            });
        })
        .forget();
    }

    fn load_set(&mut self, ctx: &Context<Self>, messages: Vec<ChannelMessage>) {
        if self.cache.is_none() {
            self.cache = Some(
                CACHED_CHANNELS
                    .lock()
                    .unwrap()
                    .get_or_insert(ctx.props().channel_id, || {
                        Arc::new(Mutex::new(ChannelCache {
                            messages: Vec::new(),
                            is_scrolled_to_top: false,
                            scroll_y: 0,
                        }))
                    })
                    .clone(),
            );
        }
        let destination = self.cache.as_ref().unwrap();
        let mut lock = destination.lock().unwrap();

        if messages.len() < 50 {
            lock.is_scrolled_to_top = true;
        }

        for message in messages {
            lock.messages.insert(0, (message.message_id, message.clone()));
        }
    }

    fn load_up(&mut self, ctx: &Context<Self>) {
        if let Some(cache) = &self.cache {
            if cache.lock().unwrap().is_scrolled_to_top {
                return;
            }
        }

        let before = match self.cache.as_ref() {
            Some(arc) => {
                let cache = arc.lock().unwrap();
                if cache.messages.is_empty() {
                    return;
                }
                cache.messages[0].1.message_id
            }
            None => return,
        };

        if before != self.latest_before {
            self.latest_before = before;
            self.load(ctx, before);
        }
    }

    fn change_channel(&mut self, ctx: &Context<Self>) {
        OPENED_CHANNEL.set(Arc::new(Some((
            ctx.props().channel_id,
            ctx.link().callback(|m| m).into(),
        ))));

        let mut lock = CACHED_CHANNELS.lock().unwrap();
        let messages = lock.get(&ctx.props().channel_id);
        self.cache = messages.cloned();

        if messages.is_none() {
            self.load(ctx, 0);
        }
    }

    fn set_scroll(&self, ctx: &Context<Self>, scroll: i32) {
        if let Some(cache) = &self.cache {
            let mut lock = cache.lock().unwrap();
            lock.scroll_y = scroll;

            // Send ack.
            if scroll == 0 && WebPage::is_focused() {
                let last = lock.messages[lock.messages.len() - 1].1.message_id;
                self.send_ack(ctx, last);
                navigator::add_pings(ctx.props().channel_id, i64::MIN, last);
            }
        }
    }

    fn send_ack(&self, ctx: &Context<Self>, last_read_message_id: i64) {
        api::post("channels/direct/messages/ack")
            .body(&json!({
                "directChannelId": ctx.props().channel_id,
                "lastReadDirectMessageId": last_read_message_id
            }))
            .send_without_ok(move |r| match r {
                ApiResponse::Ok(_) => (),
                ApiResponse::BadRequest(_) => todo!(),
            });
    }
}

fn process_message_view(ctx: LoadUserContext<ChannelMessage>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="channel-message channel-message-with-avatar">
            <img class="message-avatar noselect" src={user.avatar_url.clone()} alt={"avatar"} />
            <div>
                <label class="message-name">{user.name.clone()}</label>
                <br/>
                <label>{ctx.props.get_html().clone()}</label>
            </div>
        </div>
    }
}
