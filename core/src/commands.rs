extern crate tokio;
extern crate tokio_core;
use std::env;
use std::time::Duration;
use crate::connection_fib::connection_fib;
use crate::rib::local_rib_handler;
use crate::structs::GDPStatus;
use crate::topic_manager::ros_topic_manager;
use futures::future;

use tokio::sync::mpsc::{self};

use tokio::time::sleep;
use utils::app_config::AppConfig;
use utils::error::Result;

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file
#[tokio::main]
async fn router_async_loop() {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);
    let mut future_handles = Vec::new();


    let ros_topic_manager_handle = tokio::spawn(ros_topic_manager(
    ));
    future_handles.push(ros_topic_manager_handle);

    future::join_all(future_handles).await;
}

/// Show the configuration file
pub fn router() -> Result<()> {
    warn!("router is started!");

    // NOTE: uncomment to use pnet
    // libpnet::pnet_proc_loop();
    router_async_loop();

    Ok(())
}

/// Show the configuration file
pub fn config() -> Result<()> {
    let config = AppConfig::fetch()?;
    info!("{:#?}", config);

    Ok(())
}
#[tokio::main]
/// Simulate an error
pub async fn simulate_error() -> Result<()> {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);
    // test_cert();
    // get address from default gateway

    // ros_sample();
    // TODO: uncomment them
    // webrtc_main("my_id".to_string(), Some("other_id".to_string())).await;
    Ok(())
}
