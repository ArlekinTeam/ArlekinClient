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
        let lang = localization::get_language();
        let mut vec = Vec::new();
        let mut keys = err.iter().map(|x| &x.1.translation_key).collect::<Vec<&String>>();
        keys.dedup();
        for translation_key in keys {
            vec.push(html! {
                <span>{lang.get(translation_key)}</span>});
        }

        html! { 
            <div class="status-error">
                {vec}
            </div>
         }
    }
}
