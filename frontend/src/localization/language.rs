use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Language {
    translations: HashMap<String, String>,
}

impl Language {
    pub fn get(&self, key: &str) -> String {
        match self.translations.get(key) {
            Some(value) => value.clone(),
            None => format!("{{{{{key}}}}}"),
        }
    }
}
