#[cfg(feature = "zbus")]
use crate::error::{Error, Result};

#[cfg(feature = "zbus")]
use zbus::{Message as ZbusMessage, MessageBuilder as ZbusMessageBuilder};

#[cfg(feature = "zbus")]
use std::convert::TryFrom;

/// This is the message type that is used for serialization
/// and deserialization at this time. We will soon also support
/// serializing and deserializing to `zbus` messages in this crate,
/// by making this crate depend on `zbus`. Eventually, deeper
/// integration will be sought out.
///
/// Note that signatures in this message type are bracketed with `(`
/// and `)` at the top level if the top level is serialized struct
/// style. These brackets will need to be removed before actually
/// sending over DBus. This will be handled by the conversions to
/// zbus messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
}

impl Message {
    #[cfg(feature = "zbus")]
    pub fn as_zbus_message(&self, builder: ZbusMessageBuilder) -> Result<ZbusMessage> {
        Ok(builder.build_raw_body(&self.data, &self.signature, vec![])?)
    }
}

#[cfg(feature = "zbus")]
impl TryFrom<&ZbusMessage> for Message {
    type Error = Error;

    fn try_from(value: &ZbusMessage) -> Result<Self> {
        let data = value.body_as_bytes()?.to_vec();
        let signature = value.body_signature()?.as_bytes().to_vec();
        Ok(Message { data, signature })
    }
}
