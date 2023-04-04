use std::{sync::{Arc, Mutex}, num::NonZeroUsize};

use arc_cell::ArcCell;
use js_sys::Reflect;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKeyPair, CryptoKey};
use yew::Callback;
use base64::{engine::general_purpose, Engine as _};

use crate::{helpers::prelude::WebPage, api::{self, ApiResponse, Platform, ErrorData, ErrorDataElement}, app, common::UnsafeSync};

const RSA_BITS: usize = 4096;
const AES_BITS: usize = 256;
const AES_BLOCK_BITS: usize = 64;

lazy_static! {
    static ref ENCRYPTION_BLOCK_DATA: ArcCell<Option<UnsafeSync<PrivateKeyEncryptionData>>> = ArcCell::default();

    static ref USED_ENCRYPTION_KEYS: Mutex<LruCache<i64, i64>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(512).unwrap()));

    static ref CACHED_ENCRYPTION_BLOCKS_PRIVATE: Mutex<LruCache<i64, UnsafeSync<Arc<CryptoKey>>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()));
    static ref CACHED_ENCRYPTION_KEYS: Mutex<LruCache<i64, Arc<EncryptionKey>>> =
        Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()));
}

struct PrivateKeyEncryptionData {
    c0: CryptoKey,
    c1: CryptoKey,
    c2: CryptoKey,
    c3: CryptoKey
}

