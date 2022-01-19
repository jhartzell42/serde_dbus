#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub data: Vec<u8>,
    pub signature: Vec<u8>,
}
