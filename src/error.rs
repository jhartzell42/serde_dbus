use thiserror;

use std;
use std::fmt::Display;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error ("Error serializing")]
    Serializing(String),
    #[error ("Error deserializing")]
    Deserializing(String),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serializing(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Deserializing(msg.to_string())
    }
}
