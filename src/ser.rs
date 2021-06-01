use serde::{ser, Serialize};
use dbus::arg::{Variant, PropMap};

use error::{Error, Result};

use std::collections::HashMap;

pub struct Serializer;

pub fn to_prop_map<T>(value: &T) -> Result<PropMap>
where
    T: Serialize,
{
    let mut serializer = Serializer;
    value.serialize(&mut serializer)?
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = Variant;
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_i8(self, v: i8) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }


    fn serialize_i32(self, v: i32) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }


    fn serialize_i64(self, v: i64) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }


    fn serialize_u32(self, v: u32) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }


    fn serialize_u64(self, v: u64) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }


    fn serialize_f64(self, v: f64) -> Result<Variant> {
        Ok(Variant(Box::new(v)))
    }

    fn serialize_char(self, v: char) -> Result<Variant> {
        self.serialize_str(&v.to_str())
    }

    fn serialize_str(self, v: &str) -> Result<Variant> {
        Ok(Variant(Box::new(String::from(v))))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        panic!("Not yet implemented");
    }
}
