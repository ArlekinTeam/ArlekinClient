use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use gloo_timers::callback::Timeout;
use serde::{Serialize, Deserialize};
use serde_json::json;
use web_sys::FileList;
use yew::prelude::*;

use crate::{
    app::App,
    channel_views::channel_content::ChannelContent,
    direct_messages_views::encryption,
    helpers::prelude::*,
    localization, navigator,
    route::{self, Route}, api,
};

use super::{channel_content, channel_message::ChannelMessage};

pub struct Channel {
    sent_message_id: i64,
}

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub channel_id: i64,
}

pub enum Msg {
    Refresh,
    ChangeChannel,
    Send,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBucketResponseData {
    attachment_id: i64,
    name: String,
    storage_domain: String,
    token: String
}

struct FileSenderBucket {
    message_content: String,
    sent_message_id: i64,
    channel_id: i64,
    counter: usize,
    files: Vec<Option<SendedFile>>
}

struct SendedFile {
    attachment_id: i64,
    name: String,
    key: String,
}

impl Component for Channel {
    type Message = Msg;
    type Properties = Props;

    fn create(_: &Context<Self>) -> Self {
        Self { sent_message_id: 0 }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Refresh => (),
            Msg::ChangeChannel => (),
            Msg::Send => self.send_message(ctx),
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let lang = localization::get_language();
        html! {
            <div class="channel-container">
                <route::Router route={Route::Direct { id: ctx.props().channel_id }} />

                <div>
                    <h2>{"Channel name"}</h2>
                </div>

                <ChannelContent channel_id={ctx.props().channel_id} />

                <div class="channel-send-button-container">
                    <input type="file" id="message-file" multiple=true />
                    <input type="text" name="message" id="message" />
                    <button onclick={ctx.link().callback(|_| Msg::Send)}>{lang.get("viewChannelSendMessage")}</button>
                </div>
            </div>
        }
    }
}

impl Channel {
    fn send_message(&mut self, ctx: &Context<Self>) {
        let input = Input::by_id("message");
        let message_content = input.value();

        let channel_id = ctx.props().channel_id;
        self.sent_message_id -= 1;
        let sent_message_id = self.sent_message_id;

        let file_input = Input::by_id("message-file");
        let files = file_input.files().unwrap();
        if files.length() > 0 {
            Self::send_files(message_content, sent_message_id, channel_id, files);
            file_input.set_value("");
        } else {
            Self::send_message_worker(message_content, sent_message_id, channel_id);
        }

        input.set_value("");
    }

    fn send_message_worker(message_content: String, sent_message_id: i64, channel_id: i64) {
        channel_content::notify_message(
            channel_id,
            ChannelMessage::new(
                sent_message_id,
                App::user_id(),
                Ok(Arc::new(message_content.clone())),
            ),
        );

        Timeout::new(0, move || {
            wasm_bindgen_futures::spawn_local(async move {
                let message_id = encryption::send_message(channel_id, message_content.clone()).await;

                channel_content::edit_message(
                    channel_id,
                    sent_message_id,
                    ChannelMessage::new(
                        message_id,
                        App::user_id(),
                        Ok(Arc::new(message_content)),
                    ),
                );

                navigator::add_pings(channel_id, i64::MIN, message_id);
            });
        })
        .forget();

        channel_content::set_scroll(channel_id, 0);
    }

    fn send_files(message_content: String, sent_message_id: i64, channel_id: i64, files: FileList) {
        let sender_bucket = Arc::new(async_std::sync::Mutex::new(
            FileSenderBucket {
                message_content,
                sent_message_id,
                channel_id,
                counter: files.length() as usize,
                files: (0..files.length()).map(|_| None).collect()
            }
        ));

        for i in 0..files.length() {
            let file = files.item(i).unwrap();
            let bucket = sender_bucket.clone();

            Timeout::new(0, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    Self::send_file_worker(i, &file, bucket).await;
                });
            }).forget();
        }
    }

    async fn send_file_worker(
        index: u32, file: &web_sys::File, sender_bucket: Arc<async_std::sync::Mutex<FileSenderBucket>>
    ) {
        let aes = encryption::generate_aes().await;
        let mut nonce: [u8; 16] = Default::default();
        WebPage::crypto()
            .get_random_values_with_u8_array(&mut nonce)
            .unwrap();

        let mut body = File::to_bytes_without_exif(file).await;

        let mut response = api::put("attachments/direct/bucket")
            .body(&json!({
                "size": body.len() as i64,
                "name":  file.name(),
                "alternateTextNonce": general_purpose::STANDARD.encode(nonce),
                "encryptedAlternateText": general_purpose::STANDARD.encode("hi")
            }))
            .send_async()
            .await;
        let bucket = match response.status() {
            200 => response.json::<CreateBucketResponseData>().await.unwrap(),
            400 => todo!(),
            _ => unreachable!(),
        };

        WebPage::crypto()
            .get_random_values_with_u8_array(&mut nonce)
            .unwrap();

        encryption::encrypt_aes(&aes, &nonce, &mut body).await;

        response = api::put_with_own(&format!("{}/attachments", bucket.storage_domain))
            .query([("token", bucket.token)])
            .body_raw(body)
            .send_async()
            .await;
        match response.status() {
            200 => (),
            400 => todo!(),
            _ => unreachable!(),
        };

        let mut vec = Vec::new();
        vec.extend_from_slice(&nonce);
        vec.extend_from_slice(&encryption::export_key(&aes, "raw").await);

        let mut lock = sender_bucket.lock().await;
        lock.files[index as usize] = Some(SendedFile {
            attachment_id: bucket.attachment_id,
            name: bucket.name,
            key: general_purpose::URL_SAFE.encode(vec)
        });

        lock.decrement_counter();
    }
}

impl FileSenderBucket {
    fn decrement_counter(&mut self) {
        self.counter -= 1;
        if self.counter > 0 {
            return;
        }

        let mut message_content = self.message_content.clone();
        for file in self.files.iter() {
            let file = file.as_ref().unwrap();
            message_content.push_str(&format!("<^a/{}/{}/{}>", file.attachment_id, file.name, file.key));
        }

        Channel::send_message_worker(message_content, self.sent_message_id, self.channel_id);
    }
}
