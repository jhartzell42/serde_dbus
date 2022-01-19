use crate::error::Result;

use std::mem::size_of;

pub(crate) trait DbusPrimitive {
    fn signature() -> u8;
    fn alignment() -> usize;
    fn size(&self) -> usize;
    fn serialize(&self, out: &mut [u8]) -> Result<()>;
}

macro_rules! basic_primitive {
    ($type:ident, $sig:expr) => {
        impl DbusPrimitive for $type {
            fn signature() -> u8 {
                $sig as u8
            }

            fn size(&self) -> usize {
                size_of::<$type>()
            }

            fn serialize(&self, out: &mut [u8]) -> Result<()> {
                out.copy_from_slice(&self.to_le_bytes());
                Ok(())
            }

            fn alignment() -> usize {
                size_of::<$type>()
            }
        }
    };
}

basic_primitive!(u8, 'y');
basic_primitive!(f64, 'd');
basic_primitive!(i16, 'n');
basic_primitive!(u16, 'q');
basic_primitive!(i32, 'i');
basic_primitive!(u32, 'u');
basic_primitive!(i64, 'x');
basic_primitive!(u64, 't');

impl DbusPrimitive for bool {
    fn signature() -> u8 {
        b'b'
    }

    fn size(&self) -> usize {
        4
    }

    fn serialize(&self, out: &mut [u8]) -> Result<()> {
        out.copy_from_slice(&(*self as u32).to_le_bytes());
        Ok(())
    }

    fn alignment() -> usize {
        4
    }
}

impl DbusPrimitive for &str {
    fn signature() -> u8 {
        b's'
    }

    fn size(&self) -> usize {
        self.as_bytes().len() + 5 // size and terminating null
    }

    fn serialize(&self, out: &mut [u8]) -> Result<()> {
        let bytes = self.as_bytes();
        out[0..4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        out[4..4 + bytes.len()].copy_from_slice(bytes);
        out[4 + bytes.len()] = 0u8;
        Ok(())
    }

    fn alignment() -> usize {
        4
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signature(pub String);

impl DbusPrimitive for Signature {
    fn signature() -> u8 {
        b'g'
    }

    fn size(&self) -> usize {
        self.0.as_bytes().len() + 2 // size and terminating null
    }

    fn serialize(&self, out: &mut [u8]) -> Result<()> {
        let bytes = self.0.as_bytes();
        out[0..1].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        out[1..1 + bytes.len()].copy_from_slice(bytes);
        out[1 + bytes.len()] = 0u8;
        Ok(())
    }

    fn alignment() -> usize {
        1
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPath(pub String);

impl DbusPrimitive for ObjectPath {
    fn signature() -> u8 {
        b'o'
    }

    fn size(&self) -> usize {
        self.0.as_bytes().len() + 5 // size and terminating null
    }

    fn serialize(&self, out: &mut [u8]) -> Result<()> {
        let bytes = self.0.as_bytes();
        out[0..4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        out[4..4 + bytes.len()].copy_from_slice(bytes);
        out[4 + bytes.len()] = 0u8;
        Ok(())
    }

    fn alignment() -> usize {
        4
    }
}
