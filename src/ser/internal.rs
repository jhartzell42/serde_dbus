use crate::error::{Error, Result};
use crate::message::Message;
use crate::primitives::DbusPrimitive;

use super::message_builder::PendingMessage;

use std::mem::swap;

pub(super) struct ReadySerializer {
    mesg: PendingMessage,
}

pub(super) struct DoneSerializer {
    mesg: PendingMessage,
}

impl DoneSerializer {
    fn extract(self) -> PendingMessage {
        self.mesg
    }

    pub(super) fn complete(self) -> Result<Message> {
        Ok(Message {
            data: self.mesg.builder.complete(),
            signature: self.mesg.signature,
        })
    }
}

impl ReadySerializer {
    pub(super) fn new() -> Self {
        Self {
            mesg: PendingMessage::new(),
        }
    }

    pub(super) fn serialize_primitive<T: DbusPrimitive>(
        self,
        primitive: &T,
    ) -> Result<DoneSerializer> {
        let mut mesg = self.mesg;

        mesg.builder.align(T::alignment());
        primitive.serialize(mesg.builder.prepare_write(primitive.size()))?;
        mesg.signature.push(T::signature());

        Ok(DoneSerializer { mesg })
    }

    pub(super) fn start_struct(self) -> ReadyStructSerializer {
        ReadyStructSerializer::new(self.mesg)
    }

    pub(super) fn start_kv_pair(self) -> ReadyStructSerializer {
        ReadyStructSerializer::new_kv_pair(self.mesg)
    }

    pub(super) fn start_array(self, item_sig: Vec<u8>) -> ReadyArraySerializer {
        ReadyArraySerializer::new(self.mesg, item_sig)
    }

    pub(super) fn start_dict(self) -> ReadyDictSerializer {
        ReadyDictSerializer {
            ser: self.start_array(vec![b'{', b's', b'v', b'}']),
        }
    }

    pub(super) fn start_variant(self) -> (VariantSerializer, ReadySerializer) {
        (
            VariantSerializer { mesg: self.mesg },
            ReadySerializer::new(),
        )
    }
}

impl Default for ReadySerializer {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) struct VariantSerializer {
    mesg: PendingMessage,
}

impl VariantSerializer {
    pub(super) fn finish_variant(self, value: DoneSerializer) -> DoneSerializer {
        let mut mesg = self.mesg;
        let value_mesg = value.extract();

        let signature_len = value_mesg.signature.len();

        // signature size
        mesg.builder.prepare_write(1)[0] = signature_len as u8;
        let signature_store = mesg.builder.prepare_write(signature_len);

        // signature
        signature_store.copy_from_slice(&value_mesg.signature);

        // null terminator for signature
        mesg.builder.prepare_write(1)[0] = 0u8;

        // value (inherits alignment)
        mesg.builder.append_data(&value_mesg.builder);

        // add variant signature to mesg
        mesg.signature.push(b'v');

        DoneSerializer { mesg }
    }
}

pub(super) struct ReadyStructSerializer {
    mesg: PendingMessage,
}

impl ReadyStructSerializer {
    fn new(mut mesg: PendingMessage) -> Self {
        mesg.builder.align(8);
        mesg.signature.push(b'(');
        ReadyStructSerializer { mesg }
    }

    fn new_kv_pair(mut mesg: PendingMessage) -> Self {
        mesg.builder.align(8);
        mesg.signature.push(b'{');
        ReadyStructSerializer { mesg }
    }

    pub(super) fn start_item(self) -> (PendingStructSerializer, ReadySerializer) {
        (PendingStructSerializer, ReadySerializer { mesg: self.mesg })
    }

    pub(super) fn finish_struct(self) -> DoneSerializer {
        let mut mesg = self.mesg;
        mesg.signature.push(b')');
        DoneSerializer { mesg }
    }

    pub(super) fn finish_kv_pair(self) -> DoneSerializer {
        let mut mesg = self.mesg;
        mesg.signature.push(b'}');
        DoneSerializer { mesg }
    }
}

pub(super) struct PendingStructSerializer;

impl PendingStructSerializer {
    pub(super) fn finish_item(self, item: DoneSerializer) -> ReadyStructSerializer {
        ReadyStructSerializer {
            mesg: item.extract(),
        }
    }
}

pub(super) struct ReadyArraySerializer {
    prev: PendingMessage,

    // contents has all deserialized data from inside the array
    // the signature is to be kept empty
    contents: PendingMessage,
    item_sig: Vec<u8>,
}

impl ReadyArraySerializer {
    fn new(mesg: PendingMessage, item_sig: Vec<u8>) -> Self {
        Self {
            prev: mesg,
            contents: PendingMessage::new(),
            item_sig,
        }
    }

