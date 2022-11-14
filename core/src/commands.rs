
extern crate tokio; 
extern crate tokio_core;
use utils::app_config::AppConfig;
use utils::error::Result;
use crate::network::libpnet;
use crate::crypto::cert::*;

use futures::future; // 0.3.19
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    time,
}; // 1.16.1

use tokio::net::{TcpListener, TcpStream};
use std::io;
async fn process(socket: TcpStream, foo_tx: &UnboundedSender<String>) {
    // ...
    println!("got packets!!");
    let message = format!("hello");
    foo_tx.send(message).unwrap();
}

async fn tcp_listener(msg: &'static str, foo_tx: UnboundedSender<String>)  {
    let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        // Process each socket concurrently.
        process(socket, &foo_tx).await
    }
}

async fn message_sender(msg: &'static str, foo_tx: UnboundedSender<String>) {
    for count in 0.. {
        let message = format!("{msg}{count}");
        foo_tx.send(message).unwrap();

        if msg == "foo" {
            time::sleep(Duration::from_millis(100)).await;
        } else {
            time::sleep(Duration::from_millis(300)).await;
        }
    }
}

#[tokio::main]
async fn router_async_loop() {
    let (foo_tx, mut foo_rx) = mpsc::unbounded_channel();
    let (bar_tx, mut bar_rx) = mpsc::unbounded_channel();

    let foo_sender_handle = tokio::spawn(message_sender("foo", foo_tx));
    //let bar_sender_handle = tokio::spawn(message_sender("bar", bar_tx));
    let bar_sender_handle = tokio::spawn(tcp_listener("bar", bar_tx));

    let receive_handle = tokio::spawn(async move {
        let mut foo = None;
        let mut bar = None;

        loop {
            tokio::select! {
                f = foo_rx.recv() => foo = f,
                b = bar_rx.recv() => bar = b,
            }

            if let (Some(foo), Some(bar)) = (&foo, &bar) {
                println!("{foo}{bar}");
            }
            // TODO: flush foo & bar to be none, only one can survive
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
