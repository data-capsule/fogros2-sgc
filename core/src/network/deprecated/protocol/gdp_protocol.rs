use pnet_macros::packet;
use pnet_macros_support::types::*;

/// Documentation for MyProtocol
#[packet]
pub struct GdpProtocol {
    #[length = "32"]
    src_gdpname: Vec<u8>,
    #[length = "32"]
    dst_gdpname: Vec<u8>,
    #[length = "16"]
    uuid: Vec<u8>,
    num_packets: u32be,
    packet_no: u32be,
    data_len: u16be,
    action: u8,
    ttl: u8,
    #[payload]
    payload: Vec<u8>,
}
