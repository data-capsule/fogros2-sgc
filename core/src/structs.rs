use anyhow::{anyhow, Result};
use rand::Rng;
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
    AdvertiseResponse = 3,
    RibGet = 4,
    RibReply = 5,
    Nack = 6,
    Control = 7,
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
            x if x == GdpAction::AdvertiseResponse as u8 => Ok(GdpAction::AdvertiseResponse),
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
pub struct GDPName(pub [u8; 4]); // 256 bit destination
impl fmt::Display for GDPName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn generate_random_gdp_name() -> GDPName {
    // u8:4
    GDPName([
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
    ])
}

pub(crate) trait Packet {
    /// get protobuf object of the packet
    /// get serialized byte array of the packet
    fn get_byte_payload(&self) -> Option<&Vec<u8>>;

    fn get_header(&self) -> GDPHeaderInTransit;
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GDPPacket {
    pub action: GdpAction,
    pub gdpname: GDPName,
    // the payload can be either (both)
    // Vec u8 bytes or protobuf
    // converting back and forth between proto and u8 is expensive
    // preferably forward directly without conversion
    pub payload: Option<Vec<u8>>,
    pub name_record: Option<GDPNameRecord>,
    pub source: GDPName,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct GDPHeaderInTransit {
    pub action: GdpAction,
    pub destination: GDPName,
    pub length: usize,
}

impl Packet for GDPPacket {
    fn get_byte_payload(&self) -> Option<&Vec<u8>> {
        match &self.payload {
            Some(p) => Some(p),
            None => None, // TODO
        }
    }

    fn get_header(&self) -> GDPHeaderInTransit {
        let name_record_length = match &self.name_record {
            Some(name_record) => serde_json::to_string(&name_record)
                .unwrap()
                .as_bytes()
                .len(),
            None => 0,
        };
        let transit_packet = match &self.payload {
            Some(payload) => GDPHeaderInTransit {
                action: self.action,
                destination: self.gdpname,
                length: payload.len() + name_record_length,
            },
            None => {
                GDPHeaderInTransit {
                    action: self.action,
                    destination: self.gdpname,
                    length: name_record_length, // doesn't have any payload
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
        } else {
            write!(f, "{:?}: packet do not exist", self.gdpname)
        }
    }
}

#[derive(Debug, Clone)]
pub struct GDPChannel {
    pub gdpname: GDPName,
    pub source: GDPName,
    pub channel: UnboundedSender<GDPPacket>,
    pub comment: String,
}

// union in rust is unsafe, use struct instead
// name record is what being stored in RIB and used for routing
// one can resolve the GDPNameRecord using RIB put and get
// it can be safely ported for another machine to connect
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GDPNameRecord {
    pub record_type: GDPNameRecordType,
    pub gdpname: GDPName,
    // the source of the record
    // if the record is the query, then the source_gdpname is the destination
    // that forward the data
    pub source_gdpname: GDPName,
    pub webrtc_offer: Option<String>,
    pub ip_address: Option<String>,
    pub ros: Option<(String, String)>,
    // indirect to another GDPName
    // this occurs if certain gdpname is hosted on a machine;
    // then we solve the GDP name to the machine's GDPName
    pub indirect: Option<GDPName>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum GDPNameRecordType {
    EMPTY,
    INFO, // inform the existence of the record, does not replace if present
    QUERY,
    UPDATE, // update the existing record by replacing the old one
    MERGE,  // merge-able into the existing record
    DELETE,
}

use sha2::Digest;
use sha2::Sha256;
pub fn get_gdp_name_from_topic(topic_name: &str, topic_type: &str, cert: &[u8]) -> [u8; 4] {
    // create a Sha256 object
    let mut hasher = Sha256::new();

    info!(
        "Name is generated from topic_name: {}, topic_type: {}, cert: (too long, not printed)",
        topic_name, topic_type
    );
    // hash with name, type and certificate
    hasher.update(topic_name);
    hasher.update(topic_type);
    hasher.update(cert);
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

pub fn gdp_name_to_string(GDPName(name): GDPName) -> String {
    format!("{:x}{:x}{:x}{:x}", name[0], name[1], name[2], name[3])
}
pub fn string_to_gdp_name(name: &str) -> GDPName {
    let mut bytes = [0u8; 4];
    for (i, byte) in name.as_bytes().chunks(2).enumerate() {
        bytes[i] = u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 16).unwrap();
    }
    GDPName(bytes)
}
