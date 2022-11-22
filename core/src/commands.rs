extern crate tokio;
extern crate tokio_core;
use crate::network::tcp::tcp_listener;
use futures::future;
use tokio::sync::mpsc::{self};
use utils::app_config::AppConfig;
use utils::error::Result;

use crate::connection_rib::connection_router;
use crate::network::dtls::{dtls_listener, dtls_test_client};
use tonic::{transport::Server, Request, Response, Status};

use crate::gdp_proto::globaldataplane_client::GlobaldataplaneClient;
use crate::gdp_proto::globaldataplane_server::{Globaldataplane, GlobaldataplaneServer};
use crate::gdp_proto::{GdpPacket, GdpResponse, GdpUpdate};
use crate::network::grpc::GDPService;

// const TCP_ADDR: &'static str = "127.0.0.1:9997";
// const DTLS_ADDR: &'static str = "127.0.0.1:9232";
// const GRPC_ADDR: &'static str = "0.0.0.0:50001";

/// inspired by https://stackoverflow.com/questions/71314504/how-do-i-simultaneously-read-messages-from-multiple-tokio-channels-in-a-single-t
/// TODO: later put to another file
#[tokio::main]
async fn router_async_loop() {
    let config = AppConfig::fetch().expect("App config unable to load");
    println!("{:#?}", config);

    // initialize the address binding 
    let all_addr = "0.0.0.0"; //optionally use [::0] for ipv6 address
    let tcp_bind_addr = format!("{}:{}", all_addr , config.tcp_port);
    let dtls_bind_addr = format!("{}:{}", all_addr, config.dtls_port);
    let grpc_bind_addr = format!("{}:{}", all_addr, config.grpc_port);

    // rib_rx <GDPPacket = [u8]>: forward gdppacket to rib
    let (rib_tx, rib_rx) = mpsc::channel(config.channel_size);
    // channel_tx <GDPChannel = <gdp_name, sender>>: forward channel maping to rib
    let (channel_tx, channel_rx) = mpsc::channel(config.channel_size);
    // stat_tx <GdpUpdate proto>: any status update from other routers
    let (stat_tx, stat_rx) = mpsc::channel(config.channel_size);

    let tcp_sender_handle =
        tokio::spawn(tcp_listener(tcp_bind_addr, rib_tx.clone(), channel_tx.clone()));

    let dtls_sender_handle =
        tokio::spawn(dtls_listener(dtls_bind_addr, rib_tx.clone(), channel_tx.clone()));

    let psl_service = GDPService {
        rib_tx: rib_tx,
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
        rib_rx, // receive packets to forward 
        stat_rx, // recevie control place info, e.g. routing 
        channel_rx // receive channel information for connection rib
    ));

    future::join_all([
        tcp_sender_handle,
        rib_handle,
        dtls_sender_handle,
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
    println!("{:#?}", config);

    Ok(())
}

/// Simulate an error
pub fn simulate_error() -> Result<()> {
    let config = AppConfig::fetch().expect("App config unable to load");
    println!("{:#?}", config);
    // test_cert();
    // get address from default gateway
    let test_router_addr = format!("{}:{}", config.default_gateway , config.dtls_port);
    dtls_test_client(test_router_addr).expect("DLTS Client error");
    Ok(())
}
