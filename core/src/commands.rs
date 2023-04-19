extern crate tokio;
extern crate tokio_core;
use std::env;
use std::time::Duration;

use crate::connection_fib::connection_fib;

use crate::network::dtls::{dtls_listener, dtls_to_peer};
use crate::network::tcp::{tcp_listener, tcp_to_peer};
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

    let mut peer_with_gateway = config.peer_with_gateway;
    let default_gateway_ip = match env::var_os("GATEWAY_IP") {
        Some(gateway_ip) => {
            info!("override to connect gateway to be true");
            info!("Replace Gateway IP with {}", gateway_ip.to_string_lossy());
            peer_with_gateway = true;
            gateway_ip
                .into_string()
                .expect("Gateway IP is not a valid string")
        }
        None => config.default_gateway,
    };

    // initialize the address binding
    let all_addr = "0.0.0.0"; // optionally use [::0] for ipv6 address
    let tcp_bind_addr = format!("{}:{}", all_addr, config.tcp_port);
    let dtls_bind_addr = format!("{}:{}", all_addr, config.dtls_port);

    let default_gateway_addr = match config.ros_protocol.as_str() {
        "dtls" => format!("{}:{}", default_gateway_ip, config.dtls_port),
        "tcp" => format!("{}:{}", default_gateway_ip, config.tcp_port),
        _ => panic!("Unknown protocol"),
    };

    let ros_topic_manager_handle = tokio::spawn(ros_topic_manager(
        peer_with_gateway,
        default_gateway_addr.clone(),
    ));
    future_handles.push(ros_topic_manager_handle);

    // let join_handles = future::join_all(future_handles);
    // let futures: FuturesUnordered<_> = future_handles
    // .into_iter()
    // .collect();
    // futures.push(join_handles);
    // futures.push(webrtc_listener_handle);
    // futures.collect().await;
    // webrtc_listener_handle.await;
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
