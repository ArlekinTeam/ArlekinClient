use web_sys::{Document, Window};

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
}
