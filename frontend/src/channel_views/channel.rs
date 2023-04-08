use yew::prelude::*;

use crate::{
    app,
    channel_views::channel_content::ChannelContent,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization,
    route::{self, Route},
};

pub struct Channel {}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
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
        Self {}
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

                <ChannelContent app_callback={ctx.props().app_callback.clone()} channel_id={ctx.props().channel_id} />

                <div class="channel-send-button-container">
                    <input type="text" name="message" id="message" />
                    <button onclick={ctx.link().callback(|_| Msg::Send)}>{lang.get("viewChannelSendMessage")}</button>
                </div>
            </div>
        }
    }
}

impl Channel {
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
