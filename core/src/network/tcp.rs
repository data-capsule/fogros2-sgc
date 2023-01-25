use crate::pipeline::construct_gdp_advertisement_from_bytes;
use crate::pipeline::proc_gdp_packet;

use crate::structs::GDPName;
use crate::structs::{GDPChannel, GDPPacket, GdpAction, Packet};
use std::io;

use std::{net::SocketAddr, str::FromStr};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
const UDP_BUFFER_SIZE: usize = 4096; // 17480 17kb TODO: make it formal
use crate::pipeline::construct_gdp_forward_from_bytes;
use crate::structs::GDPPacketInTransit;
use rand::Rng;

fn generate_random_gdp_name_for_thread() -> GDPName {
    // u8:4
    GDPName([
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
        rand::thread_rng().gen(),
    ])
}

/// handle one single session of tcpstream
/// 1. init and advertise the mpsc channel to connection rib
/// 2. select between
///         incoming tcp packets -> receive and send to rib
///         incomine packets from rib -> send to the tcp session
async fn handle_tcp_stream(
    stream: TcpStream, rib_tx: &UnboundedSender<GDPPacket>,
    channel_tx: &UnboundedSender<GDPChannel>, m_tx: UnboundedSender<GDPPacket>,
    mut m_rx: UnboundedReceiver<GDPPacket>, thread_name: GDPName,
) {
    // ...

    // init variables

    let mut received_header_yet = false;
    let mut gdp_header: GDPPacketInTransit = GDPPacketInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, //doesn't have any payload
    };
    let mut read_payload_size: usize = 0;
    let mut gdp_payload: Vec<u8> = vec![];
    loop {
        // Wait for the TCP socket to be readable
        // or new data to be sent
        tokio::select! {
            // new stuff from TCP!
            _f = stream.readable() => {
                // Creating the buffer **after** the `await` prevents it from
                // being stored in the async task.

                let mut receiving_buf = vec![0u8; UDP_BUFFER_SIZE];

                // Try to read data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_read(&mut receiving_buf) {
                    Ok(0) => break,
                    Ok(receiving_buf_size) => {
                        println!("read {} bytes", receiving_buf_size);
                        match received_header_yet {
                            true => {
                                gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                                read_payload_size += receiving_buf_size;
                                if read_payload_size < gdp_header.length {
                                    continue;
                                }
                            },
                            false => {
                                let received_buffer = &receiving_buf[..receiving_buf_size];
                                // use the first null byte \0 as delimiter
                                // split to the first /0 as delimiter
                                info!("received buffer: {:?}", received_buffer);
                                let header_and_remaining = received_buffer.splitn(2, |c| c == &0).collect::<Vec<_>>();
                                let header_buf = header_and_remaining[0];

                                let header:&str = std::str::from_utf8(header_buf).unwrap();
                                info!("received header json string: {:?}", header);
                                gdp_header = serde_json::from_str::<GDPPacketInTransit>(header).unwrap().clone();
                                received_header_yet = true;

                                let payload = header_and_remaining[1];

                                info!("received header {:?}", payload);
                                // parse header from json
                                // append the rest of the payload to the payload buffer
                                read_payload_size += payload.len();
                                gdp_payload.append(&mut payload.to_vec());
                                info!("received payload with size {:}",  payload.len());
                                if read_payload_size < gdp_header.length {
                                    continue;
                                }
                            }
                        };
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    },
                    Err(_e) => {
                        continue;
                    }
                }

                info!("the total received payload with size {:} and size {} with gdp header length {}",  gdp_payload.len(), read_payload_size, gdp_header.length);

                let deserialized = gdp_header; //TODO: change the var name here

                if deserialized.action == GdpAction::Forward {
                    let packet = construct_gdp_forward_from_bytes(deserialized.destination, thread_name, gdp_payload); //todo
                    proc_gdp_packet(packet,  // packet
                        rib_tx,  //used to send packet to rib
                        channel_tx, // used to send GDPChannel to rib
                        &m_tx //the sending handle of this connection
                    ).await;
                }
                else if deserialized.action == GdpAction::Advertise {
                    let packet = construct_gdp_advertisement_from_bytes(deserialized.destination, thread_name);
                    proc_gdp_packet(packet,  // packet
                        rib_tx,  //used to send packet to rib
                        channel_tx, // used to send GDPChannel to rib
                        &m_tx //the sending handle of this connection
                    ).await;
                }
                else{
                    info!("TCP received a packet but did not handle: {:?}", deserialized)
                }

                read_payload_size = 0;
                gdp_payload = vec!();
                received_header_yet = false;
            },

            // new data to send to TCP!
            Some(pkt_to_forward) = m_rx.recv() => {
                // okay this may have deadlock
                stream.writable().await.expect("TCP stream is closed");

                //info!("TCP packet to forward: {:?}", pkt_to_forward);
                let transit_header = pkt_to_forward.get_header();
                let mut header_string = serde_json::to_string(&transit_header).unwrap();
                info!("the final serialized size is {}", header_string.len());

                //insert the first null byte to separate the packet header
                header_string.push(0u8 as char);
                // Convert the Point to a JSON string.
                // Try to write data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_write(header_string.as_bytes()) {
                    Ok(n) => {
                        println!("write {} bytes", n);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue
                    }
                    Err(_e) => {
                        println!("Err of other kind");
                        continue
                    }
                }
                if let Some(payload) = pkt_to_forward.payload {
                    match stream.try_write(&payload) {
                        Ok(n) => {
                            println!("payload with size {}: write {} bytes", payload.len(), n);
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue
                        }
                        Err(_e) => {
                            println!("Err of other kind");
                            continue
                        }
                    }
                }
            },
        }
    }
}

/// listen at @param address and process on tcp accept()
///     rib_tx: channel that send GDPPacket to rib
///     channel_tx: channel that advertise GDPChannel to rib
pub async fn tcp_listener(
    addr: String, rib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
) {
    let listener = TcpListener::bind(&addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let rib_tx = rib_tx.clone();
        let channel_tx = channel_tx.clone();

        // Process each socket concurrently.
        tokio::spawn(async move {
            let (m_tx, m_rx) = mpsc::unbounded_channel();
            handle_tcp_stream(
                socket,
                &rib_tx,
                &channel_tx,
                m_tx,
                m_rx,
                generate_random_gdp_name_for_thread(),
            )
            .await
        });
    }
}

pub async fn tcp_to_peer(
    addr: String, rib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
) {
    let stream = match TcpStream::connect(SocketAddr::from_str(&addr).unwrap()).await {
        Ok(s) => s,
        Err(_) => {
            error!("TCP: Unable to connect to the dafault gateway {}", addr);
            return;
        }
    };

    println!("{:?}", stream);

    let m_gdp_name = generate_random_gdp_name_for_thread();
    info!("TCP takes gdp name {:?}", m_gdp_name);

    let (m_tx, m_rx) = mpsc::unbounded_channel();
    let node_advertisement = construct_gdp_advertisement_from_bytes(m_gdp_name, m_gdp_name);
    proc_gdp_packet(
        node_advertisement, // packet
        &rib_tx,            //used to send packet to rib
        &channel_tx,        // used to send GDPChannel to rib
        &m_tx,              //the sending handle of this connection
    )
    .await;
    handle_tcp_stream(stream, &rib_tx, &channel_tx, m_tx, m_rx, m_gdp_name).await;
}
