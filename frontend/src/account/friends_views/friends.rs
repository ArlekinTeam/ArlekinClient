use yew::prelude::*;

use crate::{
    account::friends_views::{
        add_friend::AddFriend, friend_requests::FriendRequests, friends_list::FriendsList,
    },
    app, localization,
    route::{self, Route},
};

pub struct Friends {
    props: Props,
    state: Msg,
}

#[derive(PartialEq)]
pub enum Msg {
    Online,
    All,
    Pending,
    Add,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub app_callback: Callback<app::Msg>,
}

impl Component for Friends {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            props: ctx.props().clone(),
            state: Msg::Online,
        }
    }

    fn update(&mut self, _: &Context<Self>, msg: Self::Message) -> bool {
        match self.state == msg {
            true => return false,
            false => self.state = msg,
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();

        let app_callback = self.props.app_callback.clone();
        let content = match self.state {
            Msg::Online => html! { <FriendsList {app_callback} /> },
            Msg::All => html! { <FriendsList {app_callback} /> },
            Msg::Pending => html! { <FriendRequests /> },
            Msg::Add => html! { <AddFriend /> },
        };

        let add_friend_class = match self.state == Msg::Add {
            true => "add-friend-selected",
            false => "add-friend",
        };

        html! {
            <div class="noselect">
                <route::Router route={Route::Friends} />

                <header class="friends-header">
                    <h1>{lang.get("viewAccountFriendsTitle")}</h1>
                    <button
                        onclick={ctx.link().callback(|_| Msg::Online)} class={self.get_selected(Msg::Online)}
                    >{lang.get("viewAccountFriendsOnline")}</button>
                    <button
                        onclick={ctx.link().callback(|_| Msg::All)} class={self.get_selected(Msg::All)}
                    >{lang.get("viewAccountFriendsAll")}</button>
                    <button
                        onclick={ctx.link().callback(|_| Msg::Pending)} class={self.get_selected(Msg::Pending)}
                    >{lang.get("viewAccountFriendsPending")}</button>
                    <button
                        onclick={ctx.link().callback(|_| Msg::Add)} class={add_friend_class}
                    >{lang.get("viewAccountFriendsAdd")}</button>
                </header>

                {content}
            </div>
        }
    }
}

impl Friends {
    fn get_selected(&self, expected: Msg) -> String {
        match self.state == expected {
            true => "button-selected",
            false => "",
        }
        .to_owned()
    }
}