struct EncryptionKey {
    encryption_key_id: i64,
    #[allow(dead_code)]
    encryption_block_id: i64,
    key: UnsafeSync<CryptoKey>
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
    public_key: String
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionPublicKeyResponseData {
    public_keys: Vec<EncryptionPublicKeyElementResponseData>
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionPrivateKeyResponseData {
    encrypted_private_key: String
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysPutDataElement {
    encryption_block_id: i64,
    encrypted_key: String
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysPutResponseData {
    encryption_block_id: i64,
    encryption_key_id: i64
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionKeysGetEncryptedKeyResponseData {
    encryption_block_id: i64,
    encryption_key_id: i64,
    encrypted_key: String
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessagesPutResponseData {
    direct_message_id: i64,
    send_new_encryption_key: bool
}

pub async fn init(encryption_block_hash: &[u8]) {
    let mut chunks = encryption_block_hash.chunks(AES_BITS / 8);
    ENCRYPTION_BLOCK_DATA.set(Arc::new(Some(UnsafeSync(PrivateKeyEncryptionData {
        c0: import_aes(chunks.next().unwrap()).await,
        c1: import_aes(chunks.next().unwrap()).await,
        c2: import_aes(chunks.next().unwrap()).await,
        c3: import_aes(chunks.next().unwrap()).await,
    }))));
}

pub async fn put_new_encryption_block(_app_callback: Callback<app::Msg>, direct_channel_id: i64) {
    let (public_key, private_key) = generate_rsa().await;
    let encrypted_private_key = encrypt_rsa_private_key(&private_key, direct_channel_id).await;
    let public_key = export_key(&public_key, "spki").await;

    log::info!("smutne3 {}", general_purpose::STANDARD.encode(&public_key));

    let response = api::put("channels/direct/encryption")
    .body(&json!({
        "directChannelId": direct_channel_id,
        "publicKey":  general_purpose::STANDARD.encode(public_key),
        "encryptedPrivateKey": general_purpose::STANDARD.encode(encrypted_private_key)
    })).send_async().await;
    match response.status() {
        200 => {
            let r =
                response.json::<PutEncryptionBlockResponseData>().await.unwrap();

            CACHED_ENCRYPTION_BLOCKS_PRIVATE.lock().unwrap().put(
                r.encryption_block_id, Arc::new(private_key).into()
            );
        },
        400 => {
            let errors = response.json::<ErrorData>().await.unwrap().errors;
            if // ToFast
                errors.len() == 1 &&
                errors.get("directChannelId").unwrap_or(&ErrorDataElement::default()).code == 4002
            {
                return;
            }

            todo!();
        },
        _ => unreachable!(),
    };
}

pub async fn put_new_encryption_key(app_callback: Callback<app::Msg>, direct_channel_id: i64) {
    put_new_encryption_block(app_callback.clone(), direct_channel_id).await;

    let response = api::post("channels/direct/encryption/getpublickeys")
    .body(&json!({
        "directChannelId": direct_channel_id
    })).send_async().await;
    match response.status() {
        200 => {
            let r =
                response.json::<EncryptionPublicKeyResponseData>().await.unwrap();
            put_new_encryption_key_worker(app_callback, direct_channel_id, r.public_keys).await;
        },
        400 => {
            todo!();
        },
        _ => unreachable!(),
    };
}

pub fn send_message(app_callback: Callback<app::Msg>, direct_channel_id: i64, content: String) {
    wasm_bindgen_futures::spawn_local(async move {
        // Zero for newest.
        let key = get_encryption_key(app_callback.clone(), direct_channel_id, 0).await;

        let mut nonce: [u8; 16] = Default::default();
        log::info!("{}", nonce[0]);
        WebPage::crypto().get_random_values_with_u8_array(&mut nonce).unwrap();
        log::info!("{}", nonce[0]);

        let mut buffer = content.as_bytes().to_vec();
        encrypt_aes(&key.key, &nonce, &mut buffer).await;

        api::put("channels/direct/messages")
        .body(&json!({
            "directChannelId": direct_channel_id,
            "encryptionKeyId": key.encryption_key_id,
            "nonce": general_purpose::STANDARD.encode(nonce),
            "encryptedText": general_purpose::STANDARD.encode(buffer)
        }))
        .send(
            app_callback,
            move |r: ApiResponse<MessagesPutResponseData>| match r {
                ApiResponse::Ok(_) => (),
                ApiResponse::BadRequest(_) => todo!(),
            }
        );
    });
}

async fn get_encryption_key(
    app_callback: Callback<app::Msg>, direct_channel_id: i64, encryption_key_id: i64
) -> Arc<EncryptionKey> {
    loop {
        log::info!("get_encryption_key");
        if let Some(key) = CACHED_ENCRYPTION_KEYS.lock().unwrap().get(&encryption_key_id) {
            return key.clone();
        }

        let response = api::post("channels/direct/encryption/keys/getencryptedkey")
        .body(&json!({
            "directChannelId": direct_channel_id,
            "encryptionKeyId": encryption_key_id
        })).send_async().await;
        match response.status() {
            200 => {
                let r =
                    response.json::<EncryptionKeysGetEncryptedKeyResponseData>().await.unwrap();

                let private_key = get_private_key(
                    app_callback.clone(), direct_channel_id, r.encryption_block_id
                ).await;
                let mut buffer = general_purpose::STANDARD.decode(r.encrypted_key).unwrap();
                buffer = decrypt_rsa(&private_key, &mut buffer).await;
                let key = import_aes(&buffer).await;

                CACHED_ENCRYPTION_KEYS.lock().unwrap().put(encryption_key_id, Arc::new(EncryptionKey {
                    encryption_key_id: r.encryption_key_id,
                    encryption_block_id: r.encryption_block_id,
                    key: key.into(),
                }));
                USED_ENCRYPTION_KEYS.lock().unwrap().put(direct_channel_id, encryption_key_id);
            },
            400 => {
                let errors = response.json::<ErrorData>().await.unwrap().errors;
                if // DirectChannelEncryptionKeyNotFound
                    errors.len() == 1 &&
                    errors.get("encryptionKeyId").unwrap_or(&ErrorDataElement::default()).code == 3003
                {
                    put_new_encryption_key(app_callback.clone(), direct_channel_id).await;
                    continue;
                }

                todo!();
            },
            _ => unreachable!(),
        };
    }
}

async fn get_private_key(
    _app_callback: Callback<app::Msg>, direct_channel_id: i64, encryption_block_id: i64
) -> Arc<CryptoKey> {
    loop {
        log::info!("get_private_key");
        if let Some(key) = CACHED_ENCRYPTION_BLOCKS_PRIVATE.lock().unwrap().get(&encryption_block_id) {
            return key.0.clone();
        }

        let response = api::post("channels/direct/encryption/getprivatekey")
            .body(&json!({
                "directChannelId": direct_channel_id,
                "encryptionBlockId": encryption_block_id
            })).send_async().await;
        match response.status() {
            200 => {
                let r =
                    response.json::<EncryptionPrivateKeyResponseData>().await.unwrap();

                get_private_key_worker(direct_channel_id, encryption_block_id, r.encrypted_private_key).await;
            },
            400 => {
                todo!();
            },
            _ => unreachable!(),
        };
    }
}

async fn get_private_key_worker(direct_channel_id: i64, encryption_block_id: i64, encrypted_private_key: String) {
    let nounces = get_rsa_private_key_encryption_nounces(direct_channel_id);
    let encryption_block = ENCRYPTION_BLOCK_DATA.get();
    let encryption = match encryption_block.as_ref() {
        Some(e) => e,
        None => panic!("Encryption is not initialized."),
    };

    log::info!("encrypted {}", encrypted_private_key);

    let mut buffer = general_purpose::STANDARD.decode(encrypted_private_key).unwrap();
    let mut parts: [Vec<u8>; 4] = Default::default();

    let mut length_buffer: [u8; 4] = Default::default();
    length_buffer.copy_from_slice(&buffer[0..4]);
    let length = u32::from_le_bytes(length_buffer) as usize;

    let part_length = (buffer.len() - 4) / 4;
    let keys = [&encryption.c0, &encryption.c1, &encryption.c2, &encryption.c3];
    for i in 0..4 {
        let slice = &mut buffer[(4 + i * part_length)..(4 + (i + 1) * part_length)];
        //decrypt_aes(keys[i], &nounces[i], slice).await;
        parts[i].extend_from_slice(slice);
    }

    buffer.clear();
    for i in 0..(part_length * 4) {
        buffer.push(parts[i % 4][i / 4]);
    }

    log::info!("raw {}", general_purpose::STANDARD.encode(&buffer));

    let private_key = import_rsa(&buffer[0..length], "pkcs8", "decrypt").await;
    CACHED_ENCRYPTION_BLOCKS_PRIVATE.lock().unwrap().put(
        encryption_block_id, Arc::new(private_key).into()
    );

}

async fn put_new_encryption_key_worker(
    _app_callback: Callback<app::Msg>, direct_channel_id: i64, public_keys: Vec<EncryptionPublicKeyElementResponseData>
) {
    let key = generate_aes().await;
    let mut key_raw = export_key(&key, "raw").await;
    let mut elements = Vec::new();

    for element in public_keys {
        log::info!("smutne {}", element.public_key);
        let imported = import_rsa(
            &general_purpose::STANDARD.decode(&element.public_key).unwrap(),
            "spki", "encrypt"
        ).await;
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
    })).send_async().await;
    match response.status() {
        200 => {
            let r =
                response.json::<EncryptionKeysPutResponseData>().await.unwrap();

            CACHED_ENCRYPTION_KEYS.lock().unwrap().put(r.encryption_key_id, Arc::new(EncryptionKey {
                encryption_key_id: r.encryption_block_id,
                encryption_block_id: r.encryption_key_id,
                key: UnsafeSync(key),
            }));
            USED_ENCRYPTION_KEYS.lock().unwrap().put(direct_channel_id, r.encryption_key_id);
        },
        400 => {
            todo!();
        },
        _ => unreachable!(),
    };
}

async fn export_key(key: &CryptoKey, format: &str) -> Vec<u8> {
    let promise = WebPage::crypto().subtle().export_key(format, key)
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

    let key_promise = WebPage::crypto().subtle()
        .generate_key_with_object(&algorithm, true, &key_usages)
        .expect("Unable to generate RSA keys.");
    let key_pair: CryptoKeyPair = JsFuture::from(key_promise).await.unwrap().into();

    let public_key: CryptoKey = Reflect::get(
        &key_pair, &JsValue::from("publicKey")
    ).expect("Unable to get public key.").into();
    let private_key: CryptoKey = Reflect::get(
        &key_pair, &JsValue::from("privateKey")
    ).expect("Unable to get private key.").into();

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

    let key_promise = WebPage::crypto().subtle().import_key_with_object(
        format, &key_data, &algorithm, true, &key_usages
    ).unwrap();
    JsFuture::from(key_promise).await.unwrap().into()
}

async fn encrypt_rsa(key: &CryptoKey, data: &mut [u8]) -> Vec<u8> {
    let promise = WebPage::crypto().subtle()
        .encrypt_with_str_and_u8_array("RSA-OAEP", key, data)
        .expect("Unable to encrypt RSA data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.to_vec()
}

async fn decrypt_rsa(key: &CryptoKey, data: &mut [u8]) -> Vec<u8> {
    let promise = WebPage::crypto().subtle()
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

    let key_promise = WebPage::crypto().subtle()
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

    let key_promise = WebPage::crypto().subtle().import_key_with_object(
        "raw", &key_data, &algorithm, true, &key_usages
    ).unwrap();
    JsFuture::from(key_promise).await.unwrap().into()
}

async fn encrypt_aes(key: &CryptoKey, nonce: &[u8], data: &mut [u8]) {
    let algorithm = encryption_aes_algorithm(nonce);
    let promise = WebPage::crypto().subtle()
        .encrypt_with_object_and_u8_array(&algorithm, key, data)
        .expect("Unable to encrypt AES data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.copy_to(data);
}

async fn decrypt_aes(key: &CryptoKey, nonce: &[u8], data: &mut [u8]) {
    let algorithm = encryption_aes_algorithm(nonce);
    let promise = WebPage::crypto().subtle()
        .decrypt_with_object_and_u8_array(&algorithm, key, data)
        .expect("Unable to decrypt AES data.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.copy_to(data);
}

async fn encrypt_rsa_private_key(private_key: &CryptoKey, direct_channel_id: i64) -> Vec<u8> {
    let mut private_key_raw = export_key(private_key, "pkcs8").await;
    let length = private_key_raw.len();
    while private_key_raw.len() % 4 != 0 {
        private_key_raw.push(0);
    }

    let mut private_key_raw_parts: [Vec<u8>; 4] = Default::default();
    for i in 0..private_key_raw.len() {
        let part = &mut private_key_raw_parts[i % 4];
        part.push(private_key_raw[i]);
    }

    log::info!("raw {}", general_purpose::STANDARD.encode(&private_key_raw));

    let nounces = get_rsa_private_key_encryption_nounces(direct_channel_id);
    let encryption_block = ENCRYPTION_BLOCK_DATA.get();
    let encryption = match encryption_block.as_ref() {
        Some(e) => e,
        None => panic!("Encryption is not initialized."),
    };

    //encrypt_aes(&encryption.c0, &nounces[0], &mut private_key_raw_parts[0]).await;
    //encrypt_aes(&encryption.c1, &nounces[1], &mut private_key_raw_parts[1]).await;
    //encrypt_aes(&encryption.c2, &nounces[2], &mut private_key_raw_parts[2]).await;
    //encrypt_aes(&encryption.c3, &nounces[3], &mut private_key_raw_parts[3]).await;

    private_key_raw.clear();
    private_key_raw.extend_from_slice(&(length as u32).to_le_bytes());
    for part in &mut private_key_raw_parts {
        private_key_raw.append(part);
    }

    log::info!("encrypted {}", general_purpose::STANDARD.encode(&private_key_raw));

    private_key_raw
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

fn get_rsa_private_key_encryption_nounces(direct_channel_id: i64) -> Vec<Vec<u8>> {
    let mut vec = Vec::with_capacity(4);

    let mut digest = md5::compute(direct_channel_id.to_le_bytes()).0;
    vec.push(digest.to_vec());
    for _ in 0..3 {
        digest = md5::compute(digest).0;
        vec.push(digest.to_vec());
    }

    vec
}
