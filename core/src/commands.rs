
extern crate tokio; 
extern crate tokio_core;
use utils::app_config::AppConfig;
use utils::error::Result;
use crate::network::tcpsocket::{
    message_sender, tcp_listener
};
use crate::crypto::cert::*;

use futures::future; 
use tokio::{sync::mpsc::{self, channel, Sender}};


/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
#[tokio::main]
async fn router_async_loop() {
    let (foo_tx, mut foo_rx) = mpsc::channel(32);
    let (bar_tx, mut bar_rx) = mpsc::channel(32);

    let foo_sender_handle = tokio::spawn(tcp_listener("127.0.0.1:9999", foo_tx));
    //let bar_sender_handle = tokio::spawn(message_sender("bar", bar_tx));
    let bar_sender_handle = tokio::spawn(tcp_listener("127.0.0.1:9998", bar_tx));

    let receive_handle = tokio::spawn(async move {
        let mut foo:Option<String> = None;
        let mut bar:Option<String> = None;

        loop {
            tokio::select! {
                f = foo_rx.recv() => foo = f,
                b = bar_rx.recv() => bar = b,
            }

            if let Some(foo) = &foo {
                println!("9999: {foo}");
            }
            if let Some(bar) = &bar {
                println!("9998: {bar}");
            }
            
            foo = None;
            bar = None;
        }
    });

    future::join_all([foo_sender_handle, bar_sender_handle, receive_handle]).await;
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
