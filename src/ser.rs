use crate::error::{Error, Result};
use crate::message::Message;

use serde::{ser, Serialize};
use std::mem::take;
use std::str::from_utf8;

mod internal;
mod message_builder;
pub mod serializer_policy;

use internal::*;
use serializer_policy::{DefaultSerializerPolicy, SerializerPolicy, StructSerializationStyle};

/// This is the entry point to the serializer. The default
/// serialization policy, [`DefaultSerializerPolicy`]
/// serializes tuples and tuple structs in "struct" style,
/// e.g. a tuple with two `i32`s and a `String` would serialize
/// as `(iis)`. Structs with named fields are serialized in dictionary
/// style, as `a{sv}` where the keys of the dictionary are the names
/// of the fields, and the values are the values of those fields,
/// wrapped in variants, which is DBus's mechanism for type erasure.
///
/// Instead, to always use struct-style serialization, as `zvariant`
/// does, you can use [`StronglyTypedSerializerPolicy`].
///
/// To make these decisions on a struct-by-struct basis, you can
/// create a custom implementation of the [`SerializerPolicy`] trait.
///
/// Currently, all arrays are serialized as `av`. This is a known
/// shortcoming and will be addressed in future versions.
///
/// [`DefaultSerializerPolicy`]: serializer_policy::DefaultSerializerPolicy
/// [`StronglyTypedSerializerPolicy`]: serializer_policy::StronglyTypedSerializerPolicy
/// [`SerializerPolicy`]: serializer_policy::SerializerPolicy
pub fn serialize_with_policy(
    value: impl Serialize,
    config: impl SerializerPolicy,
) -> Result<Message> {
    let internal_ser = ReadySerializer::new();
    let ser = Serializer {
        internal_ser,
        config,
    };
    let done_serializer = value.serialize(ser)?;
    done_serializer.complete()
}

/// This is a convenience function that simply calls [`serialize_with_policy`]
/// with the default policy.
///
/// [`serialize_with_policy`]: serialize_with_policy
pub fn serialize(value: impl Serialize) -> Result<Message> {
    serialize_with_policy(value, DefaultSerializerPolicy)
}

struct Serializer<T: SerializerPolicy> {
    internal_ser: ReadySerializer,
    config: T,
}

