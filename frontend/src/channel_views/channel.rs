use std::sync::Arc;

use gloo_timers::callback::Timeout;
use yew::prelude::*;

use crate::{
    app::App,
    channel_views::channel_content::ChannelContent,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization,
    route::{self, Route},
};

use super::channel_content::{self, ChannelMessage};

pub struct Channel {
    sent_message_id: i64,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub channel_id: i64,
}

pub enum Msg {
    Refresh,
    ChangeChannel,
    Send,
}

impl Component for Channel {
    type Message = Msg;
    type Properties = Props;

    fn create(_: &Context<Self>) -> Self {
        Self { sent_message_id: 0 }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
            Msg::ChangeChannel => (),
            Msg::Send => self.send(ctx),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();
        html! {
            <div class="channel-container">
                <route::Router route={Route::Direct { id: ctx.props().channel_id }} />

                <div>
                    <h2>{"Channel name"}</h2>
                </div>

                <ChannelContent channel_id={ctx.props().channel_id} />

                <div class="channel-send-button-container">
                    <input type="text" name="message" id="message" />
                    <button onclick={ctx.link().callback(|_| Msg::Send)}>{lang.get("viewChannelSendMessage")}</button>
                </div>
            </div>
        }
    }
}

impl Channel {
    fn send(&mut self, ctx: &Context<Self>) {
        let input = Input::by_id("message");
        let value = input.value();

        let channel_id = ctx.props().channel_id;
        self.sent_message_id -= 1;
        let sent_message_id = self.sent_message_id;

        channel_content::notify_message(
            channel_id,
            ChannelMessage {
                message_id: sent_message_id,
                author_user_id: App::user_id(),
                text: Ok(Arc::new(input.value())),
            },
        );

        Timeout::new(0, move || {
            wasm_bindgen_futures::spawn_local(async move {
                let message_id = encryption::send_message(channel_id, value.clone()).await;

                channel_content::edit_message(
                    channel_id,
                    sent_message_id,
                    ChannelMessage {
                        message_id,
                        author_user_id: App::user_id(),
                        text: Ok(Arc::new(value)),
                    },
                );
            });
        })
        .forget();

        channel_content::set_scroll(channel_id, 0);

        input.set_value("");
    }
}
