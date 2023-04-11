use serde::{Deserialize, Serialize};
use serde_json::json;
use yew::prelude::*;

use crate::{
    account::load_user::{LoadUser, LoadUserContext},
    api::{self, ApiResponse},
    localization,
};

pub struct FriendRequests {
    data: Option<FriendRequestsLoadResponseData>,
}

pub enum Msg {
    Reload,
    Load(FriendRequestsLoadResponseData),
    Reject(i64),
    Accept(i64),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequestsLoadResponseData {
    received: Vec<i64>,
    sent: Vec<i64>,
}

impl Component for FriendRequests {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            data: None,
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => self.load(ctx),
            Msg::Load(data) => self.data = Some(data),
            Msg::Reject(requested_friend_user_id) => {
                self.reject(ctx, requested_friend_user_id);
                return false;
            }
            Msg::Accept(requested_friend_user_id) => {
                self.accept(ctx, requested_friend_user_id);
                return false;
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        let data = match &self.data {
            Some(data) => {
                let mut vec = Vec::new();

                if !data.received.is_empty() {
                    vec.push(html! { <h4>{lang.get("viewAccountFriendRequestsReceived")}</h4> });
                    for e in &data.received {
                        let user_id = *e;
                        vec.push(html! {
                            <div class="friends-profile-container">
                                <LoadUser<()>
                                    props={()}
                                    user_id={user_id}
                                    view={Callback::from(process_user_view)}
                                />

                                <div>
                                    <button onclick={ctx.link().callback(move |_| Msg::Accept(user_id))}>{
                                        lang.get("viewAccountFriendRequestsAccept")
                                    }</button>
                                    <button onclick={ctx.link().callback(move |_| Msg::Reject(user_id))}>{
                                        lang.get("viewAccountFriendRequestsReject")
                                    }</button>
                                </div>
                            </div>
                        })
                    }
                }

                if !data.sent.is_empty() {
                    vec.push(html! { <h4>{lang.get("viewAccountFriendRequestsSent")}</h4> });
                    for e in &data.sent {
                        let user_id = *e;
                        vec.push(html! {
                            <div class="user-profile-container friends-profile-container">
                                <LoadUser<()>
                                    props={()}
                                    user_id={user_id}
                                    view={Callback::from(process_user_view)}
                                />

                                <button onclick={ctx.link().callback(move |_| Msg::Reject(user_id))}>{
                                    lang.get("viewAccountFriendRequestsCancel")
                                }</button>
                            </div>
                        })
                    }
                }

                if data.received.is_empty() && data.sent.is_empty() {
                    vec.push(html! {
                        <h2>{lang.get("viewAccountFriendRequestsEmpty")}</h2>
                    });
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

impl FriendRequests {
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);

        api::get("accounts/friendrequests").send(
            move |r: ApiResponse<FriendRequestsLoadResponseData>| match r {
                ApiResponse::Ok(r) => callback.emit(r),
                ApiResponse::BadRequest(_) => todo!(),
            },
        );
    }

    fn reject(&self, ctx: &Context<Self>, requested_friend_user_id: i64) {
        let callback = ctx.link().callback(|_: ()| Msg::Reload);

        api::delete("accounts/friendrequests")
            .body(&json!({
                "requestedFriendUserId": requested_friend_user_id
            }))
            .send_without_ok(move |r| match r {
                ApiResponse::Ok(_) => callback.emit(()),
                ApiResponse::BadRequest(_) => todo!(),
            });
    }

    fn accept(&self, ctx: &Context<Self>, requested_friend_user_id: i64) {
        let callback = ctx.link().callback(|_: ()| Msg::Reload);

        api::post("accounts/friendrequests")
            .body(&json!({
                "requestedFriendUserId": requested_friend_user_id
            }))
            .send_without_ok(move |r| match r {
                ApiResponse::Ok(_) => callback.emit(()),
                ApiResponse::BadRequest(_) => todo!(),
            });
    }
}

fn process_user_view(ctx: LoadUserContext<()>) -> Html {
    if ctx.user.is_none() {
        return html! { {"Loading..."} };
    }
    let user = ctx.user.unwrap();

    html! {
        <div class="user-profile">
            <img class="user-avatar" src={user.avatar_url.clone()} alt={"avatar"} />
            <div class="select">
                <label class="user-name">{user.name.clone()}</label>
                <br/>
                <span class="user-info">{"@"}{user.username.clone()}</span>
            </div>
        </div>
    }
}