impl<C: SerializerPolicy> ser::Serializer for Serializer<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    type SerializeSeq = SerializeSeq<C>;
    type SerializeTuple = SerializeTuple<C>;
    type SerializeTupleStruct = SerializeTupleStruct<C>;
    type SerializeTupleVariant = SerializeTupleVariant<C>;
    type SerializeMap = SerializeMap<C>;
    type SerializeStruct = SerializeStruct<C>;
    type SerializeStructVariant = SerializeStructVariant<C>;

    fn serialize_bool(self, val: bool) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_i8(self, val: i8) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&(val as i16))
    }

    fn serialize_i16(self, val: i16) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_i32(self, val: i32) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_i64(self, val: i64) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_u8(self, val: u8) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_u16(self, val: u16) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_u32(self, val: u32) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_u64(self, val: u64) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_f32(self, val: f32) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&(val as f64))
    }

    fn serialize_f64(self, val: f64) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    fn serialize_char(self, val: char) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&(val as u32))
    }

    fn serialize_str(self, val: &str) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&val)
    }

    // TODO: This seems like the wrong thing to do. Come back later?
    fn serialize_bytes(self, val: &[u8]) -> Result<DoneSerializer> {
        self.internal_ser.serialize_primitive(&from_utf8(val)?)
    }

    fn serialize_none(self) -> Result<DoneSerializer> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, val: &T) -> Result<DoneSerializer>
    where
        T: Serialize + ?Sized,
    {
        val.serialize(self)
    }

    fn serialize_unit(self) -> Result<DoneSerializer> {
        let ser = self.internal_ser;
        let ser = ser.start_struct();
        let ser = ser.finish_struct();
        Ok(ser)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<DoneSerializer> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        variant_index: u32,
        _: &'static str,
    ) -> Result<DoneSerializer> {
        variant_index.serialize(self)
    }

    fn serialize_newtype_struct<T>(self, _: &'static str, value: &T) -> Result<DoneSerializer>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<DoneSerializer>
    where
        T: Serialize + ?Sized,
    {
        let ser = self.internal_ser;
        let ser = ser.start_dict();

        let (ser, item) = ser.start_item();
        let item = value.serialize(Serializer {
            internal_ser: item,
            config: self.config,
        })?;
        let ser = ser.finish_item(variant, item)?;

        let ser = ser.finish_dict();
        Ok(ser)
    }

    fn serialize_seq(self, _: std::option::Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeSeq {
            internal_ser: Some(self.internal_ser.start_array(vec![b'v'])),
            config: self.config,
        })
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        Ok(SerializeTuple {
            internal_ser: Some(self.internal_ser.start_struct()),
            config: self.config,
        })
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SerializeTupleStruct {
            internal_ser: Some(self.internal_ser.start_struct()),
            config: self.config,
        })
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        let parent_ser = self.internal_ser.start_dict();
        let (parent_ser, internal_ser) = parent_ser.start_item();
        let internal_ser = Some(internal_ser.start_struct());
        Ok(SerializeTupleVariant {
            internal_ser,
            parent_ser,
            name: variant,
            config: self.config,
        })
    }

    fn serialize_map(self, _: std::option::Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeMap {
            internal_ser: Some(self.internal_ser.start_array(vec![b'{', b's', b'v', b'}'])),
            inner_ser: None,
            pending_ser: None,
            config: self.config,
        })
    }

    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        let internal_ser = match self.config.query_struct_name(name) {
            StructSerializationStyle::Dict => {
                SerializeStructInternal::Dict(self.internal_ser.start_dict())
            }
            StructSerializationStyle::StronglyTyped => {
                SerializeStructInternal::Struct(self.internal_ser.start_struct())
            }
        };

        Ok(SerializeStruct {
            internal_ser: internal_ser,
            config: self.config,
        })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let parent_ser = self.internal_ser.start_dict();
        let (parent_ser, internal_ser) = parent_ser.start_item();
        let internal_ser = Some(internal_ser.start_dict());
        Ok(SerializeStructVariant {
            internal_ser,
            parent_ser,
            name: variant,
            config: self.config,
        })
    }
}

struct SerializeSeq<T: SerializerPolicy> {
    internal_ser: Option<ReadyArraySerializer>,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeSeq for SerializeSeq<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (internal_ser, item) = internal_ser.start_item();
        let (item, sub_item) = item.start_variant();
        let sub_item = value.serialize(Serializer {
            internal_ser: sub_item,
            config: self.config.clone(),
        })?;
        let item = item.finish_variant(sub_item);
        let internal_ser = internal_ser.finish_item(item)?;
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        Ok(self.internal_ser.expect("programming error").finish_array())
    }
}

struct SerializeTuple<T: SerializerPolicy> {
    internal_ser: Option<ReadyStructSerializer>,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeTuple for SerializeTuple<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (internal_ser, item) = internal_ser.start_item();
        let item = value.serialize(Serializer {
            internal_ser: item,
            config: self.config.clone(),
        })?;
        let internal_ser = internal_ser.finish_item(item);
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        Ok(self
            .internal_ser
            .expect("programming error")
            .finish_struct())
    }
}

struct SerializeTupleStruct<T: SerializerPolicy> {
    internal_ser: Option<ReadyStructSerializer>,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeTupleStruct for SerializeTupleStruct<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (internal_ser, item) = internal_ser.start_item();
        let item = value.serialize(Serializer {
            internal_ser: item,
            config: self.config.clone(),
        })?;
        let internal_ser = internal_ser.finish_item(item);
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        Ok(self
            .internal_ser
            .expect("programming error")
            .finish_struct())
    }
}

