use crate::pipeline::construct_gdp_advertisement_from_bytes;
use crate::pipeline::construct_gdp_advertisement_from_structs;
use crate::pipeline::proc_gdp_packet;

use crate::structs::generate_random_gdp_name;
use crate::structs::GDPName;
use crate::structs::{GDPChannel, GDPPacket, GdpAction, Packet};
use std::io;

use std::{net::SocketAddr, str::FromStr};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
const UDP_BUFFER_SIZE: usize = 17480; // 17480 17kb TODO: make it formal
use crate::pipeline::construct_gdp_forward_from_bytes;
use crate::structs::GDPHeaderInTransit;

use crate::structs::GDPNameRecord;

/// parse the header of the packet using the first null byte as delimiter
/// return a vector of (header, payload) pairs if the header is complete
/// return the remaining (header, payload) pairs if the header is incomplete
fn parse_header_payload_pairs(
    mut buffer: Vec<u8>,
) -> (
    Vec<(GDPHeaderInTransit, Vec<u8>)>,
    Option<(GDPHeaderInTransit, Vec<u8>)>,
) {
    let mut header_payload_pairs: Vec<(GDPHeaderInTransit, Vec<u8>)> = Vec::new();
    // TODO: get it to default trace later
    let default_gdp_header: GDPHeaderInTransit = GDPHeaderInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, // doesn't have any payload
    };
    if buffer.len() == 0 {
        return (header_payload_pairs, None);
    }
    loop {
        // parse the header
        // use the first null byte \0 as delimiter
        // split to the first \0 as delimiter
        let header_and_remaining = buffer.splitn(2, |c| c == &0).collect::<Vec<_>>();
        let header_buf = header_and_remaining[0];
        let header: &str = std::str::from_utf8(header_buf).unwrap();
        info!("received header json string: {:?}", header);
        let gdp_header_parsed = serde_json::from_str::<GDPHeaderInTransit>(header);
        if gdp_header_parsed.is_err() {
            // if the header is not complete, return the remaining
            warn!("header is not complete, return the remaining");
            return (
                header_payload_pairs,
                Some((default_gdp_header, header_buf.to_vec())),
            );
        }
        let gdp_header = gdp_header_parsed.unwrap();
        let remaining = header_and_remaining[1];

        if gdp_header.length > remaining.len() {
            // if the payload is not complete, return the remaining
            return (header_payload_pairs, Some((gdp_header, remaining.to_vec())));
        } else if gdp_header.length == remaining.len() {
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
/// 1. init and advertise the mpsc channel to connection fib
/// 2. select between
///         incoming tcp packets -> receive and send to fib
///         incomine packets from fib -> send to the tcp session
async fn handle_tcp_stream(
    stream: TcpStream, fib_tx: &UnboundedSender<GDPPacket>,
    channel_tx: &UnboundedSender<GDPChannel>, m_tx: UnboundedSender<GDPPacket>,
    mut m_rx: UnboundedReceiver<GDPPacket>, thread_name: GDPName,
    rib_query_tx: &UnboundedSender<GDPNameRecord>,
) {
    // ...

    // init variables

    let mut need_more_data_for_previous_header = false;
    let mut remaining_gdp_header: GDPHeaderInTransit = GDPHeaderInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, // doesn't have any payload
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
                        info!("read {} bytes", receiving_buf_size);

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
                            if remaining_gdp_header.action == GdpAction::Noop {
                                warn!("last time it has incomplete buffer to complete, the action is Noop.");
                                // receiving_buf.append(&mut remaining_gdp_payload.clone());
                                remaining_gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                                receiving_buf = remaining_gdp_payload.clone();
                            }
                            else if read_payload_size < remaining_gdp_header.length { //still need more things to read!
                                info!("more data to read. Current {}, need {}, expect {}", read_payload_size, remaining_gdp_header.length, remaining_gdp_header.length - read_payload_size);
                                remaining_gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                                continue;
                            }
                            else if read_payload_size == remaining_gdp_header.length { // match the end of the packet
                                remaining_gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                                header_payload_pair.push((remaining_gdp_header, remaining_gdp_payload.clone()));
                                receiving_buf = vec!();
                            }
                            else{ //overflow!!
                                // only get what's needed
                                warn!("The packet is overflowed!!! read_payload_size {}, remaining_gdp_header.length {}, remaining_gdp_payload.len() {}, receiving_buf_size {}", read_payload_size, remaining_gdp_header.length, remaining_gdp_payload.len(), receiving_buf_size);
                                let num_remaining = remaining_gdp_header.length - remaining_gdp_payload.len();
                                remaining_gdp_payload.append(&mut receiving_buf[..num_remaining].to_vec());
                                header_payload_pair.push((remaining_gdp_header, remaining_gdp_payload.clone()));
                                // info!("remaining_gdp_payload {:?}", remaining_gdp_payload);

                                receiving_buf = receiving_buf[num_remaining..].to_vec();
                            }
                        }

                        let (mut processed_gdp_packets, processed_remaining_header) = parse_header_payload_pairs(receiving_buf.to_vec());
                        header_payload_pair.append(&mut processed_gdp_packets);
                        for (header, payload) in header_payload_pair {
                            let deserialized = header; //TODO: change the var name here

                            info!("the total received payload with size {:} with gdp header length {}",  payload.len(), header.length);

                            if deserialized.action == GdpAction::Forward {
                                let packet = construct_gdp_forward_from_bytes(deserialized.destination, thread_name, payload); //todo
                                proc_gdp_packet(packet,  // packet
                                    fib_tx,  //used to send packet tofib
                                    channel_tx, // used to send GDPChannel tofib
                                    &m_tx, //the sending handle of this connection
                                    &rib_query_tx,
                                    "".to_string(),
                                ).await;
                            }
                            else if deserialized.action == GdpAction::Advertise {
                                let packet = construct_gdp_advertisement_from_bytes(deserialized.destination, thread_name, payload);
                                proc_gdp_packet(packet,  // packet
                                    fib_tx,  //used to send packet tofib
                                    channel_tx, // used to send GDPChannel tofib
                                    &m_tx, //the sending handle of this connection
                                    &rib_query_tx,
                                    format!("{}-{}", "tcp", thread_name),
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
                                remaining_gdp_payload = vec!();
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
                info!("the header to sent is {}", header_string);

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
                    let mut bytes_written = 0;
                    let bytes_read = payload.len();
                    while bytes_written < bytes_read {
                        match stream.try_write(&payload[bytes_written..bytes_read]) {
                            Ok(n) => {
                                info!("payload with size {}: write {} bytes", payload.len(), n);
                                bytes_written += n;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                info!("would block");
                                stream.writable().await.expect("TCP stream is closed");
                            }
                            Err(_e) => {
                                info!("Err of other kind");
                                break
                            }
                        }
                    }
                }

                if let Some(name_record) = pkt_to_forward.name_record {
                    let name_record_string = serde_json::to_string(&name_record).unwrap();
                    let name_record_buffer = name_record_string.as_bytes();
                    info!("the name record length is {}", name_record_buffer.len());
                    match stream.try_write(&name_record_buffer[..name_record_buffer.len()]) {
                        Ok(_n) => {
                            info!("advertisement with size {}", name_record_buffer.len());
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            info!("would block");
                            stream.writable().await.expect("TCP stream is closed");
                        }
                        Err(_e) => {
                            info!("Err of other kind");
                            break
                        }
                    }
                }

            },
        }
    }
}

