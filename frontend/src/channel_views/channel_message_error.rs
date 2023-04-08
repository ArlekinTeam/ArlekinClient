use std::{error::Error, fmt::Display};

use crate::direct_messages_views::encryption_error::EncryptionError;

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelMessageError {
    Encryption(EncryptionError),
}

impl ChannelMessageError {
    pub fn to_translation_key(&self) -> &str {
        match self {
            ChannelMessageError::Encryption(e) => e,
        }
        .to_translation_key()
    }
}

impl Error for ChannelMessageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for ChannelMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
