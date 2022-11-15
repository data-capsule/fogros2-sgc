use anyhow::{anyhow, Result};
use std::fmt;
use strum_macros::EnumIter;
pub const MAGIC_NUMBERS: u16 = u16::from_be_bytes([0x26, 0x2a]);

pub type GdpName = [u8; 32];

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, EnumIter)]
pub enum GdpAction {
    Noop = 0,
    Put = 1,
    Get = 2,
    RibGet = 3,
    RibReply = 4,
    Forward = 5,
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
            x if x == GdpAction::Get as u8 => Ok(GdpAction::Get),
            x if x == GdpAction::Put as u8 => Ok(GdpAction::Put),
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub struct GDPName(pub [u8; 4]); //256 bit destination
impl fmt::Display for GDPName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct GDPPacket {
    pub action: GdpAction,
    pub gdpname: GDPName,
    pub payload: [u8; 2048],
}

impl fmt::Display for GDPPacket {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(
            f,
            "{:?}: {:?}",
            self.gdpname,
            std::str::from_utf8(&self.payload)
                .expect("parsing failure")
                .trim_matches(char::from(0))
        )
    }
}

use tokio::sync::mpsc::Sender;
pub struct GDPChannel {
    pub gdpname: GDPName,
    pub channel: Sender<GDPPacket>,
}
