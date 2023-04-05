use std::sync::Arc;

use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    app,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization,
    route::{self, Route},
};

pub struct Channel {
    props: Props,
    messages: Option<Vec<ChannelMessage>>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
    pub channel_id: i64,
}

pub enum Msg {
    Load(Vec<ChannelMessage>),
    Reload,
    Send,
}

pub struct ChannelMessage {
    pub message_id: i64,
    pub author_user_id: i64,
    pub text: Arc<String>,
}

impl Component for Channel {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            props: ctx.props().clone(),
            messages: None,
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => self.load(ctx),
            Msg::Load(messages) => self.messages = Some(messages),
            Msg::Send => self.send(ctx),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        let content = match &self.messages {
            Some(messages) => {
                let mut vec = Vec::with_capacity(messages.len());
                for message in messages.iter().rev() {
                    vec.push(html! {
                        <LoadUser<Arc<String>>
                            props={message.text.clone()}
                            app_callback={self.props.app_callback.clone()}
                            user_id={message.author_user_id}
                            view={Callback::from(process_message_view)}
                        />
                    });
                }

                vec
            }
            None => vec![html! { <p>{"Loading..."}</p> }],
        };

        html! { <>
            <route::Router route={Route::Chat { id: self.props.channel_id }} />
            <link rel="stylesheet" href="/static/css/channel_views/channel.css" />

            {content}

            <input type="text" name="message" id="message" />
            <button onclick={ctx.link().callback(|_| Msg::Send)}>{lang.get("viewChannelSendMessage")}</button>
        </> }
    }
}

impl Channel {
    fn load(&self, ctx: &Context<Self>) {
        let app_callback = self.props.app_callback.clone();
        let callback = ctx.link().callback(Msg::Load);
        let channel_id = self.props.channel_id;

        wasm_bindgen_futures::spawn_local(async move {
            callback.emit(encryption::get_messages(app_callback, channel_id, 0).await);
        });
    }

    fn send(&self, _: &Context<Self>) {
        let message = Input::by_id("message").value();
        encryption::send_message(
            self.props.app_callback.clone(),
            self.props.channel_id,
            message,
        );
    }
}

fn process_message_view(ctx: LoadUserContext<Arc<String>>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="channel-message">
            <img class="message-avatar noselect" src={user.avatar_url.clone()} alt={"avatar"} />
            <div>
                <label class="message-name">{user.name.clone()}</label>
                <br/>
                <label>{ctx.props}</label>
            </div>
        </div>
    }
}
