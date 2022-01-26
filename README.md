# Serde DBUS

This crate is very freshly written and it is very much so "in development"
and a work in progress. It does not yet support all the features of
`dbus`. Please test thoroughly before using, and please feel free to
contribute bug reports or MRs.

The goal of this crate is to provide `serde` support for the DBus
message format. Actually sending the messages is outside of the scope
of this crate.  This crate can be used with a more complete messaging
system like `zbus`, as an alternative to its bundled `zvariant`
`serde`-implementation. It is, to be clear, a fresh re-implementation of
`serde` for the DBus format, and not a fork of `zvariant`.

The primary difference between this crate and `zvariant` is that
this crate intends to support serializing Rust structures as either
DBus structures (e.g. with a signature like `(iis)`) or DBus
dictionaries (with a signature of `a{sv}`) as equal first-class
options. `zvariant` uses special structs with special known names
to accomplish this, but `serde_dbus` instead can be configured
with either an overall default (such as all structs with named
fields become dicts whereas all tuple structs become structs),
or a case-by-case basis with configurable lists of struct names.

This means that the same `Serialize` instance can be used to
create DBus dicts as JSON dicts, which is an issue with `zvariant`
that this crate exists to solve.

As a side effect of the reimplementation, instead of passing
the signature of the entire structure as out-of-band data, as
in `zvariant`, this crate requires use of `SerializerPolicy` to
provide necessary out-of-band data. Due to constraints coming from
`serde`, especially this bug which `serde`
[won't fix](https://github.com/serde-rs/serde/issues/607),
use of `SerializerPolicy` will be necessary to provide item signatures
for arrays (otherwise they will default to use variants and be
`av`).

As of right now, this hasn't been implemented, and so arrays are currently
only supported for the `av` signature. This is a known issue, and will
be addressed in future versions.

The deserializer does not require any out of band information in any
circumstance.

Performance has so far not been a substantial consideration,
instead getting a working version out first, and then subsequently addressing
performance.

Serializing will only be supported in little endian, but deserializing
will have to support either endianness.
