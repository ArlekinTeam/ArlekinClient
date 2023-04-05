use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{
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
    channel_ids: Vec<i64>,
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

                for e in &data.channel_ids {
                    let channel_id = *e;
                    vec.push(html! {
                        <div class="friends-profile-container">
                            <h2>{channel_id}</h2>
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

impl DirectMessages {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("channels/direct").send(
            self.props.app_callback.clone(),
            move |r: ApiResponse<DirectMessagesLoadResponseData>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            },
        );
    }
}
