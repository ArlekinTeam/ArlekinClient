use arc_cell::ArcCell;
use gloo_net::{http::Request, Error};
use once_cell::sync::Lazy;
use std::sync::Arc;
use yew::suspense::{Suspension, SuspensionResult};

use self::language::Language;

pub mod language;

static CURRENT_LANGUAGE: Lazy<ArcCell<Option<Arc<Language>>>> = Lazy::new(ArcCell::default);

pub async fn get_language_from_code_async(language: &str) -> Result<Language, Error> {
    let response =
        match Request::get(format!("/static/localization/languages/{language}.json").as_str())
            .send()
            .await
        {
            Ok(r) => {
                if r.ok() {
                    r
                } else {
                    Request::get("/static/localization/languages/en-US.json")
                        .send()
                        .await
                        .unwrap()
                }
            }
            Err(_) => Request::get("/static/localization/languages/en-US.json")
                .send()
                .await
                .unwrap(),
        };

    if !response.ok() {
        log::error!("Language failed to load.");
        return Ok(Language::default());
    }

    response.json::<Language>().await
}

pub async fn get_language_async() -> Arc<Language> {
    if let Some(current) = Option::as_ref(&CURRENT_LANGUAGE.get()) {
        return current.clone();
    }

    let language = Arc::new(get_language_from_code_async("en-US").await.unwrap());
    CURRENT_LANGUAGE.set(Arc::new(Some(language.clone())));
    language
}

pub fn init_language() -> SuspensionResult<()> {
    if Option::as_ref(&CURRENT_LANGUAGE.get()).is_some() {
        return Ok(());
    }

    let suspension = Suspension::from_future(async {
        get_language_async().await;
    });
    if suspension.resumed() {
        if Option::as_ref(&CURRENT_LANGUAGE.get()).is_some() {
            Ok(())
        } else {
            unreachable!("Unable to get language.")
        }
    } else {
        Err(suspension)
    }
}

pub fn get_language() -> Arc<Language> {
    if let Some(current) = Option::as_ref(&CURRENT_LANGUAGE.get()) {
        return current.clone();
    }

    log::error!("Function init_language() was not called.");
    Arc::new(Default::default())
}
