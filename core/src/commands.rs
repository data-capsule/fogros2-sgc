

use utils::app_config::AppConfig;
use utils::error::Result;
use crate::network::libpnet;

/// Show the configuration file
pub fn router() -> Result<()> {
    warn!("router is started!");

    libpnet::pnet_proc_loop();
    
    Ok(())
}

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    println!("{:#?}", config);

    Ok(())
}

/// Simulate an error
pub fn simulate_error() -> Result<()> {
    // Log this Error simulation
    info!("We are simulating an error");

    Ok(())
}
