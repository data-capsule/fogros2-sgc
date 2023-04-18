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

    // fib_rx <GDPPacket = [u8]>: forward gdppacket to fib
    let (fib_tx, fib_rx) = mpsc::unbounded_channel();
    // rib_rx <GDPNameRecord>: rib queries
    let (rib_query_tx, rib_query_rx) = mpsc::unbounded_channel();
    // rib_rx <GDPNameRecord>: rib queries
    let (rib_response_tx, rib_response_rx) = mpsc::unbounded_channel();
    // channel_tx <GDPChannel = <gdp_name, sender>>: forward channel maping to fib
    let (channel_tx, channel_rx) = mpsc::unbounded_channel();
    // stat_tx <GdpUpdate proto>: any status update from other routers
    let (stat_tx, stat_rx) = mpsc::unbounded_channel();

    // ros_manager <GDPNameRecord>: to and from ros manager
    let (_ros_manager_tx, ros_manager_rx) = mpsc::unbounded_channel();

    let rib_handle = tokio::spawn(local_rib_handler(
        rib_query_rx,    // get routing queries/updates to rib
        rib_response_tx, // send routing queries/updates from rib
        stat_tx.clone(), // send status updates to fib
    ));
    future_handles.push(rib_handle);

    let fib_handle = tokio::spawn(connection_fib(
        fib_rx,               // receive packets to forward
        rib_query_tx.clone(), // send routing queries to rib
        rib_response_rx,      // get routing queries from rib
        stat_rx,              // recevie control place info, e.g. routing
        channel_rx,           // receive channel information for connection fib
    ));
    future_handles.push(fib_handle);

    sleep(Duration::from_millis(500)).await;

    let tcp_sender_handle = tokio::spawn(tcp_listener(
        tcp_bind_addr,
        fib_tx.clone(),
        channel_tx.clone(),
        rib_query_tx.clone(),
    ));
    future_handles.push(tcp_sender_handle);

    let dtls_sender_handle = tokio::spawn(dtls_listener(
        dtls_bind_addr,
        fib_tx.clone(),
        channel_tx.clone(),
        rib_query_tx.clone(),
    ));
    future_handles.push(dtls_sender_handle);

    // let webrtc_sender_handle = tokio::spawn(webrtc_main(
    //     "other_id".to_string(),
    //     None,
    //     fib_tx.clone(),
    //     channel_tx.clone(),
    //     rib_query_tx.clone(),
    // ));
    // future_handles.push(webrtc_sender_handle);

    // let webrtc_sender_handle2 = tokio::spawn(webrtc_main(
    //     "other_id2".to_string(),
    //     Some("other_id".to_string()),
    //     fib_tx.clone(),
    //     channel_tx.clone(),
    //     rib_query_tx.clone(),
    // ));
    // future_handles.push(webrtc_sender_handle2);

    // grpc
    // TODO: uncomment for grpc
    // let psl_service = GDPService {
    //     fib_tx: fib_tx.clone(),
    //     status_tx: stat_tx,
    // };

    // let serve = Server::builder()
    //     .add_service(GlobaldataplaneServer::new(psl_service))
    //     .serve(grpc_bind_addr.parse().unwrap());
    // let manager_handle = tokio::spawn(async move {
    //     if let Err(e) = serve.await {
    //         eprintln!("Error = {:?}", e);
    //     }
    // });
    // let grpc_server_handle = manager_handle;
    // future_handles.push(grpc_server_handle);

    let ros_topic_manager_handle = tokio::spawn(ros_topic_manager(
        peer_with_gateway,
        default_gateway_addr.clone(),
        fib_tx.clone(),
        ros_manager_rx,
        channel_tx.clone(),
        rib_query_tx.clone(),
    ));
    future_handles.push(ros_topic_manager_handle);

    // This connection is only used to advertise the routing information
    // to the gateway. It is not used to forward any packets.
    // the packets are forwarded by direct peer connections in ros_topic_manager
    if peer_with_gateway {
        let (m_tx, m_rx) = mpsc::unbounded_channel();
        if config.ros_protocol == "dtls" {
            let peer_advertisement = tokio::spawn(dtls_to_peer(
                default_gateway_addr.clone().into(),
                fib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
                rib_query_tx.clone(),
            ));
            future_handles.push(peer_advertisement);
        } else if config.ros_protocol == "tcp" {
            let peer_advertisement = tokio::spawn(tcp_to_peer(
                default_gateway_addr.clone().into(),
                fib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
                rib_query_tx.clone(),
            ));
            future_handles.push(peer_advertisement);
        }

        // only non-gateway proxy needs to advertise themselves
        // it needs the connection to be settled first, otherwise the advertisement is lost
        sleep(Duration::from_millis(1500)).await;
        info!("Flushing the RIB....");
        stat_tx
            .send(GDPStatus { sink: m_tx })
            .expect("Flush the RIB Failure");
    }

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