struct SerializeTupleVariant<T: SerializerPolicy> {
    parent_ser: PendingDictSerializer,
    internal_ser: Option<ReadyStructSerializer>,
    name: &'static str,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeTupleVariant for SerializeTupleVariant<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (internal_ser, item) = internal_ser.start_item();
        let item = value.serialize(Serializer {
            internal_ser: item,
            config: self.config.clone(),
        })?;
        let internal_ser = internal_ser.finish_item(item);
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        let SerializeTupleVariant {
            parent_ser,
            internal_ser,
            name,
            config: _,
        } = self;
        let internal_ser = internal_ser.expect("programming error");
        let internal_ser = internal_ser.finish_struct();
        let parent_ser = parent_ser.finish_optional_item(name, internal_ser)?;
        let parent_ser = parent_ser.finish_dict();
        Ok(parent_ser)
    }
}

struct SerializeMap<T: SerializerPolicy> {
    internal_ser: Option<ReadyArraySerializer>,
    pending_ser: Option<PendingArraySerializer>,
    inner_ser: Option<ReadyStructSerializer>,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeMap for SerializeMap<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_key<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (pending_ser, item) = internal_ser.start_item();
        let item = item.start_kv_pair();
        let (item, key_item) = item.start_item();
        let key_item = value.serialize(Serializer {
            internal_ser: key_item,
            config: self.config.clone(),
        })?;
        let item = item.finish_item(key_item);
        self.pending_ser = Some(pending_ser);
        self.inner_ser = Some(item);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let pending_ser = self.pending_ser.take().expect("programming error");
        let inner_ser = self.inner_ser.take().expect("programming error");
        let (inner_ser, inner_inner_ser) = inner_ser.start_item();
        let (inner_inner_ser, inner_inner_inner_ser) = inner_inner_ser.start_variant();
        let inner_inner_inner_ser = value.serialize(Serializer {
            internal_ser: inner_inner_inner_ser,
            config: self.config.clone(),
        })?;
        let inner_inner_ser = inner_inner_ser.finish_variant(inner_inner_inner_ser);
        let inner_ser = inner_ser.finish_item(inner_inner_ser);
        let inner_ser = inner_ser.finish_kv_pair();
        let internal_ser = pending_ser.finish_item(inner_ser)?;
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        let internal_ser = self.internal_ser.expect("programming error");
        Ok(internal_ser.finish_array())
    }
}

enum SerializeStructInternal {
    Dict(ReadyDictSerializer),
    Struct(ReadyStructSerializer),
    Empty,
}

impl Default for SerializeStructInternal {
    fn default() -> Self {
        Self::Empty
    }
}

struct SerializeStruct<T: SerializerPolicy> {
    internal_ser: SerializeStructInternal,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeStruct for SerializeStruct<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_field<T>(&mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = take(&mut self.internal_ser);
        match internal_ser {
            SerializeStructInternal::Dict(internal_ser) => {
                let (internal_ser, item) = internal_ser.start_item();
                let item = value.serialize(Serializer {
                    internal_ser: item,
                    config: self.config.clone(),
                })?;
                let internal_ser = internal_ser.finish_optional_item(name, item)?;
                self.internal_ser = SerializeStructInternal::Dict(internal_ser);
            }
            SerializeStructInternal::Struct(internal_ser) => {
                let (internal_ser, item) = internal_ser.start_item();
                let item = value.serialize(Serializer {
                    internal_ser: item,
                    config: self.config.clone(),
                })?;
                let internal_ser = internal_ser.finish_item(item);
                self.internal_ser = SerializeStructInternal::Struct(internal_ser);
            }
            SerializeStructInternal::Empty => {
                unreachable!() // Because it's always put back at the end
            }
        }
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        match self.internal_ser {
            SerializeStructInternal::Dict(internal_ser) => Ok(internal_ser.finish_dict()),
            SerializeStructInternal::Struct(internal_ser) => Ok(internal_ser.finish_struct()),
            SerializeStructInternal::Empty => {
                unreachable!() // Never left in this state except in panic
            }
        }
    }
}

struct SerializeStructVariant<T: SerializerPolicy> {
    internal_ser: Option<ReadyDictSerializer>,
    parent_ser: PendingDictSerializer,
    name: &'static str,
    config: T,
}

