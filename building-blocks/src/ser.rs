//! The serialization implemented in this crate follows the
//! description of the REdis Serialization Protocol (RESP) as
//! described in [the Redis Protocol
//! specification](https://redis.io/topics/protocol).

use crate::{Error, Result};
use serde::{ser, Serialize};
use std::io::Write;

pub struct Serializer<W> {
    writer: W,
}

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    let mut serializer = Serializer { writer };
    value.serialize(&mut serializer)?;
    Ok(())
}

impl<W: Write> ser::Serializer for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        write!(&mut self.writer, ":{}", v)?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        write!(&mut self.writer, "${}\r\n", v.len())?;
        self.writer.write_all(v)?;
        self.writer.write_all(b"\r\n")?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.writer.write_all(b"$-1\r\n")?;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = len.ok_or(Error::LenNotAvailable)?;
        write!(&mut self.writer, "*{}\r\n", len)?;
        Ok(self)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        name.to_uppercase().serialize(&mut *self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        variant.to_uppercase().serialize(&mut *self)
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        write!(&mut self.writer, "*{}\r\n", len + 1)?;
        let name = name.to_uppercase();
        self.serialize_bytes(name.as_bytes())?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        write!(&mut self.writer, "*{}\r\n", len + 1)?;
        let name = variant.to_uppercase();
        self.serialize_bytes(name.as_bytes())?;
        Ok(self)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        todo!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        todo!()
    }
}

impl<W: Write> ser::SerializeSeq for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W: Write> ser::SerializeStruct for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W: Write> ser::SerializeStructVariant for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W: Write> ser::SerializeTuple for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

macro_rules! todo_impl {
    ($trait_name:path) => {
        impl<W: Write> $trait_name for & mut Serializer<W> {
            type Ok = ();
            type Error = Error;

            fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<()>
            where
                T: Serialize,
            {
                todo!()
            }

            fn end(self) -> Result<Self::Ok> {
                todo!()
            }
        }
    };

	($trait_name:path, $($rest:path),+) => {
		todo_impl!( $trait_name );
		todo_impl!( $($rest),+ );
	};
}
todo_impl!(ser::SerializeTupleStruct, ser::SerializeTupleVariant);

impl<W: Write> ser::SerializeMap for &mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

///////////////////////////////////////////////////////////////////////////////////

#[test]
fn test_command() {
    #[derive(Serialize)]
    struct Ping<'a> {
        msg: Option<&'a str>,
    }

    let mut buffer = Vec::new();
    let ping = Ping { msg: Some("test") };
    to_writer(&mut buffer, &ping).unwrap();
    dbg!(&std::str::from_utf8(&buffer));
    assert_eq!(&buffer, b"*2\r\n$4\r\nPING\r\n$4\r\ntest\r\n");
}
