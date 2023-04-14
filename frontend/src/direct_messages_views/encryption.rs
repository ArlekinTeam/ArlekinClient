use core::panic;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use arc_cell::ArcCell;
use base64::{engine::general_purpose, Engine as _};
use js_sys::Reflect;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, CryptoKeyPair};

use crate::{
    api::{self, ErrorData, ErrorDataElement, Platform},
    channel_views::{channel_content::ChannelMessage, channel_message_error::ChannelMessageError},
    common::UnsafeSync,
    helpers::prelude::WebPage,
};

use super::encryption_error::EncryptionError;

const RSA_BITS: usize = 4096;
const AES_BITS: usize = 256;
const AES_BLOCK_BITS: usize = 64;
const PRIVATE_KEY_BLOCKS: usize = 8;

lazy_static! {
    static ref ENCRYPTION_BLOCK_DATA: ArcCell<Option<UnsafeSync<PrivateKeyEncryptionData>>> =
        ArcCell::default();
    static ref USED_ENCRYPTION_KEYS: Mutex<LruCache<i64, i64>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(512).unwrap()));
    static ref CACHED_ENCRYPTION_BLOCKS_PRIVATE: Mutex<LruCache<i64, UnsafeSync<Arc<CryptoKey>>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()));
    static ref CACHED_ENCRYPTION_KEYS: Mutex<LruCache<(i64, i64), Arc<EncryptionKey>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(512).unwrap()));
}

struct PrivateKeyEncryptionData {
    keys: [CryptoKey; PRIVATE_KEY_BLOCKS],
}

