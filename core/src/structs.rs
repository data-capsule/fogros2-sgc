use anyhow::{anyhow, Result};
use std::fmt;
use strum_macros::EnumIter;
use tokio::sync::mpsc::UnboundedSender;
pub const MAGIC_NUMBERS: u16 = u16::from_be_bytes([0x26, 0x2a]);

pub type GdpName = [u8; 32];
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash, EnumIter)]
pub enum GdpAction {
    Noop = 0,
    Forward = 1,
    Advertise = 2,
    RibGet = 3,
    RibReply = 4,
    Nack = 5,
    Control = 6,
}

impl Default for GdpAction {
    fn default() -> Self {
        GdpAction::Noop
    }
}

impl TryFrom<u8> for GdpAction {
    type Error = anyhow::Error;

    fn try_from(v: u8) -> Result<Self> {
        match v {
            x if x == GdpAction::Noop as u8 => Ok(GdpAction::Noop),
            x if x == GdpAction::RibGet as u8 => Ok(GdpAction::RibGet),
            x if x == GdpAction::RibReply as u8 => Ok(GdpAction::RibReply),
            x if x == GdpAction::Forward as u8 => Ok(GdpAction::Forward),
            x if x == GdpAction::Nack as u8 => Ok(GdpAction::Nack),
            x if x == GdpAction::Control as u8 => Ok(GdpAction::Control),
            unknown => Err(anyhow!("Unknown action byte ({:?})", unknown)),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, packed)]
pub struct u16be(u16);

impl From<u16> for u16be {
    fn from(item: u16) -> Self {
        u16be(u16::to_be(item))
    }
}

impl From<u16be> for u16 {
    fn from(item: u16be) -> Self {
        u16::from_be(item.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash, Default)]
pub struct GDPName(pub [u8; 4]); //256 bit destination
impl fmt::Display for GDPName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

use crate::gdp_proto::GdpPacket;
pub(crate) trait Packet {
    /// get protobuf object of the packet
    fn get_proto(&self) -> Option<&GdpPacket>;
    /// get serialized byte array of the packet
    fn get_byte_payload(&self) -> Option<&Vec<u8>>;

    fn get_header(&self) -> GDPPacketInTransit;
}

#[derive(Debug, PartialEq, Clone)]
pub struct GDPPacket {
    pub action: GdpAction,
    pub gdpname: GDPName,
    // the payload can be either (both)
    // Vec u8 bytes or protobuf
    // converting back and forth between proto and u8 is expensive
    // preferably forward directly without conversion
    pub payload: Option<Vec<u8>>,
    pub proto: Option<GdpPacket>,
    pub source: GDPName,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct GDPPacketInTransit {
    pub action: GdpAction,
    pub destination: GDPName,
    pub length: usize,
}

impl Packet for GDPPacket {
    fn get_proto(&self) -> Option<&GdpPacket> {
        match &self.proto {
            Some(p) => Some(p),
            None => None, //TODO
        }
    }
    fn get_byte_payload(&self) -> Option<&Vec<u8>> {
        match &self.payload {
            Some(p) => Some(p),
            None => None, //TODO
        }
    }
    fn get_header(&self) -> GDPPacketInTransit {
        let transit_packet = match &self.payload {
            Some(payload) => GDPPacketInTransit {
                action: self.action,
                destination: self.gdpname,
                length: payload.len(),
            },
            None => {
                GDPPacketInTransit {
                    action: self.action,
                    destination: self.gdpname,
                    length: 0, //doesn't have any payload
                }
            }
        };
        // serde_json::to_string(&transit_packet).unwrap()
        transit_packet
    }
}

impl fmt::Display for GDPPacket {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        if let Some(payload) = &self.payload {
            let ret = match std::str::from_utf8(&payload) {
                Ok(payload) => payload.trim_matches(char::from(0)),
                Err(_) => "unable to render",
            };
            write!(f, "{:?}: {:?}", self.gdpname, ret)
        } else if let Some(payload) = &self.proto {
            write!(f, "{:?}: {:?}", self.gdpname, payload)
        } else {
            write!(f, "{:?}: packet do not exist", self.gdpname)
        }
    }
}

#[derive(Debug, Clone)]
pub struct GDPChannel {
    pub gdpname: GDPName,
    pub channel: UnboundedSender<GDPPacket>,
    pub advertisement: GDPPacket,
}

use sha2::Digest;
use sha2::Sha256;
pub fn get_gdp_name_from_topic(topic_name: &str) -> [u8; 4] {
    // create a Sha256 object
    let mut hasher = Sha256::new();

    // write input message
    hasher.update(topic_name);
    let result = hasher.finalize();
    // Get the first 4 bytes of the digest
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&result[..4]);

    bytes
    // // Convert the bytes to a u32
    // unsafe { transmute::<[u8; 4], u32>(bytes) }
}

#[derive(Debug, Clone)]
pub struct GDPStatus {
    pub sink: UnboundedSender<GDPPacket>,
}
