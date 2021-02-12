use serde::{de, ser};
use std::{self, fmt::Display, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Created by data structures through the `serde::ser::Error` and
    /// `serde::de::Error` traits.
    #[error("{0}")]
    Message(String),
    /// I/O error encountered during (de)serialization.
    #[error("{0}")]
    Io(#[from] io::Error),
    /// Length of sequence not available during serialization.
    #[error("sequence length not available")]
    LenNotAvailable,

    /// Unexpected EOF.
    #[error("unexpected EOF")]
    Eof,
    /// Non-utf8 data encountered during deserialization.
    #[error("invalid data: {0}")]
    InvalidData(#[from] std::str::Utf8Error),
    /// Failed to parse integer.
    #[error("failed to parse integer: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    /// Incorrect sequence length indication encountered
    #[error("incorrect sequence length indication encountered")]
    InvalidLen,
    /// Unexpected tokens encountered in input
    #[error("unexpected byte encountered: {0}")]
    InvalidFormat(u8),
    /// Encountered an empty bulk array when expecting command.
    #[error("empty bulk array is not a valid command")]
    InvalidCommand,

    /// Did not encounter array when expected
    #[error("expected array")]
    ExpectedArray,
    /// Did not encounter integer when expected
    #[error("expected int")]
    ExpectedInt,
    /// Did not encounter bulk string when expected
    #[error("expected bulk string")]
    ExpectedBulkString,
    /// Did not encounter simple string when expected
    #[error("expected simple string")]
    ExpectedSimpleString,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