    pub(super) fn start_item(self) -> (PendingArraySerializer, ReadySerializer) {
        let pending = PendingArraySerializer {
            prev: self.prev,
            item_sig: self.item_sig,
        };
        let ready = ReadySerializer {
            mesg: self.contents,
        };
        (pending, ready)
    }

    pub(super) fn finish_array(self) -> DoneSerializer {
        let Self {
            prev: mut mesg,
            contents,
            mut item_sig,
        } = self;

        // Get signature correct
        mesg.signature.push(b'a');
        mesg.signature.append(&mut item_sig);

        mesg.builder.align(4);
        let token = mesg.builder.start_length();
        mesg.builder.append_data(&contents.builder);
        mesg.builder.finish_length(token);

        DoneSerializer { mesg }
    }
}

pub(super) struct PendingArraySerializer {
    prev: PendingMessage,
    item_sig: Vec<u8>,
}

impl PendingArraySerializer {
    pub(super) fn finish_item(self, item: DoneSerializer) -> Result<ReadyArraySerializer> {
        let mut children_mesg = item.extract();
        let mut sig = Vec::new();
        swap(&mut sig, &mut children_mesg.signature);
        let item_sig = self.item_sig;
        if item_sig != sig {
            return Err(Error::MismatchSignature(item_sig, sig));
        }
        Ok(ReadyArraySerializer {
            prev: self.prev,
            contents: children_mesg,
            item_sig: item_sig,
        })
    }
}

// For standard dicts-of-variants i.e. a{sv}, with cancellation and optional dropping of {}
pub(super) struct ReadyDictSerializer {
    ser: ReadyArraySerializer,
}

impl ReadyDictSerializer {
    pub(super) fn start_item(self) -> (PendingDictSerializer, ReadySerializer) {
        let dict = PendingDictSerializer { ser: self.ser };
        let ready = ReadySerializer::new();
        (dict, ready)
    }

    pub(super) fn finish_dict(self) -> DoneSerializer {
        self.ser.finish_array()
    }
}

pub(super) struct PendingDictSerializer {
    ser: ReadyArraySerializer,
}

impl PendingDictSerializer {
    pub(super) fn cancel_item(self) -> ReadyDictSerializer {
        ReadyDictSerializer { ser: self.ser }
    }

    pub(super) fn finish_optional_item(
        self,
        name: &str,
        value: DoneSerializer,
    ) -> Result<ReadyDictSerializer> {
        if &value.mesg.signature == &[b'(', b')'] {
            Ok(self.cancel_item())
        } else {
            self.finish_item(name, value)
        }
    }

    pub(super) fn finish_item(
        self,
        name: &str,
        value: DoneSerializer,
    ) -> Result<ReadyDictSerializer> {
        let ser = self.ser;
        let (ser, kv_ser) = ser.start_item();
        let kv_ser = kv_ser.start_kv_pair();
        let (kv_ser, str_ser) = kv_ser.start_item();
        let str_ser = str_ser.serialize_primitive(&name)?;
        let kv_ser = kv_ser.finish_item(str_ser);
        let (kv_ser, var_ser) = kv_ser.start_item();
        let (var_ser, _) = var_ser.start_variant();
        let var_ser = var_ser.finish_variant(value); // This relies on the implementation detail that variant DoneSerializers are interchangeable
        let kv_ser = kv_ser.finish_item(var_ser);
        let kv_ser = kv_ser.finish_kv_pair();
        let ser = ser.finish_item(kv_ser)?;
        Ok(ReadyDictSerializer { ser })
    }
}

#[cfg(test)]
mod tests {
    use super::ReadySerializer;
    use crate::error::Result;
    use crate::message::Message;

