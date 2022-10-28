#[macro_use]
extern crate log;

pub mod commands;
pub mod error;
pub mod hazard;
pub mod pipeline;
pub mod packet;


use utils::error::Result;

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
