extern crate tokio;
extern crate tokio_core;
use crate::connection_rib::connection_router;
use crate::network::dtls::{dtls_listener, dtls_test_client, dtls_to_peer};
use crate::network::tcp::{tcp_listener, tcp_to_peer};
use futures::future;
use tokio::sync::mpsc::{self};
use tonic::{transport::Server, Request, Response, Status};
use utils::app_config::AppConfig;
use utils::error::Result;

use crate::gdp_proto::globaldataplane_client::GlobaldataplaneClient;
use crate::gdp_proto::globaldataplane_server::{Globaldataplane, GlobaldataplaneServer};
use crate::gdp_proto::{GdpPacket, GdpResponse, GdpUpdate};
use crate::network::grpc::GDPService;

#[cfg(feature = "ros")]
use crate::network::ros::{ros_listener, ros_sample};
// const TCP_ADDR: &'static str = "127.0.0.1:9997";
// const DTLS_ADDR: &'static str = "127.0.0.1:9232";
// const GRPC_ADDR: &'static str = "0.0.0.0:50001";

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file
#[tokio::main]
async fn router_async_loop() {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);

    // initialize the address binding
    let all_addr = "0.0.0.0"; //optionally use [::0] for ipv6 address
    let tcp_bind_addr = format!("{}:{}", all_addr, config.tcp_port);
    let dtls_bind_addr = format!("{}:{}", all_addr, config.dtls_port);
    let grpc_bind_addr = format!("{}:{}", all_addr, config.grpc_port);

    // rib_rx <GDPPacket = [u8]>: forward gdppacket to rib
    let (rib_tx, rib_rx) = mpsc::channel(config.channel_size);
    // channel_tx <GDPChannel = <gdp_name, sender>>: forward channel maping to rib
    let (channel_tx, channel_rx) = mpsc::channel(config.channel_size);
    // stat_tx <GdpUpdate proto>: any status update from other routers
    let (stat_tx, stat_rx) = mpsc::channel(config.channel_size);

    let tcp_sender_handle = tokio::spawn(tcp_listener(
        tcp_bind_addr,
        rib_tx.clone(),
        channel_tx.clone(),
    ));

    let dtls_sender_handle = tokio::spawn(dtls_listener(
        dtls_bind_addr,
        rib_tx.clone(),
        channel_tx.clone(),
    ));

    let peer_advertisement = tokio::spawn(tcp_to_peer(
        "128.32.37.48:9997".into(),
        rib_tx.clone(),
        channel_tx.clone(),
    ));

    let psl_service = GDPService {
        rib_tx: rib_tx.clone(),
        status_tx: stat_tx,
    };

    // grpc
    let serve = Server::builder()
        .add_service(GlobaldataplaneServer::new(psl_service))
        .serve(grpc_bind_addr.parse().unwrap());
    let manager_handle = tokio::spawn(async move {
        if let Err(e) = serve.await {
            eprintln!("Error = {:?}", e);
        }
    });
    let grpc_server_handle = manager_handle;
    let rib_handle = tokio::spawn(connection_router(
        rib_rx,     // receive packets to forward
        stat_rx,    // recevie control place info, e.g. routing
        channel_rx, // receive channel information for connection rib
    ));

    #[cfg(feature = "ros")]
    let ros_sender_handle = tokio::spawn(ros_listener(rib_tx.clone(), channel_tx.clone()));

    future::join_all([
        tcp_sender_handle,
        rib_handle,
        #[cfg(feature = "ros")]
        ros_sender_handle,
        dtls_sender_handle,
        peer_advertisement,
        grpc_server_handle,
    ])
    .await;
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
    info!("{:#?}", config);

    Ok(())
}

/// Simulate an error
pub fn simulate_error() -> Result<()> {
    let config = AppConfig::fetch().expect("App config unable to load");
    info!("{:#?}", config);
    // test_cert();
    // get address from default gateway

    #[cfg(feature = "ros")]
    ros_sample();
    // TODO: uncomment them
    let test_router_addr = format!("{}:{}", config.default_gateway, config.dtls_port);
    println!("{}", test_router_addr);
    dtls_test_client("128.32.37.48:9232".into()).expect("DLTS Client error");
    Ok(())
}