    #[test]
    fn serialize_int() -> Result<()> {
        let i = 37i32;
        let serializer = ReadySerializer::new();
        let serializer = serializer.serialize_primitive(&i)?;
        let message = serializer.complete()?;

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
    fn serialize_variant_int() -> Result<()> {
        let i = 37i32;
        let serializer = ReadySerializer::new();
        let (serializer, sub_serializer) = serializer.start_variant();
        let sub_serializer = sub_serializer.serialize_primitive(&i)?;
        let serializer = serializer.finish_variant(sub_serializer);
        let message = serializer.complete()?;

        let correct_message = Message {
            data: vec![1, 105, 0, 0, 37, 0, 0, 0],
            signature: "v".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "variant of i32 message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_variant_farray() -> Result<()> {
        let top_level_serializer = ReadySerializer::new();
        let (top_level_serializer, serializer) = top_level_serializer.start_variant();
        let serializer = serializer.start_array("d".as_bytes().to_vec());

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&1.0)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&2.0)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&3.0)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&4.0)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let serializer = serializer.finish_array();
        let top_level_serializer = top_level_serializer.finish_variant(serializer);
        let message = top_level_serializer.complete()?;

        let correct_message = Message {
            data: vec![
                2, 97, 100, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 64, 0,
                0, 0, 0, 0, 0, 8, 64, 0, 0, 0, 0, 0, 0, 16, 64,
            ],
            signature: "v".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "array message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_intary() -> Result<()> {
        let serializer = ReadySerializer::new();
        let serializer = serializer.start_array("i".as_bytes().to_vec());

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&1)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&2)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&3)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&4)?;
        let serializer = serializer.finish_item(sub_serializer)?;

        let serializer = serializer.finish_array();
        let message = serializer.complete()?;

        let correct_message = Message {
            data: vec![
                16u8, 0u8, 0u8, 0u8, 1u8, 0u8, 0u8, 0u8, 2u8, 0u8, 0u8, 0u8, 3u8, 0u8, 0u8, 0u8,
                4u8, 0u8, 0u8, 0u8,
            ],
            signature: "ai".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "array message serialized incorrectly"
        );
        Ok(())
    }

    #[test]
    fn serialize_struct() -> Result<()> {
        let serializer = ReadySerializer::new();
        let serializer = serializer.start_struct();

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&"Hi")?;
        let serializer = serializer.finish_item(sub_serializer);

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&0.2f64)?;
        let serializer = serializer.finish_item(sub_serializer);

        let (serializer, sub_serializer) = serializer.start_item();

        let sub_serializer = sub_serializer.start_struct();

        let (sub_serializer, sub_sub_serializer) = sub_serializer.start_item();
        let sub_sub_serializer = sub_sub_serializer.serialize_primitive(&"Hello")?;
        let sub_serializer = sub_serializer.finish_item(sub_sub_serializer);

        let (sub_serializer, sub_sub_serializer) = sub_serializer.start_item();
        let sub_sub_serializer = sub_sub_serializer.serialize_primitive(&8.3f64)?;
        let sub_serializer = sub_serializer.finish_item(sub_sub_serializer);

        let sub_serializer = sub_serializer.finish_struct();
        let serializer = serializer.finish_item(sub_serializer);

        let serializer = serializer.finish_struct();
        let message = serializer.complete()?;

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
        let serializer = ReadySerializer::new();
        let serializer = serializer.start_dict();

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&"Hi")?;
        let serializer = serializer.finish_item("a", sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();
        let sub_serializer = sub_serializer.serialize_primitive(&0.2f64)?;
        let serializer = serializer.finish_item("b", sub_serializer)?;

        let (serializer, sub_serializer) = serializer.start_item();

        let sub_serializer = sub_serializer.start_struct();

        let (sub_serializer, sub_sub_serializer) = sub_serializer.start_item();
        let sub_sub_serializer = sub_sub_serializer.serialize_primitive(&"Hello")?;
        let sub_serializer = sub_serializer.finish_item(sub_sub_serializer);

        let (sub_serializer, sub_sub_serializer) = sub_serializer.start_item();
        let sub_sub_serializer = sub_sub_serializer.serialize_primitive(&8.3f64)?;
        let sub_serializer = sub_serializer.finish_item(sub_sub_serializer);

        let sub_serializer = sub_serializer.finish_struct();
        let serializer = serializer.finish_item("c", sub_serializer)?;

        let (serializer, item) = serializer.start_item();
        let item = item.start_struct();
        let item = item.finish_struct();
        let serializer = serializer.finish_optional_item("d", item)?;

        let serializer = serializer.finish_dict();
        let message = serializer.complete()?;

        let correct_message = Message {
            data: vec![
                88u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 0u8, 0u8, 0u8, 97u8, 0u8, 1u8, 115u8,
                0u8, 0u8, 0u8, 0u8, 2u8, 0u8, 0u8, 0u8, 72u8, 105u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                1u8, 0u8, 0u8, 0u8, 98u8, 0u8, 1u8, 100u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                154u8, 153u8, 153u8, 153u8, 153u8, 153u8, 201u8, 63u8, 1u8, 0u8, 0u8, 0u8, 99u8,
                0u8, 4u8, 40u8, 115u8, 100u8, 41u8, 0u8, 0u8, 0u8, 0u8, 0u8, 5u8, 0u8, 0u8, 0u8,
                72u8, 101u8, 108u8, 108u8, 111u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 154u8, 153u8,
                153u8, 153u8, 153u8, 153u8, 32u8, 64u8,
            ],
            signature: "a{sv}".as_bytes().to_vec(),
        };
        assert_eq!(
            correct_message, message,
            "dict message serialized incorrectly"
        );
        Ok(())
    }
}
