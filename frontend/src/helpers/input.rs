use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use super::prelude::*;

pub struct Input {}

impl Input {
    #[inline]
    pub fn by_id(element_id: &str) -> HtmlInputElement {
        Element::by_id(element_id)
            .dyn_into::<HtmlInputElement>()
            .unwrap()
    }
}
