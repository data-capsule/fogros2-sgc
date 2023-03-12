use pnet_macros::packet;
use pnet_macros_support::types::*;
use pnet_packet::PrimitiveValues;

/// Documentation for MyProtocolField
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash)]
pub struct MyProtocolField(pub u8);

impl MyProtocolField {
    pub fn new(field_val: u8) -> MyProtocolField {
        MyProtocolField(field_val)
    }
}

impl PrimitiveValues for MyProtocolField {
    type T = (u8,);

    fn to_primitive_values(&self) -> (u8,) {
        (self.0,)
    }
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod MyProtocolFieldValues {
    use super::MyProtocolField;

    /// Documentation for VALUE_FOO
    pub const VALUE_FOO: MyProtocolField = MyProtocolField(0);
    /// Documentation for VALUE_BAR
    pub const VALUE_BAR: MyProtocolField = MyProtocolField(1);
}

/// Documentation for MyProtocol
#[packet]
pub struct MyProtocol {
    #[construct_with(u8)]
    field: MyProtocolField,
    checksum: u16be,
    #[payload]
    payload: Vec<u8>,
}
