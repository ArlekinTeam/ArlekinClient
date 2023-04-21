use wasm_bindgen_futures::JsFuture;

pub struct File {}

impl File {
    pub async fn to_bytes(file: &web_sys::File) -> Vec<u8> {
        let array_buffer: js_sys::ArrayBuffer = JsFuture::from(file.array_buffer()).await.unwrap().into();
        let buffer = js_sys::Uint8Array::new(&array_buffer);
        buffer.to_vec()
    }

    pub async fn to_bytes_without_exif(file: &web_sys::File) -> Vec<u8> {
        // TODO: Remove exif data.
        Self::to_bytes(file).await
    }
}
