// mod de;
mod error;
// mod ser;

// pub use de::{from_msg, from_prop_map, Deserializer};
pub use error::{Error, Result};
// pub use ser::{to_msg, to_prop_map, Serializer};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
