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

/// parse the header of the packet using the first null byte as delimiter
/// return a vector of (header, payload) pairs if the header is complete
/// return the remaining (header, payload) pairs if the header is incomplete
fn parse_header_payload_pairs(
    mut buffer: Vec<u8>, 
)
-> (Vec<(GDPPacketInTransit, Vec<u8>)>, Option<(GDPPacketInTransit, Vec<u8>)>) {
    let mut header_payload_pairs: Vec<(GDPPacketInTransit, Vec<u8>)> = Vec::new();
    //TODO: get it to default trace later
    let mut default_gdp_header: GDPPacketInTransit = GDPPacketInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, //doesn't have any payload
    };
    loop {
        // parse the header
        // use the first null byte \0 as delimiter
        // split to the first \0 as delimiter
        let header_and_remaining = buffer.splitn(2, |c| c == &0).collect::<Vec<_>>();
        let header_buf = header_and_remaining[0];
        let header:&str = std::str::from_utf8(header_buf).unwrap();
        info!("received header json string: {:?}", header);
        let gdp_header = serde_json::from_str::<GDPPacketInTransit>(header).unwrap().clone();
        let remaining = header_and_remaining[1];

        if (gdp_header.length > remaining.len()) {
            // if the payload is not complete, return the remaining
            return (header_payload_pairs, Some((gdp_header, remaining.to_vec())));
        } else if (gdp_header.length == remaining.len()) {
            // if the payload is complete, return the pair
            header_payload_pairs.push((gdp_header, remaining.to_vec()));
            return (header_payload_pairs, None);
        } else {
            // if the payload is longer than the remaining, continue to parse
            header_payload_pairs.push((gdp_header, remaining[..gdp_header.length].to_vec()));
            buffer = remaining[gdp_header.length..].to_vec();
        }
    }
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

    let mut need_more_data_for_previous_header = false;
    let mut remaining_gdp_header: GDPPacketInTransit = GDPPacketInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, //doesn't have any payload
    };
    let mut remaining_gdp_payload: Vec<u8> = vec![];

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
                        let mut receiving_buf = receiving_buf[..receiving_buf_size].to_vec();
                        println!("read {} bytes", receiving_buf_size);

                        // match need_more_data_for_previous_header {
                        //     true => { // last time it has incomplete buffer to complete
                        //         read_payload_size += receiving_buf_size; 
                        //         if read_payload_size < gdp_header.length { //still need more things to read!
                        //             info!("more data to read");
                        //             gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                        //             continue;
                        //         } 
                        //         else if read_payload_size == gdp_header.length { // match the end of the packet
                        //             info!("match and this is the end of buffer");
                        //             payload_need_to_process.push(gdp_payload.clone());
                        //         } 
                        //         else{ //overflow!!
                        //             error!("Bytes are extra!!! {}", read_payload_size - gdp_header.length);
                        //         }
                        //     },
                        //     false => { // no, it need to read header first, a clean start
                        //         let received_buffer = &receiving_buf[..receiving_buf_size];
                        //         info!("received buffer size: {}", receiving_buf_size);

                        //         // parse header from json
                        //         // append the rest of the payload to the payload buffer
                        //         read_payload_size += remaining.len();
                        //         gdp_payload.append(&mut remaining.to_vec());
                        //         info!("received payload with size {:}",  remaining.len());
                        //         if read_payload_size < gdp_header.length { //TODO: not needed if do it right
                        //             continue;
                        //         }
                        //         else if read_payload_size == gdp_header.length {
                        //             payload_need_to_process.push(gdp_payload.clone());
                        //         }
                        //         else{
                        //             error!("Bytes are extra!!! {}", read_payload_size - gdp_header.length);
                        //         }
                        //     }
                        // };

                        let mut header_payload_pair = vec!();

                        // last time it has incomplete buffer to complete
                        if need_more_data_for_previous_header { 
                            let read_payload_size = remaining_gdp_payload.len() + receiving_buf_size;
                            if read_payload_size < remaining_gdp_header.length { //still need more things to read!
                                info!("more data to read");
                                remaining_gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                                continue;
                            } 
                            else if read_payload_size == remaining_gdp_header.length { // match the end of the packet
                                info!("match and this is the end of buffer");
                                header_payload_pair.push((remaining_gdp_header, remaining_gdp_payload.clone()));
                            } 
                            else{ //overflow!!
                                // only get what's needed
                                remaining_gdp_payload.append(&mut receiving_buf[..(remaining_gdp_header.length - remaining_gdp_payload.len())].to_vec());
                                warn!("Bytes are extra!!! {}", read_payload_size - remaining_gdp_header.length);
                                receiving_buf = receiving_buf[(remaining_gdp_header.length - remaining_gdp_payload.len())..].to_vec();
                            }
                        }

                        let (processed_gdp_packets, processed_remaining_header) = parse_header_payload_pairs(receiving_buf.to_vec());

                        for (header, payload) in processed_gdp_packets {
                            let deserialized = header; //TODO: change the var name here
        
                            info!("the total received payload with size {:} with gdp header length {}",  payload.len(), header.length);

                            if deserialized.action == GdpAction::Forward {
                                let packet = construct_gdp_forward_from_bytes(deserialized.destination, thread_name, payload); //todo
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
                        }

                        match processed_remaining_header {
                            Some((header, payload)) => {
                                remaining_gdp_header = header;
                                remaining_gdp_payload = payload;
                                need_more_data_for_previous_header = true;
                            },
                            None => {
                                need_more_data_for_previous_header = false;
                            }
                        }
                        
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    },
                    Err(_e) => {
                        continue;
                    }
                }

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
    m_tx: UnboundedSender<GDPPacket>, mut m_rx: UnboundedReceiver<GDPPacket>
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

/// does not go to rib when peering
pub async fn tcp_to_peer_direct(
    addr: String,
    rib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>,
    peer_tx: UnboundedSender<GDPPacket>,   // used
    peer_rx: UnboundedReceiver<GDPPacket>, // used to send packet over the network
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
    info!("TCP connection takes gdp name {:?}", m_gdp_name);

    handle_tcp_stream(stream, &rib_tx, &channel_tx, peer_tx, peer_rx, m_gdp_name).await;
}
