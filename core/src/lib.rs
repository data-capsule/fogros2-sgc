#[macro_use]
extern crate log;

// sub crates and primitives
pub mod crypto;
pub mod network;
pub mod protocol;
pub mod structs;
// helper processing lines
pub mod rib;
// network processing
pub mod connection_rib;
pub mod pipeline;
pub mod pipeline_pnet;
// util
pub mod commands;

use utils::error::Result;

pub mod gdp_proto {
    tonic::include_proto!("gdp"); // The string specified here must match the proto package name
}


pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
