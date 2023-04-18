#[macro_use]
extern crate log;

// sub crates and primitives
pub mod crypto;
pub mod network;
pub mod structs;

pub mod rib;
// network processing
pub mod connection_fib;
pub mod pipeline;
// util
pub mod commands;
pub mod topic_manager;
use utils::error::Result;

pub mod gdp_proto {
    tonic::include_proto!("gdp"); // The string specified here must match the proto package name
}

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
