use std::{
    collections::HashMap,
    marker::PhantomData,
    num::NonZeroUsize,
    sync::{atomic::AtomicI64, Arc, Mutex},
};

use lazy_static::__Deref;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use yew::prelude::*;

use crate::{api, common::UnsafeSync};

lazy_static! {
    static ref REQUESTING_LOCK: async_std::sync::Mutex<()> = async_std::sync::Mutex::new(());
    static ref CACHED_USERS: Mutex<LruCache<i64, Arc<User>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(2048).unwrap()));
    static ref NEXT_ID: AtomicI64 = AtomicI64::new(0);
    static ref DISPLAYED_COMPONENTS: Mutex<HashMap<i64, HashMap<i64, DisplayedComponent>>> =
        Mutex::new(HashMap::new());
}

pub fn received_user_status(data: ReceivedUserStatusData) {
    reload_user_status(data.user_id, data.status);
}

pub fn reload_user(user: User) {
    let user_id = user.user_id;
    CACHED_USERS.lock().unwrap().put(user_id, Arc::new(user));
    reload_worker(user_id);
}

pub fn reload_user_status(user_id: i64, status: UserStatus) {
    let mut users = CACHED_USERS.lock().unwrap();
    if let Some(user) = users.get(&user_id) {
        let mut user = user.deref().clone();
        user.status = Some(status);
        users.put(user_id, Arc::new(user));
    }

    reload_worker(user_id);
}

fn reload_worker(user_id: i64) {
    let components = DISPLAYED_COMPONENTS.lock().unwrap();
    if let Some(component_map) = components.get(&user_id) {
        for component in component_map.values() {
            component.callback.emit(Msg::Reload(false));
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedUserStatusData {
    user_id: i64,
    status: UserStatus,
}

struct DisplayedComponent {
    callback: UnsafeSync<Callback<Msg>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: i64,
    pub username: String,
    pub name: String,
    pub avatar_url: String,
    pub status: Option<UserStatus>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserStatus {
    pub status: i32,
    pub mobile: bool,
    pub rich_presence: String,
}

impl UserStatus {
    pub fn icon_html(&self) -> Html {
        let status = format!(
            "user-status {}",
            match self.status {
                0 => "user-status-offline",
                1 => "user-status-online",
                2 => "user-status-idle",
                3 => "user-status-donotdisturb",
                _ => unimplemented!(),
            }
        );

        html! {
            <div class={status}></div>
        }
    }
}

pub struct LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    id: i64,
    user: Option<Arc<User>>,
    phantom: PhantomData<T>,
}

pub struct LoadUserContext<T>
where
    T: Clone + PartialEq + 'static,
{
    pub user: Option<Arc<User>>,
    pub props: T,
}

pub enum Msg {
    Reload(bool),
    Load(Arc<User>),
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props<T>
where
    T: Clone + PartialEq + 'static,
{
    pub props: T,
    pub user_id: i64,
    pub view: Callback<LoadUserContext<T>, Html>,
    pub with_status: bool,
    pub refresh: bool,
}

impl<T> Component for LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    type Message = Msg;
    type Properties = Props<T>;

    fn create(ctx: &Context<Self>) -> Self {
        let s = Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            user: None,
            phantom: PhantomData,
        };
        s.changed_worker(ctx, 0);
        s
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        self.changed_worker(ctx, old_props.user_id);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reload(refresh) => {
                self.load(ctx, refresh);
                return false;
            }
            Msg::Load(user) => self.user = Some(user),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        ctx.props().view.emit(LoadUserContext {
            user: self.user.clone(),
            props: ctx.props().props.clone(),
        })
    }

    fn destroy(&mut self, ctx: &Context<Self>) {
        let mut lock = DISPLAYED_COMPONENTS.lock().unwrap();
        if let Some(set) = lock.get_mut(&ctx.props().user_id) {
            set.remove(&self.id);
            if set.is_empty() {
                lock.remove(&ctx.props().user_id);
            }
        }
    }
}

impl<T> LoadUser<T>
where
    T: Clone + PartialEq + 'static,
{
    fn changed_worker(&self, ctx: &Context<Self>, old_id: i64) {
        {
            let mut lock = DISPLAYED_COMPONENTS.lock().unwrap();
            if let Some(set) = lock.get_mut(&old_id) {
                set.remove(&self.id);
                if set.is_empty() {
                    lock.remove(&old_id);
                }
            }

            lock.entry(ctx.props().user_id)
                .or_insert_with(HashMap::new)
                .insert(
                    self.id,
                    DisplayedComponent {
                        callback: UnsafeSync(ctx.link().callback(|m: Msg| m)),
                    },
                );
        }

        self.load(ctx, ctx.props().refresh);
    }

    fn load(&self, ctx: &Context<Self>, refresh: bool) {
        let user_id = ctx.props().user_id;
        let with_status = ctx.props().with_status;

        let callback = ctx.link().callback(Msg::Load);
        let mut old_user = None;

        if let Some(user) = CACHED_USERS.lock().unwrap().get(&user_id) {
            if !refresh {
                if !with_status || user.status.is_some() == with_status {
                    callback.emit(user.clone());
                    return;
                }
            } else {
                old_user = Some(user.clone());
            }
        }

        wasm_bindgen_futures::spawn_local(async move {
            let _lock = REQUESTING_LOCK.lock().await;
            if let Some(user) = CACHED_USERS.lock().unwrap().get(&user_id) {
                if !refresh {
                    if !with_status || user.status.is_some() == with_status {
                        callback.emit(user.clone());
                        return;
                    }
                } else if let Some(old_user) = old_user {
                    if !Arc::ptr_eq(user, &old_user) {
                        callback.emit(user.clone());
                        return;
                    }
                }
            }

            let endpoint = match with_status {
                true => "accounts/user/withstatus",
                false => "accounts/user",
            };

            let response = api::get(endpoint)
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

            if refresh {
                reload_worker(user_id);
            }
        });
    }
}
