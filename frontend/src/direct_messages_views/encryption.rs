use std::sync::Arc;

use arc_cell::ArcCell;
use js_sys::Reflect;
use once_cell::sync::Lazy;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKeyPair, CryptoKey};

use crate::helpers::prelude::WebPage;

const RSA_BITS: usize = 4096;
const AES_BITS: usize = 256;
const AES_BLOCK_BITS: usize = 64;
static ENCRYPTION_BLOCK_DATA: Lazy<ArcCell<Option<PrivateKeyEncryptionData>>> = Lazy::new(ArcCell::default);

struct PrivateKeyEncryptionData {
    c0: CryptoKey,
    c1: CryptoKey,
    c2: CryptoKey,
    c3: CryptoKey
}

unsafe impl Send for PrivateKeyEncryptionData {}
unsafe impl Sync for PrivateKeyEncryptionData {}

pub async fn init(encryption_block_hash: &[u8]) {
    let mut chunks = encryption_block_hash.chunks(AES_BITS / 8);
    ENCRYPTION_BLOCK_DATA.set(Arc::new(Some(PrivateKeyEncryptionData {
        c0: import_aes(chunks.next().unwrap()).await,
        c1: import_aes(chunks.next().unwrap()).await,
        c2: import_aes(chunks.next().unwrap()).await,
        c3: import_aes(chunks.next().unwrap()).await,
    })));

    // TODO: remove this.
    put_new_encryption_block(0);
}

pub fn put_new_encryption_block(direct_channel_id: i64) {
    wasm_bindgen_futures::spawn_local(async move {
        web_sys::console::time();

        let (_public_key, private_key) = generate_rsa().await;

        let mut private_key_raw = export_key(&private_key, "pkcs8").await;
        log::info!("{}", private_key_raw.iter().map(|&x| x as char).collect::<String>());

        let mut private_key_raw_parts: [Vec<u8>; 4] = Default::default();
        for i in 0..private_key_raw.len() {
            let part = &mut private_key_raw_parts[i % 4];
            part.push(private_key_raw[i]);
        }

        let nounces = get_rsa_private_key_encryption_nounces(direct_channel_id);
        let encryption_block = ENCRYPTION_BLOCK_DATA.get();
        let encryption = match encryption_block.as_ref() {
            Some(e) => e,
            None => panic!("Encryption is not initialized."),
        };

        encrypt_aes(&encryption.c0, &nounces[0], &mut private_key_raw_parts[0]).await;
        encrypt_aes(&encryption.c1, &nounces[1], &mut private_key_raw_parts[1]).await;
        encrypt_aes(&encryption.c2, &nounces[2], &mut private_key_raw_parts[2]).await;
        encrypt_aes(&encryption.c3, &nounces[3], &mut private_key_raw_parts[3]).await;

        private_key_raw.clear();
        for part in &mut private_key_raw_parts {
            private_key_raw.append(part);
        }

        log::info!("{}", private_key_raw.iter().map(|&x| x as char).collect::<String>());

        web_sys::console::time_end();
    });
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

/*async fn generate_aes(nonce: &[u8]) -> CryptoKey {
    let algorithm = js_sys::Object::new();
    let counter = js_sys::Uint8Array::new_with_length(3);
    counter.copy_from(nonce);

    Reflect::set(&algorithm, &"counter".into(), &counter).unwrap();
    Reflect::set(&algorithm, &"name".into(), &"AES-CTR".into()).unwrap();
    Reflect::set(&algorithm, &"length".into(), &AES_BITS.into()).unwrap();

    let key_usages = js_sys::Array::new_with_length(2);
    key_usages.set(0, "encrypt".into());
    key_usages.set(1, "decrypt".into());

    let key_promise = WebPage::crypto().subtle()
        .generate_key_with_object(&algorithm, true, &key_usages)
        .expect("Unable to generate AES key.");
    JsFuture::from(key_promise).await.unwrap().into()
}*/

async fn export_key(key: &CryptoKey, format: &str) -> Vec<u8> {
    let promise = WebPage::crypto().subtle().export_key(format, key)
        .expect("Unable to export key.");

    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(promise).await.unwrap().into();
    let buffer = js_sys::Uint8Array::new(&array_buffer);
    buffer.to_vec()
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
