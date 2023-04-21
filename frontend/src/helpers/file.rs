use img_parts::{jpeg::Jpeg, png::Png, webp::WebP, ImageEXIF};
use wasm_bindgen_futures::JsFuture;

pub struct File {}

impl File {
    pub async fn to_bytes(file: &web_sys::File) -> Vec<u8> {
        let array_buffer: js_sys::ArrayBuffer =
            JsFuture::from(file.array_buffer()).await.unwrap().into();
        let buffer = js_sys::Uint8Array::new(&array_buffer);
        buffer.to_vec()
    }

    pub async fn to_bytes_without_exif(file: &web_sys::File) -> Vec<u8> {
        // TODO: Remove exif data.
        let bytes = Self::to_bytes(file).await;
        let extension = file.name().split('.').last().unwrap_or("").to_lowercase();

        let mut result = Vec::new();
        let r = match extension.as_str() {
            "jpg" => match Jpeg::from_bytes(bytes.clone().into()) {
                Ok(mut r) => {
                    r.set_exif(None);
                    r.encoder().write_to(&mut result).is_ok()
                }
                Err(_) => false,
            },
            "png" => match Png::from_bytes(bytes.clone().into()) {
                Ok(mut r) => {
                    r.set_exif(None);
                    r.encoder().write_to(&mut result).is_ok()
                }
                Err(_) => false,
            },
            "webp" => match WebP::from_bytes(bytes.clone().into()) {
                Ok(mut r) => {
                    r.set_exif(None);
                    r.encoder().write_to(&mut result).is_ok()
                }
                Err(_) => false,
            },
            _ => false,
        };

        match r {
            true => result,
            false => bytes,
        }
    }
}
