extern crate tokio;
extern crate tokio_core;
use crate::crypto::cert::*;
use crate::network::tcp::tcp_listener;
use futures::future;
use tokio::sync::mpsc::{self};
use utils::app_config::AppConfig;
use utils::error::Result;

use crate::connection_rib::connection_router;
use crate::network::dtls::{dtls_listener, dtls_test_client, dtls_test_server};

const dtls_addr: &'static str = "127.0.0.1:9232";

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file
#[tokio::main]
async fn router_async_loop() {
    // rib_rx <GDPPacket = [u8]>: forward gdppacket to rib
    let (rib_tx, rib_rx) = mpsc::channel(32);
    // channel_tx <GDPChannel = <gdp_name, sender>>: forward channel maping to rib
    let (channel_tx, channel_rx) = mpsc::channel(32);

    let tcp_sender_handle = tokio::spawn(tcp_listener(
        "127.0.0.1:9997",
        rib_tx.clone(),
        channel_tx.clone(),
    ));

    let dtls_sender_handle =
        tokio::spawn(dtls_listener(dtls_addr, rib_tx.clone(), channel_tx.clone()));
    //let dtls_sender_handle = tokio::spawn(dtls_test_server(dtls_addr));

    let rib_handle = tokio::spawn(connection_router(rib_rx, channel_rx));

    future::join_all([tcp_sender_handle, rib_handle, dtls_sender_handle]).await;
    //join!(foo_sender_handle, bar_sender_handle, receive_handle);
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
    println!("{:#?}", config);

    Ok(())
}

/// Simulate an error
pub fn simulate_error() -> Result<()> {
    // Log this Error simulation
    info!("We are simulating an error");
    // test_cert();
    dtls_test_client(dtls_addr);
    Ok(())
}
