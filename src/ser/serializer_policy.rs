pub enum StructSerializationStyle {
    StronglyTyped,
    Dict,
}

pub trait SerializerPolicy: Clone {
    fn query_struct_name(&self, name: &str) -> StructSerializationStyle;
}

#[derive(Clone, Debug)]
pub struct DefaultSerializerPolicy;

impl SerializerPolicy for DefaultSerializerPolicy {
    fn query_struct_name(&self, _: &str) -> StructSerializationStyle {
        StructSerializationStyle::Dict
    }
}

#[derive(Clone, Debug)]
pub struct StronglyTypedSerializerPolicy;

impl SerializerPolicy for StronglyTypedSerializerPolicy {
    fn query_struct_name(&self, _: &str) -> StructSerializationStyle {
        StructSerializationStyle::StronglyTyped
    }
}
