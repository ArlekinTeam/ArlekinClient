
use std::{error::Error, fmt::Display};

#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionError {
    UnableToRead
}

impl EncryptionError {
    pub fn to_translation_key(&self) -> &str {
        match self {
            EncryptionError::UnableToRead => "encryptionUnableToRead"
        }
    }
}

impl Error for EncryptionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
