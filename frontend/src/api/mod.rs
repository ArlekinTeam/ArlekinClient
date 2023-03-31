use std::{sync::Arc, collections::HashMap};
use arc_cell::ArcCell;
use gloo_net::http::{Request, Response};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_repr::{Serialize_repr, Deserialize_repr};
use uuid::Uuid;
use yew::prelude::*;

use crate::app;

static API_ENDPOINT: &str = "http://localhost:9080/api/v1/";
static REFRESH_TOKEN: Lazy<ArcCell<Uuid>> = Lazy::new(ArcCell::default);

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum Platform {
    Native = 0
}

pub struct ApiRequest {
    inner: Request
}

pub enum ApiResponse<T> {
    Ok(T),
    BadRequest(HashMap<String, ErrorDataElement>)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDataElement {
    pub code: u32,
    pub translation_key: String
}

#[derive(Serialize, Deserialize)]
struct ErrorData {
    errors: HashMap<String, ErrorDataElement>
}

#[derive(Serialize, Deserialize, Default)]
struct UnauthorizedData {
    is_expired: bool
}

#[derive(Serialize, Deserialize, Default)]
struct RefreshTokenData {
    refresh_token: Uuid
}

pub fn get(endpoint: &str) -> ApiRequest {
    ApiRequest { inner: Request::get(final_endpoint(endpoint).as_str()) }
}

pub fn post(endpoint: &str) -> ApiRequest {
    ApiRequest { inner: Request::post(final_endpoint(endpoint).as_str()) }
}

pub fn put(endpoint: &str) -> ApiRequest {
    ApiRequest { inner: Request::put(final_endpoint(endpoint).as_str()) }
}

pub fn delete(endpoint: &str) -> ApiRequest {
    ApiRequest { inner: Request::delete(final_endpoint(endpoint).as_str()) }
}

fn final_endpoint(endpoint: &str) -> String {
    format!("{API_ENDPOINT}{endpoint}")
}

impl ApiRequest {
    pub fn query<'a, T, V>(self, params: T) -> Self
    where
        T: IntoIterator<Item = (&'a str, V)>,
        V: AsRef<str>,
    {
        ApiRequest {
            inner: self.inner.query(params)
        }
    }

    pub fn body<T: serde::ser::Serialize + ?Sized>(self, value: &T) -> Self {
        ApiRequest {
            inner: self.inner.header("Content-Type", "application/json")
                .body(serde_json::to_string(value).unwrap())
        }
    }

    pub async fn send_async(self) -> Response {
        let response = self.inner
            //.credentials(web_sys::RequestCredentials::Include)
            //.header("Access-Control-Allow-Origin", "*")
            //.header("Access-Control-Allow-Credentials", "true")
            .header("Content-Type", "application/json")
            .send().await.expect("Unable to send API request.");

        match response.status() {
            200 | 400 => response,
            401 => match response.json::<UnauthorizedData>().await {
                Ok(data) => match data.is_expired {
                    true => {
                        REFRESH_TOKEN.set(Arc::new(post("accounts/auth/refreshtoken")
                            .body(&RefreshTokenData { refresh_token: *REFRESH_TOKEN.get() }).inner
                            .send().await.expect("Unable to send refresh token.")
                            .json::<RefreshTokenData>().await
                            .expect("Unable to deserialize refresh token.")
                            .refresh_token
                        ));

                        panic!();
                    },
                    false => panic!("logout"),
                },
                Err(_) => unreachable!()
            },
            _ => panic!("Unable to send API request.")
        }
    }

    pub fn send<F, T>(self, _app_callback: Callback<app::Msg>, callback: F)
        where
            F: FnOnce(ApiResponse<T>) + 'static,
            T: DeserializeOwned
    {
        wasm_bindgen_futures::spawn_local(async move {
            let response = self.send_async().await;
            match response.status() {
                200 => callback(ApiResponse::Ok(response.json::<T>().await.unwrap())),
                400 => callback(ApiResponse::BadRequest(response.json::<ErrorData>().await.unwrap().errors)),
                _ => unreachable!()
            };
        });
    }

    pub fn send_without_ok<F>(self, _app_callback: Callback<app::Msg>, callback: F)
        where
            F: FnOnce(ApiResponse<()>) + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            let response = self.send_async().await;
            match response.status() {
                200 => callback(ApiResponse::Ok(())),
                400 => callback(ApiResponse::BadRequest(response.json::<ErrorData>().await.unwrap().errors)),
                _ => unreachable!()
            };
        });
    }
}
