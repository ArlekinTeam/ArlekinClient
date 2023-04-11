use serde::{Deserialize, Serialize};
use serde_json::json;
use yew::prelude::*;

use crate::{
    api::{self, ApiResponse, Platform},
    helpers::prelude::*,
    localization,
};

pub struct AddFriend {
    status: Html,
}

pub enum Msg {
    SetStatus(Html),
    Send,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequestsSendResponseData {
    receiver_user_id: i64,
}

impl Component for AddFriend {
    type Message = Msg;
    type Properties = ();

    fn create(_: &Context<Self>) -> Self {
        Self {
            status: Default::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SetStatus(status) => self.status = status,
            Msg::Send => {
                self.send(ctx);
                return false;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        html! { <>
            <label for="friendRequestUsername">{lang.get("viewAccountFriendRequestsInput")}</label>
            <br/>
            <input name="friendRequestUsername" id="friendRequestUsername" type="text" />
            <br/><br/>
            <button onclick={ctx.link().callback(|_| Msg::Send)}>{
                lang.get("viewAccountFriendRequestsSubmit")
            }</button>
            {self.status.clone()}
        </> }
    }
}

impl AddFriend {
    fn send(&self, ctx: &Context<Self>) {
        let username = Input::by_id("friendRequestUsername").value();
        let status = ctx.link().callback(Msg::SetStatus);

        api::put("accounts/friendrequests")
            .body(&json!({
                "platform": Platform::Native,
                "userIdentifier": username
            }))
            .send(
                move |r: ApiResponse<FriendRequestsSendResponseData>| match r {
                    ApiResponse::Ok(_) => {
                        status.emit(Status::with_ok("viewAccountFriendRequestsSentSuccess"));
                    }
                    ApiResponse::BadRequest(err) => {
                        status.emit(Status::with_err(err));
                    }
                },
            );
    }
}
