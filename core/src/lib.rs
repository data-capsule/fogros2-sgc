#[macro_use]
extern crate log;

pub mod commands;
pub mod protocol;
pub mod pipeline;

use utils::error::Result;

pub fn start() -> Result<()> {
    // does nothing

    Ok(())
}
