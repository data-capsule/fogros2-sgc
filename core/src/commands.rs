
extern crate tokio; 
extern crate tokio_core;
use utils::app_config::AppConfig;
use utils::error::Result;
use crate::network::tcpsocket::{ tcp_listener};
use crate::crypto::cert::*;

use futures::future; 
use tokio::{sync::mpsc::{self, channel, Sender}};

use crate::connection_rib::connection_router;

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file 
#[tokio::main]
async fn router_async_loop() {
    let (tcp_tx, mut tcp_rx) = mpsc::channel(32);

    let foo_sender_handle = tokio::spawn(tcp_listener("127.0.0.1:9997", tcp_tx));
    //let bar_sender_handle = tokio::spawn(message_sender("bar", bar_tx));
    let rib_handle = tokio::spawn(connection_router(tcp_rx));

    future::join_all([foo_sender_handle, rib_handle]).await;
    //join!(foo_sender_handle, bar_sender_handle, receive_handle);
}

/// Show the configuration file
pub fn router() -> Result<()> {
    warn!("router is started!");

    // libpnet::pnet_proc_loop();
    router_async_loop();
    
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
    test_cert();

    Ok(())
}
