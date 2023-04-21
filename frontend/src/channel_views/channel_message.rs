use std::sync::Arc;
use lazy_static::__Deref;
use yew::prelude::*;

use crate::{localization, common::UnsafeSync, api};

use super::channel_message_error::ChannelMessageError;

static IMAGE_EXTENSIONS: [&str; 10] = ["jpg", "jpeg", "apng", "png", "avif", "gif", "webp", "svg", "bmp", "ico"];
static VIDEO_EXTENSIONS: [&str; 4] = ["mp4", "webm", "ogg", "wav"];

#[derive(Clone, PartialEq)]
pub struct ChannelMessage {
    pub message_id: i64,
    pub author_user_id: i64,
    content: Result<Arc<String>, ChannelMessageError>,
    html: UnsafeSync<Html>,
}

impl ChannelMessage {
    pub fn new(message_id: i64, author_user_id: i64, content: Result<Arc<String>, ChannelMessageError>) -> Self {
        let html = match &content {
            Ok(content) => {
                let content = Self::transform_attachments(content);
                match message_id > 0 {
                    true => html! { content },
                    false => html! {
                        <span class="message-sent">{content}</span>
                    },
                }
            },
            Err(err) => {
                let lang = localization::get_language();
                html! {
                    <span class="message-error">{lang.get(err.to_translation_key())}</span>
                }
            }
        };

        Self {
            message_id,
            author_user_id,
            content,
            html: UnsafeSync(html),
        }
    }

    pub fn get_html(&self) -> &Html {
        &self.html
    }

    fn find_pointer(content: &mut str) -> Option<(usize, usize)> {
        if let Some(c) = content.chars().position(|c| c == '<') {
            if let Some(d) = content.chars().position(|c| c == '>') {
                if d as isize - c as isize > 1 {
                    return Some((c, d));
                }
            }
        }
        None
    }

    fn transform_attachments(content: &Arc<String>) -> Html {
        let mut vec = Vec::new();

        let mut content = content.clone().deref().clone();
        if let Some((c, d)) = Self::find_pointer(&mut content) {
            if !content[c + 1..d].starts_with("^a/") {
                return html! { content };
            }

            let mut parts = content[c + 4..d].split('/');
            if parts.clone().count() != 3 {
                return html! { content };
            }

            let attachment_id = parts.next().unwrap().parse::<i64>().unwrap();
            let name = parts.next().unwrap();
            let key = parts.next().unwrap();

            vec.push(html! { &content[..c] });
            if c > 0 {
                vec.push(html! { <br/> });
            }

            let url = format!("{}/attachments/direct/{}/{}/{}", api::DOMAIN, key, attachment_id, name);

            let extension = name.split('.').last().unwrap_or("").to_lowercase();
            if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                vec.push(html! { <img class="channel-message-embeded" src={url} /> });
            } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
                vec.push(html! { <video class="channel-message-embeded" controls=true>
                    <source src={url} type={format!("video/{}", extension)} />
                    {"Your browser does not support the video tag."}
                </video> });
            } else {
                vec.push(html! { <div>
                    <p><strong>{name}</strong></p>
                </div> });
            }

            vec.push(html! { &content[d + 1..] });
        } else {
            vec.push(html! { content });
        }

        html! { <>{vec}</> }
    }
}
