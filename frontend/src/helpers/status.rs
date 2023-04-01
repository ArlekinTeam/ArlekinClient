use std::collections::HashMap;
use yew::prelude::*;

use crate::{api::ErrorDataElement, localization};

pub struct Status {}

impl Status {
    pub fn with_ok(translation_key: &str) -> Html {
        let lang = localization::get_language();
        html! { <p style={"color: green"}>{lang.get(translation_key)}</p> }
    }

    pub fn with_err(err: HashMap<String, ErrorDataElement>) -> Html {
        let mut message = String::new();
        for (key, value) in err {
            message.push_str(&key);
            message.push_str(": ");
            message.push_str(&value.translation_key);
            message.push_str("<br>");
        }

        html! { <p style={"color: red"}>{message}</p> }
    }
}
