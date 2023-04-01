use super::prelude::*;

pub struct Element {}

impl Element {
    #[inline]
    pub fn by_id(element_id: &str) -> web_sys::Element {
        WebPage::document().get_element_by_id(element_id).unwrap()
    }
}
