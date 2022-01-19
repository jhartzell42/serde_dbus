use thiserror;

use std::char::CharTryFromError;
use std::fmt::Display;
use std::str::Utf8Error;

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("serde error serializing")]
    Serializing(String),

    #[error("serde error deserializing")]
    Deserializing(String),

    #[error("mismatch signature in array: {0:?}, {1:?}")]
    MismatchSignature(Vec<u8>, Vec<u8>),

    #[error("converting from bytes to string: {0}")]
    StringConversion(#[from] Utf8Error),

    #[error("leftover data to deserialize: {0}")]
    LeftoverData(usize),

    #[error("leftover signature to deserialize: {0}")]
    LeftoverSignature(usize),

    #[error("signature: unrecognized {0:X}")]
    UnrecognizedSignatureCharacter(u8),

    #[error("signature: unsupported {0:X}")]
    UnsupportedSignatureCharacter(u8),

    #[error("signature: expected {0:X} got {1:X}")]
    SignatureError(u8, u8),

    #[error("signature: expected {0:?} at {1}")]
    SignatureErrorIx(Vec<u8>, usize),

    #[error("out of signature")]
    SignatureExhausted,

    #[error("index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    #[error("invalid bool value: {0}")]
    InvalidBoolValue(u32),

    #[error("invalid char: {0}")]
    CharTryFromError(#[from] CharTryFromError),

    #[error("Mismatched bracketing in signature at index: {0}")]
    MismatchedSignatureBracketing(usize),

    #[error("Array element ended at {0} overrunning bound at {1}")]
    ArrayElementOverrun(usize, usize),
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
