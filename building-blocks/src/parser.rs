use crate::RedisValue;
use nom::{
    bytes::streaming::{self as bytes, tag},
    character::streaming::char,
    combinator::{map, map_res},
    sequence::{self, delimited, terminated},
    IResult, Parser,
};
use sequence::preceded;
use std::str::{self, FromStr};

fn until_end(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let (i, data) = bytes::take_until("\r\n")(i)?;
    let (i, _) = tag("\r\n")(i)?;
    Ok((i, data))
}

fn error(i: &[u8]) -> IResult<&[u8], RedisValue> {
    let (i, _) = char('-')(i)?;
    let (i, data) = until_end(i)?;
    Ok((i, RedisValue::Err(data)))
}

fn simple_string(i: &[u8]) -> IResult<&[u8], RedisValue> {
    let (i, _) = char('+')(i)?;
    let (i, data) = until_end(i)?;
    Ok((i, RedisValue::Str(data)))
}

fn bulk_string(i: &[u8]) -> IResult<&[u8], RedisValue> {
    let (i, len) = map_res(
        map_res(preceded(char('$'), until_end), str::from_utf8),
        |s| s.parse::<u32>(),
    )(i)?;
    let (i, data) = terminated(bytes::take(len), tag("\r\n"))(i)?;
    Ok((i, RedisValue::Str(data)))
}

fn null(i: &[u8]) -> IResult<&[u8], RedisValue> {
    tag("*-1\r\n")
        .or(tag("$-1\r\n"))
        .parse(i)
        .map(|(i, _)| (i, RedisValue::Null))
}

fn integer(i: &[u8]) -> IResult<&[u8], RedisValue> {
    map_res(
        map_res(preceded(char(':'), until_end), str::from_utf8),
        |s| s.parse::<i64>().map(RedisValue::Int),
    )(i)
}

fn array(i: &[u8]) -> IResult<&[u8], RedisValue> {
    let (mut i, len) = map_res(
        map_res(preceded(char('*'), until_end), str::from_utf8),
        |s| s.parse::<u32>(),
    )(i)?;
    let mut vals = vec![];
    for _ in 0..len {
        let (rest, val) = value(i)?;
        i = rest;
        vals.push(val);
    }
    Ok((i, RedisValue::Array(vals)))
}

pub fn value(i: &[u8]) -> IResult<&[u8], RedisValue> {
    simple_string
        .or(bulk_string)
        .or(error)
        .or(null)
        .or(integer)
        .or(array)
        .parse(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_complete_value(s: &[u8], expected: RedisValue) {
        let (rest, val) = value(s).unwrap();
        assert_eq!(rest, []);
        assert_eq!(val, expected);
    }

    #[test]
    fn error() {
        test_complete_value(
            b"-This is an error\r\n",
            RedisValue::Err(b"This is an error"),
        );
    }

    #[test]
    fn simple_string() {
        test_complete_value(b"+OK\r\n", RedisValue::Str(b"OK"))
    }

    #[test]
    fn bulk_string() {
        test_complete_value(b"$6\r\nfoobar\r\n", RedisValue::Str(b"foobar"));
        test_complete_value(b"$0\r\n\r\n", RedisValue::Str(b""));
    }

    #[test]
    fn null() {
        test_complete_value(b"$-1\r\n", RedisValue::Null);
        test_complete_value(b"*-1\r\n", RedisValue::Null)
    }

    #[test]
    fn integer() {
        test_complete_value(b":1000\r\n", RedisValue::Int(1000))
    }

    #[test]
    fn array() {
        test_complete_value(b"*0\r\n", RedisValue::Array(vec![]));
        test_complete_value(
            b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n",
            RedisValue::Array(vec![RedisValue::Str(b"foo"), RedisValue::Str(b"bar")]),
        );
        test_complete_value(
            b"*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$6\r\nfoobar\r\n",
            RedisValue::Array(vec![
                RedisValue::Int(1),
                RedisValue::Int(2),
                RedisValue::Int(3),
                RedisValue::Int(4),
                RedisValue::Str(b"foobar"),
            ]),
        );
        test_complete_value(
            b"*2\r\n*3\r\n:1\r\n:2\r\n:3\r\n*2\r\n+Foo\r\n-Bar\r\n",
            RedisValue::Array(vec![
                RedisValue::Array(vec![
                    RedisValue::Int(1),
                    RedisValue::Int(2),
                    RedisValue::Int(3),
                ]),
                RedisValue::Array(vec![RedisValue::Str(b"Foo"), RedisValue::Err(b"Bar")]),
            ]),
        );
    }
}
