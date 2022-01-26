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
