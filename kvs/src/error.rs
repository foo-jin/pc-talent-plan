use std::io;
use thiserror::Error;

/// Convenience alias for `Result<T, KvsError>`.
pub type Result<T> = std::result::Result<T, KvsError>;

/// All errors that can be encountered by the KvStore.
#[derive(Error, Debug)]
pub enum KvsError {
    /// IO error
    #[error("{0}")]
    Io(#[from] io::Error),
    /// Serialization error
    #[error("{0}")]
    Ser(#[from] rmp_serde::encode::Error),
    /// Deserialization error
    #[error("{0}")]
    Des(#[from] rmp_serde::decode::Error),
    /// Error on remove with a non-existent key
    #[error("No such key: `{0}`")]
    NonExistentKey(String),
    /// Error on finding an unexpected command when retrieving a
    /// value. This indicates a corrupted log or a program error.
    #[error("Unexpected command type")]
    UnexpectedCommandType,
}
