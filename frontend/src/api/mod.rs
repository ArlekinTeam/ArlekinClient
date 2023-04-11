use arc_cell::ArcCell;
use const_format::concatcp;
use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use yew::prelude::*;

use crate::{
    app::{self, App},
    app_status_bar::AppStatusBar,
    common::threading, helpers::prelude::*,
};

//const DOMAIN: &str = "http://localhost:9080";
const DOMAIN: &str = "https://test-fsqa7u.noisestudio.net";
const API_ENDPOINT: &str = concatcp!(DOMAIN, "/api/v1/");

lazy_static! {
    static ref REFRESH_TOKEN: ArcCell<Uuid> = ArcCell::default();
    static ref REQUEST_LOCK: async_std::sync::RwLock<()> = async_std::sync::RwLock::new(());
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum Platform {
    Native = 0,
}

pub struct ApiRequest {
    kind: ApiRequestKind,
    endpoint: String,
    query: Option<Vec<(String, String)>>,
    body: Option<String>,
}

pub enum ApiResponse<T> {
    Ok(T),
    BadRequest(HashMap<String, ErrorDataElement>),
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDataElement {
    pub code: u32,
    pub translation_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorData {
    pub errors: HashMap<String, ErrorDataElement>,
}

enum ApiRequestKind {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UnauthorizedData {
    is_expired: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RefreshTokenData {
    refresh_token: Uuid,
}

pub fn get(endpoint: &str) -> ApiRequest {
    ApiRequest::new(ApiRequestKind::Get, endpoint)
}

pub fn post(endpoint: &str) -> ApiRequest {
    ApiRequest::new(ApiRequestKind::Post, endpoint)
}

pub fn put(endpoint: &str) -> ApiRequest {
    ApiRequest::new(ApiRequestKind::Put, endpoint)
}

pub fn delete(endpoint: &str) -> ApiRequest {
    ApiRequest::new(ApiRequestKind::Delete, endpoint)
}

pub fn try_load() -> bool {
    let value = WebPage::local_storage().get_item("refresh_token")
        .expect("Unable to get refresh_token from session storage.");
    if let Some(refresh_token) = value {
        REFRESH_TOKEN.set(Arc::new(Uuid::parse_str(&refresh_token).unwrap()));
        true
    } else {
        false
    }
}

pub fn set_refresh_token(refresh_token: Uuid) {
    REFRESH_TOKEN.set(Arc::new(refresh_token));

    WebPage::local_storage().set_item("refresh_token", &refresh_token.to_string())
        .expect("Unable to set refresh_token to session storage.");
}

impl ApiRequest {
    fn new(kind: ApiRequestKind, endpoint: &str) -> Self {
        Self {
            kind,
            endpoint: endpoint.to_owned(),
            query: None,
            body: None,
        }
    }

    fn final_endpoint(endpoint: &str) -> String {
        format!("{API_ENDPOINT}{endpoint}")
    }

    pub fn query<'a, T, V>(mut self, params: T) -> Self
    where
        T: IntoIterator<Item = (&'a str, V)>,
        V: AsRef<str>,
    {
        let mut vec = Vec::new();
        for (name, value) in params {
            vec.push((name.to_owned(), value.as_ref().to_owned()));
        }
        self.query = Some(vec);
        self
    }

    pub fn body<T: serde::ser::Serialize + ?Sized>(mut self, value: &T) -> Self {
        self.body = Some(serde_json::to_string(value).unwrap());
        self
    }

    pub async fn send_async(&self) -> Response {
        let mut wait_time = 0;
        loop {
            if wait_time != 0 {
                threading::sleep(wait_time).await;
                wait_time = 0;
            }

            let response = match self.create_request_and_send_read_lock().await {
                Ok(r) => r,
                Err(_) => {
                    AppStatusBar::set_connection(false);
                    wait_time = 3000;
                    continue;
                }
            };

            match response.status() {
                200 | 400 => {
                    AppStatusBar::set_connection(true);
                    return response;
                }
                401 => {
                    AppStatusBar::set_connection(true);
                    match response.json::<UnauthorizedData>().await {
                        Ok(data) => match data.is_expired {
                            true => {
                                self.refresh_token().await;
                                continue;
                            }
                            false => {
                                App::logout_without_api();
                                panic!("Logged out.");
                            }
                        },
                        Err(_) => unreachable!(),
                    }
                }
                403 => {
                    AppStatusBar::set_connection(true);
                    panic!("Forbidden.")
                }
                408 | 500 | 502 | 504 => {
                    AppStatusBar::set_connection(false);
                    wait_time = 1000;
                    continue;
                }
                503 => {
                    AppStatusBar::set_connection(false);
                    wait_time = 5000;
                    continue;
                }
                _ => panic!("Unable to send API request."),
            };
        }
    }

    pub fn send<F, T>(self, _app_callback: Callback<app::Msg>, callback: F)
    where
        F: FnOnce(ApiResponse<T>) + 'static,
        T: DeserializeOwned,
    {
        wasm_bindgen_futures::spawn_local(async move {
            let response = self.send_async().await;
            match response.status() {
                200 => callback(ApiResponse::Ok(response.json::<T>().await.unwrap())),
                400 => callback(ApiResponse::BadRequest(
                    response.json::<ErrorData>().await.unwrap().errors,
                )),
                _ => unreachable!(),
            };
        });
    }

    pub fn send_without_ok<F>(self, callback: F)
    where
        F: FnOnce(ApiResponse<()>) + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            let response = self.send_async().await;
            match response.status() {
                200 => callback(ApiResponse::Ok(())),
                400 => callback(ApiResponse::BadRequest(
                    response.json::<ErrorData>().await.unwrap().errors,
                )),
                _ => unreachable!(),
            };
        });
    }

    fn create_request(&self) -> Request {
        let endpoint_string = Self::final_endpoint(&self.endpoint);
        let endpoint = endpoint_string.as_str();
        let mut request = match self.kind {
            ApiRequestKind::Get => Request::get(endpoint),
            ApiRequestKind::Post => Request::post(endpoint),
            ApiRequestKind::Put => Request::put(endpoint),
            ApiRequestKind::Delete => Request::delete(endpoint),
        };

        if let Some(query) = &self.query {
            request = request.query(query.iter().map(|x| (x.0.as_str(), &x.1)));
        }
        if let Some(body) = &self.body {
            request = request.body(body);
        }

        request
            .credentials(web_sys::RequestCredentials::Include)
            .header("Access-Control-Allow-Origin", DOMAIN)
            .header("Access-Control-Allow-Credentials", "true")
            .header("Content-Type", "application/json")
    }

    async fn create_request_and_send_read_lock(&self) -> Result<Response, gloo_net::Error> {
        let request = self.create_request();
        let _lock = REQUEST_LOCK.read().await;
        request.send().await
    }

    async fn create_request_and_send_write_lock(&self) -> Result<Response, gloo_net::Error> {
        let request = self.create_request();
        let _lock = REQUEST_LOCK.write().await;
        request.send().await
    }

    async fn refresh_token(&self) {
        let mut wait_time = 0;
        loop {
            if wait_time != 0 {
                threading::sleep(wait_time).await;
            }

            let response = match post("accounts/auth/refreshtoken")
                .body(&RefreshTokenData {
                    refresh_token: *REFRESH_TOKEN.get(),
                })
                .create_request_and_send_write_lock()
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    AppStatusBar::set_connection(false);
                    wait_time = 3000;
                    continue;
                }
            };

            match response.status() {
                200 => {
                    AppStatusBar::set_connection(true);
                    set_refresh_token(
                        response
                            .json::<RefreshTokenData>()
                            .await
                            .unwrap()
                            .refresh_token,
                    );
                    break;
                }
                400 | 401 => {
                    AppStatusBar::set_connection(true);
                    App::logout_without_api();
                    return;
                }
                408 | 500 | 502 | 504 => {
                    AppStatusBar::set_connection(false);
                    wait_time = 1000;
                    continue;
                }
                503 => {
                    AppStatusBar::set_connection(false);
                    wait_time = 5000;
                    continue;
                }
                _ => panic!("Unable to send API request."),
            };
        }
    }
}