struct EncryptionKey {
    encryption_key_id: i64,
    #[allow(dead_code)]
    encryption_block_id: i64,
    key: UnsafeSync<CryptoKey>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutEncryptionBlockResponseData {
    encryption_block_id: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionPublicKeyElementResponseData {
    platform: Platform,
    encryption_block_id: i64,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionPublicKeyResponseData {
    public_keys: Vec<EncryptionPublicKeyElementResponseData>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionPrivateKeyResponseData {
    nonce: String,
    encrypted_private_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysPutDataElement {
    encryption_block_id: i64,
    encrypted_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysPutResponseData {
    encryption_block_id: i64,
    encryption_key_id: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysGetEncryptedKeyResponseData {
    encryption_block_id: i64,
    encryption_key_id: i64,
    encrypted_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesPutResponseData {
    direct_message_id: i64,
    send_new_encryption_key: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetMiddleKeysResponseData {
    keys: String,
    encrypted_keys: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesGetElementResultData {
    direct_message_id: i64,
    author_user_id: i64,
    encryption_key_id: i64,
    nonce: String,
    encrypted_text: String,
    edited: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesGetResultData {
    messages: Vec<MessagesGetElementResultData>,
}

pub fn try_load() -> bool {
    let encryption_block_hash = WebPage::local_storage()
        .get_item("encryption_block_hash")
        .expect("Unable to get encryption_block_hash from session storage.");
    if let Some(encryption_block_hash) = encryption_block_hash {
        let encryption_block_hash = general_purpose::STANDARD
            .decode(encryption_block_hash)
            .unwrap();

        wasm_bindgen_futures::spawn_local(async move {
            init_worker(&encryption_block_hash).await;
        });

        true
    } else {
        false
    }
}

pub async fn init(encryption_block_hash: &[u8]) {
    WebPage::local_storage()
        .set_item(
            "encryption_block_hash",
            &general_purpose::STANDARD.encode(encryption_block_hash),
        )
        .expect("Unable to set encryption_block_hash in session storage.");

    init_worker(encryption_block_hash).await;
}

pub async fn init_worker(encryption_block_hash: &[u8]) {
    let response = api::post("channels/direct/encryption/getmiddlekeys")
        .send_async()
        .await;
    let data = match response.status() {
        200 => response.json::<GetMiddleKeysResponseData>().await.unwrap(),
        400 => {
            let errors = response.json::<ErrorData>().await.unwrap().errors;
            if
            // DirectChannelEncryptionMiddleKeysNotFound
            errors.len() == 1
                && errors.get("").unwrap_or(&ErrorDataElement::default()).code == 3004
            {
                put_middle_keys(encryption_block_hash).await
            } else {
                todo!();
            }
        }
        _ => unreachable!(),
    };

    let keys_buffer = general_purpose::STANDARD.decode(data.keys).unwrap();
    let mut keys = keys_buffer.chunks(keys_buffer.len() / PRIVATE_KEY_BLOCKS);
    let encrypted_keys_buffer = general_purpose::STANDARD
        .decode(data.encrypted_keys)
        .unwrap();
    let mut encrypted_keys =
        encrypted_keys_buffer.chunks(encrypted_keys_buffer.len() / PRIVATE_KEY_BLOCKS);
    let chunks = encryption_block_hash.chunks(encryption_block_hash.len() / PRIVATE_KEY_BLOCKS);

    let mut vec = Vec::new();
    for chunk in chunks {
        let key = import_aes(keys.next().unwrap()).await;
        let mut encrypted_key_raw = encrypted_keys.next().unwrap().to_vec();
        decrypt_aes(&key, chunk, &mut encrypted_key_raw).await;

        vec.push(import_aes(&encrypted_key_raw).await);
    }

    ENCRYPTION_BLOCK_DATA.set(Arc::new(Some(UnsafeSync(PrivateKeyEncryptionData {
        keys: vec.try_into().unwrap(),
    }))));
}

pub async fn put_new_encryption_block(direct_channel_id: i64) {
    let (public_key, private_key) = generate_rsa().await;
    let (encrypted_private_key, nonce) = encrypt_rsa_private_key(&private_key).await;
    let public_key = export_key(&public_key, "spki").await;

    let response = api::put("channels/direct/encryption")
        .body(&json!({
            "directChannelId": direct_channel_id,
            "publicKey":  general_purpose::STANDARD.encode(public_key),
            "nonce": general_purpose::STANDARD.encode(nonce),
            "encryptedPrivateKey": general_purpose::STANDARD.encode(encrypted_private_key)
        }))
        .send_async()
        .await;
    match response.status() {
        200 => {
            let r = response
                .json::<PutEncryptionBlockResponseData>()
                .await
                .unwrap();

            CACHED_ENCRYPTION_BLOCKS_PRIVATE
                .lock()
                .unwrap()
                .put(r.encryption_block_id, Arc::new(private_key).into());
        }
        400 => {
            let errors = response.json::<ErrorData>().await.unwrap().errors;
            if
            // ToFast
            errors.len() == 1
                && errors
                    .get("directChannelId")
                    .unwrap_or(&ErrorDataElement::default())
                    .code
                    == 4002
            {
                return;
            }

            todo!();
        }
        _ => unreachable!(),
    };
}

pub async fn put_new_encryption_key(direct_channel_id: i64) {
    put_new_encryption_block(direct_channel_id).await;

    let response = api::post("channels/direct/encryption/getpublickeys")
        .body(&json!({ "directChannelId": direct_channel_id }))
        .send_async()
        .await;
    match response.status() {
        200 => {
            let r = response
                .json::<EncryptionPublicKeyResponseData>()
                .await
                .unwrap();
            put_new_encryption_key_worker(direct_channel_id, r.public_keys).await;
        }
        400 => {
            todo!();
        }
        _ => unreachable!(),
    };
}

pub async fn decrypt_message(
    direct_channel_id: i64,
    direct_message_id: i64,
    author_user_id: i64,
    encryption_key_id: i64,
    nonce: String,
    encrypted_text: String,
) -> ChannelMessage {
    let key = match get_encryption_key(direct_channel_id, encryption_key_id).await {
        Ok(key) => key,
        Err(err) => {
            return ChannelMessage {
                message_id: direct_message_id,
                author_user_id,
                text: Err(ChannelMessageError::Encryption(err)),
            }
        }
    };

    let nonce = general_purpose::STANDARD.decode(nonce).unwrap();
    let mut text = general_purpose::STANDARD.decode(encrypted_text).unwrap();

    decrypt_aes(&key.key, &nonce, &mut text).await;

    ChannelMessage {
        message_id: direct_message_id,
        author_user_id,
        text: Ok(Arc::new(String::from_utf8(text).unwrap())),
    }
}

pub async fn get_messages(
    direct_channel_id: i64,
    before_direct_message_id: i64,
) -> Vec<ChannelMessage> {
    let response = api::get("channels/direct/messages")
        .query([
            ("directChannelId", direct_channel_id.to_string()),
            (
                "beforeDirectMessageId",
                before_direct_message_id.to_string(),
            ),
        ])
        .send_async()
        .await;
    let messages = match response.status() {
        200 => response.json::<MessagesGetResultData>().await.unwrap(),
        400 => todo!(),
        _ => unreachable!(),
    }
    .messages;

    let mut result = Vec::with_capacity(messages.len());
    for message in messages {
        result.push(
            decrypt_message(
                direct_channel_id,
                message.direct_message_id,
                message.author_user_id,
                message.encryption_key_id,
                message.nonce,
                message.encrypted_text,
            )
            .await,
        );
    }

    result
}

pub async fn send_message(direct_channel_id: i64, content: String) -> i64 {
    // Zero for newest.
    let key = get_encryption_key(direct_channel_id, 0).await.unwrap();

    let mut nonce: [u8; 16] = Default::default();
    WebPage::crypto()
        .get_random_values_with_u8_array(&mut nonce)
        .unwrap();

    let mut buffer = content.as_bytes().to_vec();
    encrypt_aes(&key.key, &nonce, &mut buffer).await;

    let response = api::put("channels/direct/messages")
        .body(&json!({
            "directChannelId": direct_channel_id,
            "encryptionKeyId": key.encryption_key_id,
            "nonce": general_purpose::STANDARD.encode(nonce),
            "encryptedText": general_purpose::STANDARD.encode(buffer)
        }))
        .send_async()
        .await;
    match response.status() {
        200 => {
            let r = response.json::<MessagesPutResponseData>().await.unwrap();

            r.direct_message_id
        }
        400 => {
            todo!();
        }
        _ => unreachable!(),
    }
}

async fn put_middle_keys(encryption_block_hash: &[u8]) -> GetMiddleKeysResponseData {
    let mut keys = Vec::new();
    let mut encrypted_keys = Vec::new();

    for chunk in encryption_block_hash.chunks(encryption_block_hash.len() / PRIVATE_KEY_BLOCKS) {
        let key = generate_aes().await;
        let mut encrypted_key = export_key(&generate_aes().await, "raw").await;
        encrypt_aes(&key, chunk, &mut encrypted_key).await;

        keys.extend_from_slice(&export_key(&key, "raw").await);
        encrypted_keys.extend_from_slice(&encrypted_key);
    }

    let data = GetMiddleKeysResponseData {
        keys: general_purpose::STANDARD.encode(keys),
        encrypted_keys: general_purpose::STANDARD.encode(encrypted_keys),
    };

    let response = api::put("channels/direct/encryption/putmiddlekeys")
        .body(&data)
        .send_async()
        .await;
    match response.status() {
        200 => data,
        400 => todo!(),
        _ => unreachable!(),
    }
}

async fn get_encryption_key(
    direct_channel_id: i64,
    encryption_key_id: i64,
) -> Result<Arc<EncryptionKey>, EncryptionError> {
    loop {
        if let Some(key) = CACHED_ENCRYPTION_KEYS
            .lock()
            .unwrap()
            .get(&(direct_channel_id, encryption_key_id))
        {
            return Ok(key.clone());
        }

        let response = api::post("channels/direct/encryption/keys/getencryptedkey")
            .body(&json!({
                "directChannelId": direct_channel_id,
                "encryptionKeyId": encryption_key_id
            }))
            .send_async()
            .await;
        match response.status() {
            200 => {
                let r = response
                    .json::<EncryptionKeysGetEncryptedKeyResponseData>()
                    .await
                    .unwrap();

                let private_key = get_private_key(direct_channel_id, r.encryption_block_id).await;
                let mut buffer = general_purpose::STANDARD.decode(r.encrypted_key).unwrap();
                buffer = decrypt_rsa(&private_key, &mut buffer).await;
                let key = import_aes(&buffer).await;

                CACHED_ENCRYPTION_KEYS.lock().unwrap().put(
                    (direct_channel_id, encryption_key_id),
                    Arc::new(EncryptionKey {
                        encryption_key_id: r.encryption_key_id,
                        encryption_block_id: r.encryption_block_id,
                        key: key.into(),
                    }),
                );
                USED_ENCRYPTION_KEYS
                    .lock()
                    .unwrap()
                    .put(direct_channel_id, encryption_key_id);
            }
            400 => {
                let errors = response.json::<ErrorData>().await.unwrap().errors;
                if
                // DirectChannelEncryptionKeyNotFound
                errors.len() == 1
                    && errors
                        .get("encryptionKeyId")
                        .unwrap_or(&ErrorDataElement::default())
                        .code
                        == 3003
                {
                    // Ignore only when encryption key is zero.
                    if encryption_key_id == 0 {
                        put_new_encryption_key(direct_channel_id).await;
                        continue;
                    }

                    return Err(EncryptionError::UnableToRead);
                }

                todo!();
            }
            _ => unreachable!(),
        };
    }
}

async fn get_private_key(direct_channel_id: i64, encryption_block_id: i64) -> Arc<CryptoKey> {
    loop {
        if let Some(key) = CACHED_ENCRYPTION_BLOCKS_PRIVATE
            .lock()
            .unwrap()
            .get(&encryption_block_id)
        {
            return key.0.clone();
        }

        let response = api::post("channels/direct/encryption/getprivatekey")
            .body(&json!({
                "directChannelId": direct_channel_id,
                "encryptionBlockId": encryption_block_id
            }))
            .send_async()
            .await;
        match response.status() {
            200 => {
                let r = response
                    .json::<EncryptionPrivateKeyResponseData>()
                    .await
                    .unwrap();

                get_private_key_worker(encryption_block_id, r.nonce, r.encrypted_private_key).await;
            }
            400 => {
                todo!();
            }
            _ => unreachable!(),
        };
    }
}

async fn get_private_key_worker(
    encryption_block_id: i64,
    nonce: String,
    encrypted_private_key: String,
) {
    let encryption_block = ENCRYPTION_BLOCK_DATA.get();
    let encryption = match encryption_block.as_ref() {
        Some(e) => e,
        None => panic!("Encryption is not initialized."),
    };

    let raw_nonce = general_purpose::STANDARD.decode(nonce).unwrap();
    let mut buffer = general_purpose::STANDARD
        .decode(encrypted_private_key)
        .unwrap();
    let mut parts: [Vec<u8>; PRIVATE_KEY_BLOCKS] = Default::default();

    let mut length_buffer: [u8; 4] = Default::default();
    length_buffer.copy_from_slice(&buffer[0..4]);
    let length = u32::from_le_bytes(length_buffer) as usize;

    let part_length = (buffer.len() - 4) / PRIVATE_KEY_BLOCKS;
    let keys = &encryption.keys;

    for i in 0..PRIVATE_KEY_BLOCKS {
        let slice = &mut buffer[(4 + i * part_length)..(4 + (i + 1) * part_length)];
        decrypt_aes(&keys[i], &raw_nonce[(i * 16)..((i + 1) * 16)], slice).await;
        parts[i].extend_from_slice(slice);
    }

    buffer.clear();
    for i in 0..(part_length * PRIVATE_KEY_BLOCKS) {
        buffer.push(parts[i % PRIVATE_KEY_BLOCKS][i / PRIVATE_KEY_BLOCKS]);
    }

    let private_key = import_rsa(&buffer[0..length], "pkcs8", "decrypt").await;
    CACHED_ENCRYPTION_BLOCKS_PRIVATE
        .lock()
        .unwrap()
        .put(encryption_block_id, Arc::new(private_key).into());
}

async fn put_new_encryption_key_worker(
    direct_channel_id: i64,
    public_keys: Vec<EncryptionPublicKeyElementResponseData>,
) {
    let key = generate_aes().await;
    let mut key_raw = export_key(&key, "raw").await;
    let mut elements = Vec::new();

    for element in public_keys {
        let imported = import_rsa(
            &general_purpose::STANDARD
                .decode(&element.public_key)
                .unwrap(),
            "spki",
            "encrypt",
        )
        .await;
        let buffer = encrypt_rsa(&imported, &mut key_raw).await;

        elements.push(EncryptionKeysPutDataElement {
            encryption_block_id: element.encryption_block_id,
            encrypted_key: general_purpose::STANDARD.encode(&buffer),
        });
    }

    let response = api::put("channels/direct/encryption/keys")
        .body(&json!({
            "directChannelId": direct_channel_id,
            "keyData": elements
        }))
        .send_async()
        .await;
    match response.status() {
        200 => {
            let r = response
                .json::<EncryptionKeysPutResponseData>()
                .await
                .unwrap();

            CACHED_ENCRYPTION_KEYS.lock().unwrap().put(
                (direct_channel_id, r.encryption_key_id),
                Arc::new(EncryptionKey {
                    encryption_key_id: r.encryption_block_id,
                    encryption_block_id: r.encryption_key_id,
                    key: UnsafeSync(key),
                }),
            );
            USED_ENCRYPTION_KEYS
                .lock()
                .unwrap()
                .put(direct_channel_id, r.encryption_key_id);
        }
        400 => {
            let errors = response.json::<ErrorData>().await.unwrap().errors;
            if
            // ToFast
            errors.len() == 1
                && errors
                    .get("directChannelId")
                    .unwrap_or(&ErrorDataElement::default())
                    .code
                    == 4002
            {
                return;
            }

            todo!();
        }
        _ => unreachable!(),
    };
}

async fn export_key(key: &CryptoKey, format: &str) -> Vec<u8> {
    let promise = WebPage::crypto()
        .subtle()
        .export_key(format, key)
        .expect("Unable to export key.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.to_vec()
}

async fn generate_rsa() -> (CryptoKey, CryptoKey) {
    let algorithm = js_sys::Object::new();
    let public_exponent = js_sys::Uint8Array::new_with_length(3);
    public_exponent.copy_from(&[1, 0, 1]);

    Reflect::set(&algorithm, &"publicExponent".into(), &public_exponent).unwrap();
    Reflect::set(&algorithm, &"name".into(), &"RSA-OAEP".into()).unwrap();
    Reflect::set(&algorithm, &"modulusLength".into(), &RSA_BITS.into()).unwrap();
    Reflect::set(&algorithm, &"hash".into(), &"SHA-256".into()).unwrap();

    let key_usages = js_sys::Array::new_with_length(2);
    key_usages.set(0, "encrypt".into());
    key_usages.set(1, "decrypt".into());

    let key_promise = WebPage::crypto()
        .subtle()
        .generate_key_with_object(&algorithm, true, &key_usages)
        .expect("Unable to generate RSA keys.");
    let key_pair: CryptoKeyPair = JsFuture::from(key_promise).await.unwrap().into();

    let public_key: CryptoKey = Reflect::get(&key_pair, &JsValue::from("publicKey"))
        .expect("Unable to get public key.")
        .into();
    let private_key: CryptoKey = Reflect::get(&key_pair, &JsValue::from("privateKey"))
        .expect("Unable to get private key.")
        .into();

    (public_key, private_key)
}

async fn import_rsa(raw_key: &[u8], format: &str, usage: &str) -> CryptoKey {
    let algorithm = js_sys::Object::new();
    let public_exponent = js_sys::Uint8Array::new_with_length(3);
    public_exponent.copy_from(&[1, 0, 1]);

    Reflect::set(&algorithm, &"publicExponent".into(), &public_exponent).unwrap();
    Reflect::set(&algorithm, &"name".into(), &"RSA-OAEP".into()).unwrap();
    Reflect::set(&algorithm, &"modulusLength".into(), &RSA_BITS.into()).unwrap();
    Reflect::set(&algorithm, &"hash".into(), &"SHA-256".into()).unwrap();

    let key_usages = js_sys::Array::new_with_length(1);
    key_usages.set(0, usage.into());

    let key_data = js_sys::Uint8Array::new_with_length(raw_key.len() as u32);
    key_data.copy_from(raw_key);

    let key_promise = WebPage::crypto()
        .subtle()
        .import_key_with_object(format, &key_data, &algorithm, true, &key_usages)
        .unwrap();
    JsFuture::from(key_promise).await.unwrap().into()
}

async fn encrypt_rsa(key: &CryptoKey, data: &mut [u8]) -> Vec<u8> {
    let promise = WebPage::crypto()
        .subtle()
        .encrypt_with_str_and_u8_array("RSA-OAEP", key, data)
        .expect("Unable to encrypt RSA data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.to_vec()
}

async fn decrypt_rsa(key: &CryptoKey, data: &mut [u8]) -> Vec<u8> {
    let promise = WebPage::crypto()
        .subtle()
        .decrypt_with_str_and_u8_array("RSA-OAEP", key, data)
        .expect("Unable to decrypt RSA data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.to_vec()
}

async fn generate_aes() -> CryptoKey {
    let algorithm = js_sys::Object::new();
    Reflect::set(&algorithm, &"name".into(), &"AES-CTR".into()).unwrap();
    Reflect::set(&algorithm, &"length".into(), &AES_BITS.into()).unwrap();

    let key_usages = js_sys::Array::new_with_length(2);
    key_usages.set(0, "encrypt".into());
    key_usages.set(1, "decrypt".into());

    let key_promise = WebPage::crypto()
        .subtle()
        .generate_key_with_object(&algorithm, true, &key_usages)
        .expect("Unable to generate AES key.");
    JsFuture::from(key_promise).await.unwrap().into()
}

async fn import_aes(raw_key: &[u8]) -> CryptoKey {
    let algorithm = js_sys::Object::new();
    Reflect::set(&algorithm, &"name".into(), &"AES-CTR".into()).unwrap();
    Reflect::set(&algorithm, &"length".into(), &AES_BLOCK_BITS.into()).unwrap();

    let key_usages = js_sys::Array::new_with_length(2);
    key_usages.set(0, "encrypt".into());
    key_usages.set(1, "decrypt".into());

    let key_data = js_sys::Uint8Array::new_with_length(raw_key.len() as u32);
    key_data.copy_from(raw_key);

    let key_promise = WebPage::crypto()
        .subtle()
        .import_key_with_object("raw", &key_data, &algorithm, true, &key_usages)
        .unwrap();
    JsFuture::from(key_promise).await.unwrap().into()
}

async fn encrypt_aes(key: &CryptoKey, nonce: &[u8], data: &mut [u8]) {
    let algorithm = encryption_aes_algorithm(nonce);
    let promise = WebPage::crypto()
        .subtle()
        .encrypt_with_object_and_u8_array(&algorithm, key, data)
        .expect("Unable to encrypt AES data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.copy_to(data);
}

async fn decrypt_aes(key: &CryptoKey, nonce: &[u8], data: &mut [u8]) {
    let algorithm = encryption_aes_algorithm(nonce);
    let promise = WebPage::crypto()
        .subtle()
        .decrypt_with_object_and_u8_array(&algorithm, key, data)
        .expect("Unable to decrypt AES data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.copy_to(data);
}

async fn encrypt_rsa_private_key(
    private_key: &CryptoKey,
) -> (Vec<u8>, [u8; 16 * PRIVATE_KEY_BLOCKS]) {
    let mut private_key_raw = export_key(private_key, "pkcs8").await;
    let length = private_key_raw.len();
    while private_key_raw.len() % PRIVATE_KEY_BLOCKS != 0 {
        private_key_raw.push(0);
    }

    let mut private_key_raw_parts: [Vec<u8>; PRIVATE_KEY_BLOCKS] = Default::default();
    for i in 0..private_key_raw.len() {
        let part = &mut private_key_raw_parts[i % PRIVATE_KEY_BLOCKS];
        part.push(private_key_raw[i]);
    }

    let encryption_block = ENCRYPTION_BLOCK_DATA.get();
    let encryption = match encryption_block.as_ref() {
        Some(e) => e,
        None => panic!("Encryption is not initialized."),
    };

    let mut nonce: [u8; 16 * PRIVATE_KEY_BLOCKS] = [0; 16 * PRIVATE_KEY_BLOCKS];
    WebPage::crypto()
        .get_random_values_with_u8_array(&mut nonce)
        .unwrap();

    for i in 0..PRIVATE_KEY_BLOCKS {
        encrypt_aes(
            &encryption.keys[i],
            &nonce[(i * 16)..((i + 1) * 16)],
            &mut private_key_raw_parts[i],
        )
        .await;
    }

    private_key_raw.clear();
    private_key_raw.extend_from_slice(&(length as u32).to_le_bytes());
    for part in &mut private_key_raw_parts {
        private_key_raw.append(part);
    }

    (private_key_raw, nonce)
}

fn encryption_aes_algorithm(nonce: &[u8]) -> js_sys::Object {
    let algorithm = js_sys::Object::new();
    let nonce_buffer = js_sys::Uint8Array::new_with_length(16);
    nonce_buffer.copy_from(nonce);

    Reflect::set(&algorithm, &"counter".into(), &nonce_buffer).unwrap();
    Reflect::set(&algorithm, &"name".into(), &"AES-CTR".into()).unwrap();
    Reflect::set(&algorithm, &"length".into(), &AES_BLOCK_BITS.into()).unwrap();

    algorithm
}
