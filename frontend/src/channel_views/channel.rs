use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use arc_cell::ArcCell;
use lru::LruCache;
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    app,
    common::UnsafeSync,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization,
    route::{self, Route},
};

use super::channel_message_error::ChannelMessageError;

lazy_static! {
    static ref OPENED_CHANNEL: ArcCell<Option<(i64, UnsafeSync<Callback<Msg>>)>> =
        ArcCell::default();
    static ref CACHED_MESSAGES: Mutex<LruCache<i64, Arc<Mutex<Vec<ChannelMessage>>>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap()));
}

pub fn notify_message(channel_id: i64, message: ChannelMessage) {
    if let Some(messages) = CACHED_MESSAGES.lock().unwrap().get(&channel_id) {
        messages.lock().unwrap().push(message);
    }

    let opened_channel = OPENED_CHANNEL.get();
    if let Some((id, callback)) = opened_channel.as_ref() {
        if channel_id == *id {
            callback.0.emit(Msg::Refresh);
        }
    }
}

pub struct Channel {
    channel_id: i64,
    messages: Option<Arc<Mutex<Vec<ChannelMessage>>>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
    pub channel_id: i64,
}

pub enum Msg {
    Refresh,
    Reload,
    Load(Vec<ChannelMessage>),
    ChangeChannel,
    Send,
}

#[derive(Clone, PartialEq)]
pub struct ChannelMessage {
    pub message_id: i64,
    pub author_user_id: i64,
    pub text: Result<Arc<String>, ChannelMessageError>,
}

impl ChannelMessage {
    fn text_into_html(&self) -> Html {
        match self.text.clone() {
            Ok(text) => html! { text },
            Err(err) => {
                let lang = localization::get_language();
                html! {
                    <span class="message-error">{lang.get(err.to_translation_key())}</span>
                }
            }
        }
    }
}

impl Component for Channel {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let mut lock = CACHED_MESSAGES.lock().unwrap();
        let messages = lock.get(&ctx.props().channel_id);

        let s = Self {
            channel_id: ctx.props().channel_id,
            messages: messages.cloned(),
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
            Msg::Reload => {
                self.load(ctx);
                return false;
            }
            Msg::Load(messages) => self.load_set(ctx, messages),
            Msg::ChangeChannel => self.change_channel(ctx),
            Msg::Send => self.send(ctx),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if ctx.props().channel_id != self.channel_id {
            ctx.link().callback(|_| Msg::ChangeChannel).emit(());
        }

        let lang = localization::get_language();

        let content = match &self.messages {
            Some(arc) => {
                let messages = arc.lock().unwrap();

                let mut vec = Vec::with_capacity(messages.len());

                let mut last_author = 0;
                let mut count = 0;

                for message in messages.iter() {
                    vec.push(if last_author == message.author_user_id && count < 10 {
                        count += 1;
                        html! {
                            <div class="channel-message">
                                <label class="message-without-avatar">{message.text_into_html()}</label>
                            </div>
                        }
                    } else {
                        last_author = message.author_user_id;
                        count = 1;
                        html! {
                            <LoadUser<ChannelMessage>
                                props={message.clone()}
                                app_callback={ctx.props().app_callback.clone()}
                                user_id={message.author_user_id}
                                view={Callback::from(process_message_view)}
                            />
                        }
                    });
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        html! { <>
            <route::Router route={Route::Direct { id: ctx.props().channel_id }} />

            {content}

            <input type="text" name="message" id="message" />
            <button onclick={ctx.link().callback(|_| Msg::Send)}>{lang.get("viewChannelSendMessage")}</button>
        </> }
    }
}

impl Channel {
    fn load(&self, ctx: &Context<Self>) {
        let app_callback = ctx.props().app_callback.clone();
        let callback = ctx.link().callback(Msg::Load);
        let channel_id = ctx.props().channel_id;

        wasm_bindgen_futures::spawn_local(async move {
            callback.emit(encryption::get_messages(app_callback, channel_id, 0).await);
        });
    }

    fn load_set(&mut self, ctx: &Context<Self>, messages: Vec<ChannelMessage>) {
        if self.messages.is_none() {
            self.messages = Some(
                CACHED_MESSAGES
                    .lock()
                    .unwrap()
                    .get_or_insert(ctx.props().channel_id, || Arc::new(Mutex::new(Vec::new())))
                    .clone(),
            );
        }
        let destination = self.messages.as_ref().unwrap();

        let mut lock = destination.lock().unwrap();
        lock.clear();
        for message in messages.iter().rev() {
            lock.push(message.clone());
        }
    }

    fn change_channel(&mut self, ctx: &Context<Self>) {
        self.channel_id = ctx.props().channel_id;
        OPENED_CHANNEL.set(Arc::new(Some((
            ctx.props().channel_id,
            ctx.link().callback(|m| m).into(),
        ))));

        let mut lock = CACHED_MESSAGES.lock().unwrap();
        let messages = lock.get(&ctx.props().channel_id);
        self.messages = messages.cloned();

        if messages.is_none() {
            ctx.link().callback(|_| Msg::Reload).emit(());
        }
    }

    fn send(&self, ctx: &Context<Self>) {
        let input = Input::by_id("message");
        encryption::send_message(
            ctx.props().app_callback.clone(),
            ctx.props().channel_id,
            input.value(),
        );

        input.set_value("");
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
                <label>{ctx.props.text_into_html()}</label>
            </div>
        </div>
    }
}
