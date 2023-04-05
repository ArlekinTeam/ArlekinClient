use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use lru::LruCache;
use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{api, app};

lazy_static! {
    static ref REQUESTING_LOCK: async_std::sync::Mutex<()> = async_std::sync::Mutex::new(());
    static ref CACHED_USERS: Mutex<LruCache<i64, Arc<User>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(2048).unwrap()));
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub avatar_url: String,
}

pub struct LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    props: Props<T>,
    user: Option<Arc<User>>,
}

pub struct LoadUserContext<T>
where
    T: Clone + PartialEq + 'static,
{
    pub user: Option<Arc<User>>,
    pub props: T,
}

pub enum Msg {
    Reload,
    Load(Arc<User>),
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props<T>
where
    T: Clone + PartialEq + 'static,
{
    pub props: T,
    pub app_callback: Callback<app::Msg>,
    pub user_id: i64,
    pub view: Callback<LoadUserContext<T>, Html>,
}

impl<T> Component for LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    type Message = Msg;
    type Properties = Props<T>;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            props: ctx.props().clone(),
            user: None,
        };
        s.load(ctx);
        s
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload => {
                self.load(ctx);
                return false;
            }
            Msg::Load(user) => self.user = Some(user),
        };
        true
    }

    fn view(&self, _: &Context<Self>) -> Html {
        self.props.view.emit(LoadUserContext {
            user: self.user.clone(),
            props: self.props.props.clone(),
        })
    }
}

impl<T> LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    fn load(&self, ctx: &Context<Self>) {
        let callback = ctx.link().callback(Msg::Load);
        if let Some(user) = CACHED_USERS.lock().unwrap().get(&self.props.user_id) {
            callback.emit(user.clone());
            return;
        }

        let user_id = self.props.user_id;
        wasm_bindgen_futures::spawn_local(async move {
            let _lock = REQUESTING_LOCK.lock().await;
            if let Some(user) = CACHED_USERS.lock().unwrap().get(&user_id) {
                callback.emit(user.clone());
                return;
            }

            let response = api::get("accounts/user")
                .query([("id", user_id.to_string())])
                .send_async()
                .await;
            match response.status() {
                200 => {
                    let user = Arc::new(response.json::<User>().await.unwrap());
                    CACHED_USERS.lock().unwrap().put(user_id, user.clone());
                    callback.emit(user);
                }
                400 => todo!(),
                _ => unreachable!(),
            }
        });
    }
}
