mod parser;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RedisValue<'a> {
    Null,
    Str(&'a [u8]),
    Err(&'a [u8]),
    Array(Vec<RedisValue<'a>>),
    Int(i64),
}
