use crate::error::{Error, Result};
use crate::message::Message;
use crate::primitives::DbusPrimitive;
use byteorder::{ByteOrder, LE};

use std::convert::TryInto;

use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserialize;

mod internal;
use internal::{ArrayDeserializer, DataBuffer, Deserializer};

pub fn from_message<'a, T: Deserialize<'a>>(mesg: &'a Message) -> Result<T> {
    let mut buff = DataBuffer::from_message(mesg);
    let de = Deserializer::<LE>::from_message_and_buffer(&mut buff, mesg);
    let t = T::deserialize(de)?;
    buff.complete()?;
    Ok(t)
}

impl<'de, 'a, B: ByteOrder> de::Deserializer<'de> for Deserializer<'a, 'de, B> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // TODO: Maybe make all numeric types work with all signatures,
        // to be more "liberal in what we expect", and route everything
        // through here if it is numeric.
        let sig = self.peek_single_sig()?;
        match sig[0] {
            b'y' => self.deserialize_u8(visitor),
            b'b' => self.deserialize_bool(visitor),
            b'n' => self.deserialize_i16(visitor),
            b'q' => self.deserialize_u16(visitor),
            b'i' => self.deserialize_i32(visitor),
            b'u' => self.deserialize_u32(visitor),
            b'x' => self.deserialize_i64(visitor),
            b't' => self.deserialize_u64(visitor),
            b'd' => self.deserialize_f64(visitor),
            b's' => self.deserialize_str(visitor),
            b'a' => match sig[1] {
                b'{' => self.deserialize_map(visitor),
                _ => self.deserialize_seq(visitor),
            },
            b'(' => self.deserialize_tuple(0, visitor),
            b'v' => {
                let de = self.possible_variant()?;
                de.deserialize_any(visitor)
            }
            b'{' => self.deserialize_seq(visitor),
            b'h' => Err(Error::UnsupportedSignatureCharacter(sig[0])), // UNIX_FD
            b'o' => Err(Error::UnsupportedSignatureCharacter(sig[0])), // OBJECT_PATH
            b'g' => Err(Error::UnsupportedSignatureCharacter(sig[0])), // SIGNATURE
            _ => Err(Error::UnrecognizedSignatureCharacter(sig[0])),
        }
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_u32(&de.read_align_signature_value::<4>(bool::signature(), 4)?);
        if i > 1 {
            return Err(Error::InvalidBoolValue(i));
        }
        visitor.visit_bool(i == 1)
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_i16(&de.read_align_signature_value::<2>(i16::signature(), 2)?);
        visitor.visit_i8(i as i8)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_i16(&de.read_align_signature_value::<2>(i16::signature(), 2)?);
        visitor.visit_i16(i)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_i32(&de.read_align_signature_value::<4>(i32::signature(), 4)?);
        visitor.visit_i32(i)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_i64(&de.read_align_signature_value::<8>(i64::signature(), 8)?);
        visitor.visit_i64(i)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = de.read_align_signature_value::<1>(u8::signature(), 1)?[0];
        visitor.visit_u8(i)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_u16(&de.read_align_signature_value::<2>(u16::signature(), 2)?);
        visitor.visit_u16(i)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_u32(&de.read_align_signature_value::<4>(u32::signature(), 4)?);
        visitor.visit_u32(i)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_u64(&de.read_align_signature_value::<8>(u64::signature(), 8)?);
        visitor.visit_u64(i)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_f64(&de.read_align_signature_value::<8>(f64::signature(), 4)?);
        visitor.visit_f32(i as f32)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_f64(&de.read_align_signature_value::<8>(f64::signature(), 8)?);
        visitor.visit_f64(i)
    }

    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let i = B::read_u32(&de.read_align_signature_value::<4>(u32::signature(), 2)?);
        visitor.visit_char(i.try_into()?)
    }

    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let string = de.deserialize_str_basic()?;
        visitor.visit_str(string)
    }

    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let string = de.deserialize_str_basic()?;
        visitor.visit_string(string.to_owned())
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let bytes = de.deserialize_bytes_basic()?;
        visitor.visit_bytes(bytes)
    }

    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        let bytes = de.deserialize_bytes_basic()?;
        visitor.visit_byte_buf(bytes.into())
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        if de.probe_signature_bytes("()".as_bytes()) {
            de.align_reader(8)?;
            visitor.visit_none()
        } else {
            visitor.visit_some(de)
        }
    }

    fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        de.expect_signature_str("()".as_bytes())?;
        de.align_reader(8)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        if de.probe_signature_bytes("(".as_bytes()) {
            de.align_reader(8)?;
            visitor.visit_seq(StructDeserializer(de))
        } else {
            // Expect 'a' included in ArrayDeserializer::new
            visitor.visit_seq(ArrayDeserializer::new(de)?)
        }
    }

    fn deserialize_tuple<V>(mut self, _: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        de.expect_signature_byte(b'(')?;
        de.align_reader(8)?;
        visitor.visit_seq(StructDeserializer(de))
    }

    fn deserialize_tuple_struct<V>(self, _: &'static str, sz: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(sz, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let de = self.possible_variant()?;
        visitor.visit_map(ArrayDeserializer::new(de)?)
    }

    fn deserialize_struct<V>(
        mut self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut de = self.possible_variant()?;
        if de.probe_signature_bytes("(".as_bytes()) {
            visitor.visit_seq(StructDeserializer(de))
        } else {
            visitor.visit_map(ArrayDeserializer::new(de)?)
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(EnumDeserializer { de: self, name })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, 'a, B: ByteOrder> ArrayDeserializer<'a, 'de, B> {
    fn new(mut de: Deserializer<'a, 'de, B>) -> Result<Self> {
        de.expect_signature_byte(b'a')?;
        de.align_reader(4)?;
        let len = B::read_u32(
            &de.read(4)?
        ) as usize;
        de.array_deserializer(len)
    }
}

impl<'de, 'a, B: ByteOrder> SeqAccess<'de> for ArrayDeserializer<'a, 'de, B> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if let Some(de) = self.next_item_deserializer()? {
            Ok(Some(seed.deserialize(de)?))
        } else {
            Ok(None)
        }
    }
}

