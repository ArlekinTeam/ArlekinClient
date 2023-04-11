use web_sys::{Crypto, Document, Storage, Window};

pub struct WebPage {}

impl WebPage {
    #[inline]
    pub fn window() -> Window {
        web_sys::window().unwrap()
    }

    #[inline]
    pub fn document() -> Document {
        Self::window().document().unwrap()
    }

    #[inline]
    pub fn crypto() -> Crypto {
        Self::window()
            .crypto()
            .expect("Web browser does not support crypto.")
    }

    #[inline]
    pub fn local_storage() -> Storage {
        Self::window().local_storage().unwrap().unwrap()
    }
}
