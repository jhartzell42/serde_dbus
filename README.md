# Serde DBUS

The goal of this crate is to provide `serde` support for the DBus message format.
Actually sending the messages is outside of the scope of this crate, and we plan
on modifying `zbus` slightly to allow it to be used with this crate instead of
its bundled serializer, `zvariant`.

The main difference between this crate and `zvariant` is that this crate:

 * Will not require `Type` instances to specify the signatures, rather it will build
   signatures dynamically from the types provided in the structures Serialize instances.

 * Will support serializing structures as either dictionaries of variants (`a{sv}`) or
   as structures (e.g. `(iis)`), with either a global flag or configurable on a per-struct name
   basis.

Performance is not a significant consideration for now, prioritizing instead getting a working
version out first, and then subsequently cutting down on allocations. Serializing will only be
supported in little endian, but deserializing will have to support either endianness.

# Implementation

The implementation will consist of a private, internal API that will support serializing and
deserializing DBus messages in a way specific to DBus. This will then be wrapped with a serializer
and a deserializer conforming directly to the `serde` traits. There are multiple valid ways to
serialize structures in DBUS, and so the serializer will be configurable which ways to use in
which situation, while the deserializer will support all ways.