impl<C: SerializerPolicy> ser::SerializeStructVariant for SerializeStructVariant<C> {
    type Ok = DoneSerializer;
    type Error = Error;

    fn serialize_field<T>(&mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let internal_ser = self.internal_ser.take().expect("programming error");
        let (internal_ser, item) = internal_ser.start_item();
        let item = value.serialize(Serializer {
            internal_ser: item,
            config: self.config.clone(),
        })?;
        let internal_ser = internal_ser.finish_optional_item(name, item)?;
        self.internal_ser = Some(internal_ser);
        Ok(())
    }

    fn end(self) -> Result<DoneSerializer> {
        let SerializeStructVariant {
            parent_ser,
            internal_ser,
            name,
            config: _,
        } = self;
        let internal_ser = internal_ser.expect("programming error");
        let internal_ser = internal_ser.finish_dict();
        let parent_ser = parent_ser.finish_optional_item(name, internal_ser)?;
        let parent_ser = parent_ser.finish_dict();
        Ok(parent_ser)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::message::Message;
    use crate::ser::serialize;
    use crate::ser::serialize_with_policy;
    use crate::ser::serializer_policy::StronglyTypedSerializerPolicy;
    use serde::Serialize;
    use test_log::test;

    #[test]
    fn serialize_int() -> Result<()> {
        let i = 37i32;
        let message = serialize(&i)?;
        let correct_message = Message {
            data: vec![37, 0, 0, 0],
            signature: "i".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "i32 message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_tuple() -> Result<()> {
        let data = ("Hi", 0.2f64, ("Hello", 8.3f64));
        let message = serialize(&data)?;
        let correct_message = Message {
            data: vec![
                2u8, 0u8, 0u8, 0u8, 72u8, 105u8, 0u8, 0u8, 154u8, 153u8, 153u8, 153u8, 153u8,
                153u8, 201u8, 63u8, 5u8, 0u8, 0u8, 0u8, 72u8, 101u8, 108u8, 108u8, 111u8, 0u8, 0u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 154u8, 153u8, 153u8, 153u8, 153u8, 153u8, 32u8, 64u8,
            ],
            signature: "(sd(sd))".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "struct message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_struct() -> Result<()> {
        #[derive(Clone, Debug, Serialize)]
        struct StructSerialize {
            pub a: String,
            pub b: f64,
            pub c: (String, f64),
        }

        let data = ("Hi", 0.2f64, ("Hello", 8.3f64));
        let message = serialize_with_policy(&data, StronglyTypedSerializerPolicy)?;
        let correct_message = Message {
            data: vec![
                2u8, 0u8, 0u8, 0u8, 72u8, 105u8, 0u8, 0u8, 154u8, 153u8, 153u8, 153u8, 153u8,
                153u8, 201u8, 63u8, 5u8, 0u8, 0u8, 0u8, 72u8, 101u8, 108u8, 108u8, 111u8, 0u8, 0u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 154u8, 153u8, 153u8, 153u8, 153u8, 153u8, 32u8, 64u8,
            ],
            signature: "(sd(sd))".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "struct message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_dict() -> Result<()> {
        #[derive(Clone, Debug, Serialize)]
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

        let message = serialize(&data)?;
        let correct_message = Message {
            data: vec![
                88u8, 0u8, 0u8, 0u8, // 88 bytes of array
                0u8, 0u8, 0u8, 0u8, // padding(8)
                1u8, 0u8, 0u8, 0u8, // 1 byte string
                97u8, 0u8, // "a"
                1u8, // 1 byte signature
                115u8, 0u8, // 's'
                0u8, 0u8, 0u8, // padding(4)
                2u8, 0u8, 0u8, 0u8, // 2 byte string
                72u8, 105u8, 0u8, // "Hi"
                0u8, 0u8, 0u8, 0u8, 0u8, // padding(8)
                1u8, 0u8, 0u8, 0u8, // 1 byte string
                98u8, 0u8, // "b"
                1u8, // 1 byte signature
                100u8, 0u8, // "d"
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // padding(8)
                154u8, 153u8, 153u8, 153u8, 153u8, 153u8, 201u8, 63u8, // double 0.2
                1u8, 0u8, 0u8, 0u8, // 1 byte string
                99u8, 0u8, // "c"
                4u8, // 4 byte signature
                40u8, 115u8, 100u8, 41u8, 0u8, // "(sd)"
                0u8, 0u8, 0u8, 0u8, // padding(8)
                5u8, 0u8, 0u8, 0u8, // 5 byte string
                72u8, 101u8, 108u8, 108u8, 111u8, 0u8, // "Hello"
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // padding(8)
                154u8, 153u8, 153u8, 153u8, 153u8, 153u8, 32u8, 64u8, // double 8.3
            ],
            signature: "a{sv}".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "dict message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_dict_simpler() -> Result<()> {
        #[derive(Clone, Debug, Serialize)]
        struct StructSerialize {
            pub a: String,
            pub b: f64,
        }

        let data = StructSerialize {
            a: "Hi".to_owned(),
            b: 0.2,
        };

        let message = serialize(&data)?;
        let correct_message = Message {
            data: vec![
                48, 0, 0, 0, // 48 bytes of array
                0, 0, 0, 0, // pad to 8 to start kv pair
                // 48 bytes start here
                1, 0, 0, 0, // key string is 1 byte
                97, 0, // 'a' with terminating null
                1, // value signature is 1 byte
                115, 0, // 's' for string with terminating null
                0, 0, 0, // padding to begin string length
                2, 0, 0, 0, // string in question is 2 bytes
                72, 105, 0, // "Hi" plus terminating null
                0, 0, 0, 0, 0, // pad to 8 to start kv pair
                1, 0, 0, 0, // key string is 1 byte
                98, 0, // 'b' with terminating null
                1, // signature is 1 byte
                100, 0, // 'd' with terminating null
                0, 0, 0, 0, 0, 0, 0, // pad to 8 for double value
                154, 153, 153, 153, 153, 153, 201, 63, // 0.2
            ],
            signature: "a{sv}".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "dict message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn tuple_vs_struct() -> Result<()> {
        #[derive(Clone, Debug, Serialize, PartialEq)]
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
        let data_tuple = ("Hi", 0.2, ("Hello", 8.3));

        let mesg = serialize_with_policy(data, StronglyTypedSerializerPolicy);
        let mesg_tuple = serialize(data_tuple);
        assert_eq!(mesg, mesg_tuple);
        Ok(())
    }

    #[test]
    fn optional_fields() -> Result<()> {
        #[derive(Clone, Debug, Serialize)]
        struct WithOptionalField {
            a: String,
            b: Option<String>,
            c: String,
        }

        #[derive(Clone, Debug, Serialize)]
        struct WithoutOptionalField {
            a: String,
            c: String,
        }

        let data_with = WithOptionalField {
            a: "a".to_owned(),
            b: None,
            c: "c".to_owned(),
        };
        let data_without = WithoutOptionalField {
            a: "a".to_owned(),
            c: "c".to_owned(),
        };

        let mesg_with = serialize(data_with);
        let mesg_without = serialize(data_without);
        assert_eq!(mesg_with, mesg_without);
        Ok(())
    }

    #[test]
    fn optional_fields2() -> Result<()> {
        #[derive(Clone, Debug, Serialize)]
        struct WithOptionalField {
            a: String,
            b: Option<String>,
            c: String,
        }

        #[derive(Clone, Debug, Serialize)]
        struct WithoutOptionalField {
            a: String,
            c: String,
        }

        let data_with = WithOptionalField {
            a: "a".to_owned(),
            b: None,
            c: "c".to_owned(),
        };
        let data_without = WithoutOptionalField {
            a: "a".to_owned(),
            c: "c".to_owned(),
        };

        let mesg_with = serialize_with_policy(data_with, StronglyTypedSerializerPolicy);
        let mesg_without = serialize_with_policy(data_without, StronglyTypedSerializerPolicy);
        assert_ne!(mesg_with, mesg_without);
        Ok(())
    }
}
