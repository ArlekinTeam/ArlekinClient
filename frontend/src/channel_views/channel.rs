use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{
    api::{self, ApiResponse},
    app, localization,
};

pub struct Channel {
    props: Props,
    data: Option<ChannelLoadResponseData>,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

pub enum Msg {
    Load(ChannelLoadResponseData),
    Reload,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelLoadResponseData {
    channel_ids: Vec<i64>,
}

impl Component for Channel {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            props: ctx.props().clone(),
            data: None,
        };
        //s.load(ctx);
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
        let lang = localization::get_language();

        html! { <>
            <input type="text" name="message" id="message" />
            <button>{lang.get("viewChannelSendMessage")}</button>
        </> }
    }
}

impl Channel {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("channels/direct").send(
            self.props.app_callback.clone(),
            move |r: ApiResponse<ChannelLoadResponseData>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            },
        );
    }
}
