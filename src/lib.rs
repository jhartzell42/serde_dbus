//! A serde implementation for DBus.
//!
//! This crate is very freshly written and it is very much so "in development"
//! and a work in progress. It does not yet support all the features of
//! `dbus`. Please test thoroughly before using, and please feel free to
//! contribute bug reports or MRs.
//!
//! The goal of this crate is to provide `serde` support for the DBus
//! message format. Actually sending the messages is outside of the scope
//! of this crate.  This crate can be used with a more complete messaging
//! system like `zbus`, as an alternative to its bundled `zvariant`
//! `serde`-implementation. It is, to be clear, a fresh re-implementation
//! of `serde` for the DBus format, and not a fork of `zvariant`.
//!
//! This is intended to be used with garden-variety `Serialize`
//! and `Deserialize` instances from [serde] to implement just the
//! serialization and deserialization to and from [DBus] messages.
//! The intention is for this to be integrated as a serialization
//! option in [zbus], which provides full support for the DBus
//! protocol. Until it is integrated with a full DBus library
//! like [zbus] -- which I plan on doing very soon --
//! this crate is only useful as a proof-of-concept.
//!
//! Messages are in the [`Message`] type, and currently
//! that is the only format supported for serialization and
//! deserialization directly. Conversion is available into
//! [zbus] messages if the `zbus` feature is enabled.
//!
//! Deserialization is handled by the [`de`] module, but
//! really boils down to the [`from_message`] function,
//! as deserialization cannot be substantially customized.
//!
//! Serialization, on the other hand, is customizable.
//! The [`ser`] module exposes basic serialization
//! in the [`serialize`] function. However,
//! sometimes DBus allows for the same data to be serialized
//! in different ways, and this can be configured via
//! the [`serializer_policy`] module and by instead calling
//! [`serialize_with_policy`].
//!
//! [serde]: https://serde.rs
//! [DBus]: https://www.freedesktop.org/wiki/Software/dbus/
//! [zbus]: https://crates.io/crates/zbus
//! [`Message`]: crate::message::Message
//! [`de`]: crate::de
//! [`from_message`]: crate::de::from_message()
//! [`ser`]: crate::ser
//! [`serialize`]: crate::ser::serialize()
//! [`serializer_policy`]: crate::ser::serializer_policy
//! [`serialize_with_policy`]: crate::ser::serialize_with_policy()

mod align;
pub mod de;
pub mod error;
pub mod message;
mod primitives;
pub mod ser;
