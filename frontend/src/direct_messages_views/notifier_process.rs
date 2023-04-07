use serde::{Serialize, Deserialize};

use crate::channel_views::channel;

use super::encryption;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedDirectMessageData {
    direct_channel_id: i64,
    direct_message_id: i64,
    author_user_id: i64,
    encryption_key_id: i64,
    nonce: String,
    encrypted_text: String
}

pub async fn received_direct_message(data: ReceivedDirectMessageData) {
    channel::notify_message(data.direct_channel_id, encryption::decrypt_message(
        data.direct_channel_id,
        data.direct_message_id,
        data.author_user_id,
        data.encryption_key_id,
        data.nonce,
        data.encrypted_text
    ).await);
}
