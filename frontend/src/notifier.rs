use std::{cell::RefCell, sync::Arc};

use arc_cell::ArcCell;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use uuid::Uuid;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_sockets::WebSocketError;

use crate::{
    api::{self, ApiResponse},
    common::UnsafeSync,
    direct_messages_views,
    helpers::prelude::*,
};

lazy_static! {
    static ref WEB_SOCKET: ArcCell<Option<WebSocket>> = ArcCell::default();
}

struct WebSocket {
    client: UnsafeSync<Arc<RefCell<wasm_sockets::PollingClient>>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetWsResponseData {
    token: Uuid,
    address: String,
}

pub fn connect() {
    let cb: Closure<dyn FnMut()> = Closure::new(move || {
        if let Some(ws) = WEB_SOCKET.get().as_ref() {
            let borrowed = ws.client.borrow();
            borrowed.send_string(";").unwrap();
        } else {
            reconnect();
        }
    });
    WebPage::window()
        .set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 60000)
        .unwrap();
    cb.forget();

    reconnect();
}

fn reconnect() {
    WEB_SOCKET.set(Arc::new(None));
    api::get("accounts/getws").send(
        move |r: ApiResponse<GetWsResponseData>| match r {
            ApiResponse::Ok(r) => {
                connect_worker(r).unwrap();
            }
            ApiResponse::BadRequest(_) => todo!(),
        },
    );
}

fn connect_worker(data: GetWsResponseData) -> Result<(), WebSocketError> {
    let client = Arc::new(RefCell::new(wasm_sockets::PollingClient::new(&format!(
        "{}/api/v1/notifier/ws?token={}",
        data.address, data.token
    ))?));

    let clone = client.clone();
    client
        .borrow_mut()
        .event_client
        .set_on_connection(Some(Box::new(
            move |_client: &wasm_sockets::EventClient| {
                log::info!("Connection successfully created");
                WEB_SOCKET.set(Arc::new(Some(WebSocket {
                    client: clone.clone().into(),
                })));
            },
        )));

    client
        .borrow_mut()
        .event_client
        .set_on_error(Some(Box::new(|error| {
            log::error!("{:#?}", error);
        })));
    client
        .borrow_mut()
        .event_client
        .set_on_close(Some(Box::new(move |_evt| {
            log::info!("Connection closed");
            reconnect();
        })));
    client
        .borrow_mut()
        .event_client
        .set_on_message(Some(Box::new(
            |_client: &wasm_sockets::EventClient, message: wasm_sockets::Message| {
                wasm_bindgen_futures::spawn_local(async move {
                    process_message(message).await;
                });
            },
        )));
    Ok(())
}

async fn process_message(message: wasm_sockets::Message) {
    let json: serde_json::Map<String, serde_json::Value> = match message {
        wasm_sockets::Message::Text(text) => serde_json::from_str(&text),
        wasm_sockets::Message::Binary(_) => todo!(),
    }
    .unwrap();

    let code = json["code"].as_i64().unwrap();
    let data = json["data"].clone();
    match code {
        // ReceivedDirectMessage.
        0 => direct_messages_views::notifier_process::received_direct_message(
            from_value(data).unwrap(),
        ),
        _ => unimplemented!(),
    }
    .await;
}