/// listen at @param address and process on tcp accept()
///     fib_tx: channel that send GDPPacket to fib
///     channel_tx: channel that advertise GDPChannel to fib
pub async fn tcp_listener(
    addr: String, fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let listener = TcpListener::bind(&addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let fib_tx = fib_tx.clone();
        let channel_tx = channel_tx.clone();
        let rib_query_tx = rib_query_tx.clone();
        // Process each socket concurrently.
        tokio::spawn(async move {
            let (m_tx, m_rx) = mpsc::unbounded_channel();
            handle_tcp_stream(
                socket,
                &fib_tx,
                &channel_tx,
                m_tx,
                m_rx,
                generate_random_gdp_name(),
                &rib_query_tx,
            )
            .await
        });
    }
}

pub async fn tcp_to_peer(
    addr: String, fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
    m_tx: UnboundedSender<GDPPacket>, m_rx: UnboundedReceiver<GDPPacket>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let stream = match TcpStream::connect(SocketAddr::from_str(&addr).unwrap()).await {
        Ok(s) => s,
        Err(_) => {
            error!("TCP: Unable to connect to the dafault gateway {}", addr);
            return;
        }
    };

    println!("{:?}", stream);

    let m_gdp_name = generate_random_gdp_name();
    info!("TCP takes gdp name {:?}", m_gdp_name);

    let node_advertisement = construct_gdp_advertisement_from_structs(
        m_gdp_name,
        m_gdp_name,
        crate::structs::GDPNameRecord {
            record_type: crate::structs::GDPNameRecordType::UPDATE,
            gdpname: m_gdp_name,
            source_gdpname: m_gdp_name,
            webrtc_offer: None,
            ip_address: Some(addr.clone()),
            indirect: None,
            ros: None,
        },
    );
    proc_gdp_packet(
        node_advertisement, // packet
        &fib_tx,            // used to send packet to fib
        &channel_tx,        // used to send GDPChannel to fib
        &m_tx,              // the sending handle of this connection
        &rib_query_tx,      // used to query rib
        format!("tcp_to_peer({})", addr),
    )
    .await;
    handle_tcp_stream(
        stream,
        &fib_tx,
        &channel_tx,
        m_tx,
        m_rx,
        m_gdp_name,
        &rib_query_tx,
    )
    .await;
}

/// does not go tofib when peering
pub async fn tcp_to_peer_direct(
    addr: String,
    fib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>,
    peer_tx: UnboundedSender<GDPPacket>,   // used
    peer_rx: UnboundedReceiver<GDPPacket>, // used to send packet over the network
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let stream = match TcpStream::connect(SocketAddr::from_str(&addr).unwrap()).await {
        Ok(s) => s,
        Err(_) => {
            error!("TCP: Unable to connect to the dafault gateway {}", addr);
            return;
        }
    };

    println!("{:?}", stream);

    let m_gdp_name = generate_random_gdp_name();
    info!("TCP connection takes gdp name {:?}", m_gdp_name);

    handle_tcp_stream(
        stream,
        &fib_tx,
        &channel_tx,
        peer_tx,
        peer_rx,
        m_gdp_name,
        &rib_query_tx,
    )
    .await;
}
