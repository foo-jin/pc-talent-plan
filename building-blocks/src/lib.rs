mod de;
mod error;
mod parse;
mod ping;
mod ser;

pub use de::{from_reader, Deserializer};
pub use error::{Error, Result};
pub use ping::{Ping, PingResponse};
pub use ser::{to_writer, Serializer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedisValue<'a> {
    Null,
    Str(&'a [u8]),
    Err(&'a [u8]),
    Array(Vec<RedisValue<'a>>),
    Int(i64),
}
