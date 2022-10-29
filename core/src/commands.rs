

use utils::app_config::AppConfig;
use utils::error::Result;
use super::pipeline;

/// Show the configuration file
pub fn router() -> Result<()> {
    warn!("router is started!");

    pipeline::pipeline();
    
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