impl<'de, 'a, B: ByteOrder> MapAccess<'de> for ArrayDeserializer<'a, 'de, B> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some(de) = self.next_key_deserializer()? {
            Ok(Some(seed.deserialize(de)?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if let Some(de) = self.next_value_deserializer()? {
            Ok(seed.deserialize(de)?)
        } else {
            Err(Error::ArrayElementOverrun(
                self.data_buffer.data_ix,
                self.end_ix,
            ))
        }
    }
}

struct StructDeserializer<'a, 'de, B: ByteOrder>(Deserializer<'a, 'de, B>);

impl<'de, 'a, B: ByteOrder> SeqAccess<'de> for StructDeserializer<'a, 'de, B> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.0.probe_signature_bytes(")".as_bytes()) {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(self.0.subsidiary()?)?))
        }
    }
}

struct EnumDeserializer<'a, 'de: 'a, B: ByteOrder> {
    de: Deserializer<'a, 'de, B>,
    name: &'static str,
}

impl<'de, 'a, B: ByteOrder> EnumAccess<'de> for EnumDeserializer<'a, 'de, B> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de.subsidiary()?).map(|v| (v, self))
    }
}

impl<'de, 'a, B: ByteOrder> VariantAccess<'de> for EnumDeserializer<'a, 'de, B> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, self.name, &[], visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(self.de, self.name, fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::de::from_message;
    use crate::error::Result;
    use crate::ser::serialize;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use std::fmt::Debug;
    use test_log::test;

    fn round_trip<T: DeserializeOwned + Debug + Serialize + PartialEq>(val: T) -> Result<()> {
        let b = val;
        let message = serialize(&b)?;
        let b2 = from_message(&message)?;
        assert_eq!(b, b2);
        Ok(())
    }

    #[test]
    fn round_trip_bool() -> Result<()> {
        round_trip(true)
    }

    #[test]
    fn round_trip_int() -> Result<()> {
        round_trip(3)
    }

    #[test]
    fn round_trip_float() -> Result<()> {
        round_trip(3.14)
    }

    #[test]
    fn round_trip_some_string() -> Result<()> {
        round_trip(Some("string".to_owned()))
    }

    #[test]
    fn round_trip_none_string() -> Result<()> {
        let b: Option<String> = None;
        round_trip(b)
    }

    #[test]
    fn round_trip_unit() -> Result<()> {
        round_trip(())
    }

    #[test]
    fn round_trip_unit_struct() -> Result<()> {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Foo;

        let foo = Foo;
        round_trip(foo)
    }

    #[test]
    fn round_trip_array0() -> Result<()> {
        round_trip(vec![1, 3, 5, 6])
    }

    #[test]
    fn round_trip_struct0() -> Result<()> {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
        struct StructSerialize {
            pub a: String,
            pub b: f64,
        }

        let data = StructSerialize {
            a: "Hi".to_owned(),
            b: 0.2,
        };

        round_trip(data)
    }

    #[test]
    fn round_trip_struct1() -> Result<()> {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
        struct StructSerialize {
            pub a: String,
            pub b: f64,
            pub c: (String, f64),
        }

        let data = StructSerialize {
            a: "Hi".to_owned(),
            b: 0.2,
            c: ("Hello".to_owned(), 8.3),
        };

        round_trip(data)
    }

    #[test]
    fn tuple_vs_struct() -> Result<()> {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
        struct StructSerialize {
            pub a: String,
            pub b: f64,
            pub c: (String, f64),
        }

        let data_tuple = ("Hi", 0.2, ("Hello", 8.3));
        let mesg_tuple = serialize(data_tuple)?;
        let data: StructSerialize = from_message(&mesg_tuple)?;

        let data_good = StructSerialize {
            a: "Hi".to_owned(),
            b: 0.2,
            c: ("Hello".to_owned(), 8.3),
        };
        assert_eq!(data, data_good);

        Ok(())
    }
}
