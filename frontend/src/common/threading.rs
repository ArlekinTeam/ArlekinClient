use js_sys::Promise;
use wasm_bindgen_futures::JsFuture;

use crate::helpers::prelude::*;

pub async fn sleep(ms: i32) {
    let promise = Promise::new(&mut |c, _| {
        WebPage::window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&c, ms)
            .unwrap();
    });
    let future = JsFuture::from(promise);
    future.await.unwrap();
}
