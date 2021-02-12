use crate::{Error, Result};
use serde::{
    de::{self, IntoDeserializer},
    forward_to_deserialize_any, Deserialize,
};
use std::{convert::TryFrom, io::BufRead, str};

pub struct Deserializer<R> {
    reader: R,
    buffer: Vec<u8>,
}

impl<R: BufRead> Deserializer<R> {
    fn new(reader: R) -> Self {
        Deserializer {
            reader,
            buffer: Vec::new(),
        }
    }
}

pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: BufRead,
    T: de::DeserializeOwned,
{
    let mut de = Deserializer::new(reader);
    let t = T::deserialize(&mut de)?;
    Ok(t)
}

impl<'de, R: BufRead> de::Deserializer<'de> for &mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!();
        let buf = self.read_next_item()?;
        let rest = &buf[1..];
        match buf[0] {
            b'+' => visitor.visit_bytes(rest),
            b'-' => visitor.visit_str(str::from_utf8(rest)?),
            b':' => {
                let int_str = str::from_utf8(rest)?;
                let val = int_str.parse::<i64>()?;
                visitor.visit_i64(val)
            }
            b'$' => match self.parse_bulk_string()? {
                Some(bytes) => visitor.visit_bytes(bytes),
                None => visitor.visit_none(),
            },
            b'*' => {
                let len = match self.parse_len()? {
                    Some(len) => len,
                    None => return visitor.visit_none(),
                };
                for _ in 0..len {}
                unimplemented!()
            }
            _ => unimplemented!(),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_str(&self.parse_any_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.parse_any_str()?.to_owned())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        println!("option");
        let buf = self.read_next_item()?;
        if buf.starts_with(b"*-1") || buf.starts_with(b"$-1") {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let buf = self.read_next_item()?;
        if buf[0] != b'*' {
            return Err(Error::ExpectedArray);
        }

        let len = match self.parse_len()? {
            Some(s) => s,
            None => {
                return Err(Error::ExpectedArray);
            }
        };

        if len == 0 {
            return Err(Error::InvalidCommand);
        }

        let _ = self.read_next_item()?;
        println!("pre cmd");
        let cmd_name = match self.parse_bulk_string()? {
            Some(s) => s,
            None => return Err(Error::ExpectedBulkString),
        };
        let cmd_name = str::from_utf8(cmd_name)?;
        if cmd_name != name.to_uppercase() {
            return Err(Error::Message(format!(
                "invalid command: '{}', expected '{}'",
                cmd_name,
                name.to_uppercase()
            )));
        }
        println!("post cmd");

        visitor.visit_seq(Command {
            de: &mut *self,
            remaining: len - 1,
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = self.read_next_item()?;
        // super scuffed string only impl
        let s = self.parse_any_str()?;
        visitor.visit_enum(s.into_deserializer())
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
            bytes byte_buf unit unit_struct newtype_struct seq tuple
            tuple_struct map identifier ignored_any
    }
}

impl<R: BufRead> Deserializer<R> {
    fn read_next_item(&mut self) -> Result<&[u8]> {
        self.buffer.clear();
        let len = self.reader.read_until(b'\n', &mut self.buffer)?;
        dbg!(str::from_utf8(&self.buffer)?);

        if len == 0 {
            return Err(Error::Eof);
        } else if !self.buffer.ends_with(&[b'\r', b'\n']) {
            return Err(Error::InvalidFormat(b'\n'));
        }

        self.buffer.truncate(len - 2);
        Ok(&self.buffer)
    }

    fn parse_any_str(&mut self) -> Result<&str> {
        let first = self.buffer.get(0).ok_or(Error::InvalidFormat(b'\r'))?;
        let bytes = match first {
            b'$' => self.parse_bulk_string()?.ok_or(Error::ExpectedBulkString)?,
            b'+' | b'-' => &self.buffer[1..],
            b => return Err(Error::InvalidFormat(*b)),
        };
        let s = std::str::from_utf8(bytes)?;
        Ok(s)
    }

    fn parse_bulk_string(&mut self) -> Result<Option<&[u8]>> {
        match self.buffer[0] {
            b'$' => (),
            _ => return Err(Error::ExpectedBulkString),
        }
        let len = match self.parse_len()? {
            Some(len) => len,
            None => return Ok(None),
        };

        self.buffer.resize(len + 2, 0);
        let buf = &mut self.buffer;
        self.reader.read_exact(buf)?;
        dbg!(str::from_utf8(&buf)?);
        if !buf.ends_with(&[b'\r', b'\n']) {
            return Err(Error::InvalidLen);
        }
        self.buffer.truncate(len);
        Ok(Some(&self.buffer))
    }

    fn parse_len(&mut self) -> Result<Option<usize>> {
        let int_str = str::from_utf8(&self.buffer[1..])?;
        let len = int_str.parse::<isize>()?;
        if len == -1 {
            return Ok(None);
        }
        let len = usize::try_from(len).map_err(|_| Error::InvalidLen)?;
        Ok(Some(len))
    }
}

struct Command<'a, R> {
    de: &'a mut Deserializer<R>,
    remaining: usize,
}

impl<'a, 'de, R: BufRead> de::SeqAccess<'de> for Command<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct Enum<'a, R> {
    de: &'a mut Deserializer<R>,
}

impl<'a, 'de, R: BufRead> de::EnumAccess<'de> for Enum<'a, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'a, 'de, R: BufRead> de::VariantAccess<'de> for Enum<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        todo!();
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        todo!()
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        todo!()
    }
}
