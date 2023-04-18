use crate::network::dtls::parse_header_payload_pairs;
use std::sync::Arc;

use crate::pipeline::{construct_gdp_advertisement_from_bytes, proc_gdp_packet};

use crate::pipeline::construct_gdp_forward_from_bytes;
use crate::structs::GDPHeaderInTransit;
use crate::structs::{generate_random_gdp_name, GDPName, GDPNameRecord};
use crate::structs::{GDPChannel, GDPPacket, GdpAction, Packet};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
const UDP_BUFFER_SIZE: usize = 17480; // 17kb

use async_datachannel::{DataStream, Message, PeerConnection, RtcConfig};
use async_tungstenite::{tokio::connect_async, tungstenite};
use futures::{
    channel::mpsc,
    io::{AsyncReadExt, AsyncWriteExt},
    SinkExt, StreamExt,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Works with the signalling server from https://github.com/paullouisageneau/libdatachannel/tree/master/examples/signaling-server-rust
/// Start two shells
/// 1. RUST_LOG=debug cargo run --example smoke -- ws://127.0.0.1:8000 other_peer
/// 2. RUST_LOG=debug cargo run --example smoke -- ws://127.0.0.1:8000 initiator other_peer

#[derive(Debug, Serialize, Deserialize)]
struct SignalingMessage {
    // id of the peer this messaged is supposed for
    id: String,
    payload: Message,
}

pub async fn register_webrtc_stream(my_id: String, peer_to_dial: Option<String>) -> DataStream {
    let ice_servers = vec!["stun:stun.l.google.com:19302"];
    let conf = RtcConfig::new(&ice_servers);
    let (tx_sig_outbound, mut rx_sig_outbound) = mpsc::channel(32);
    let (mut tx_sig_inbound, rx_sig_inbound) = mpsc::channel(32);
    let listener = PeerConnection::new(&conf, (tx_sig_outbound, rx_sig_inbound)).unwrap();

    let signaling_uri = "ws://128.32.37.42:8000";
    let signaling_uri = format!("{}/{}", signaling_uri, my_id);
    info!("Trying to connect to {}", signaling_uri);

    let (mut write, mut read) = connect_async(&signaling_uri).await.unwrap().0.split();

    let other_peer = Arc::new(Mutex::new(peer_to_dial.clone()));
    let other_peer_c = other_peer.clone();
    let f_write = async move {
        while let Some(m) = rx_sig_outbound.next().await {
            let m = SignalingMessage {
                payload: m,
                id: other_peer_c.lock().as_ref().cloned().unwrap(),
            };
            let s = serde_json::to_string(&m).unwrap();
            debug!("Sending {:?}", s);
            write.send(tungstenite::Message::text(s)).await.unwrap();
        }
        anyhow::Result::<_, anyhow::Error>::Ok(())
    };
    tokio::spawn(f_write);
    let f_read = async move {
        while let Some(Ok(m)) = read.next().await {
            debug!("received {:?}", m);
            if let Some(val) = match m {
                tungstenite::Message::Text(t) => {
                    Some(serde_json::from_str::<serde_json::Value>(&t).unwrap())
                }
                tungstenite::Message::Binary(b) => Some(serde_json::from_slice(&b[..]).unwrap()),
                tungstenite::Message::Close(_) => panic!(),
                _ => None,
            } {
                let c: SignalingMessage = serde_json::from_value(val).unwrap();
                println!("msg {:?}", c);
                other_peer.lock().replace(c.id);
                if tx_sig_inbound.send(c.payload).await.is_err() {
                    panic!()
                }
            }
        }
        anyhow::Result::<_, anyhow::Error>::Ok(())
    };

    tokio::spawn(f_read);
    let stream = if peer_to_dial.is_some() {
        // here we are the initiator
        let dc = listener.dial("whatever").await.unwrap();
        info!("dial succeed");

        // dc.write_all(b"Ping").await.unwrap();
        dc
    } else {
        let dc = listener.accept().await.unwrap();
        info!("accept succeed");
        dc
    };
    stream
}

#[allow(unused_assignments)]
pub async fn webrtc_reader_and_writer(
    mut stream: DataStream, fib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>, rib_query_tx: UnboundedSender<GDPNameRecord>,
    m_tx: UnboundedSender<GDPPacket>, mut m_rx: UnboundedReceiver<GDPPacket>,
) {
    // tracing_subscriber::fmt::init();
    // let mut stream = register_webrtc_stream(my_id, peer_to_dial).await;

    let thread_name: GDPName = generate_random_gdp_name();
    let mut need_more_data_for_previous_header = false;
    let mut remaining_gdp_header: GDPHeaderInTransit = GDPHeaderInTransit {
        action: GdpAction::Noop,
        destination: GDPName([0u8, 0, 0, 0]),
        length: 0, // doesn't have any payload
    };
    let mut remaining_gdp_payload: Vec<u8> = vec![];
    let mut reset_counter = 0; // TODO: a temporary counter to reset the connection

    loop {
        let mut receiving_buf = vec![0u8; UDP_BUFFER_SIZE];
        // Wait for the UDP socket to be readable
        // or new data to be sent
        tokio::select! {
            // _ = do_stuff_async()
            // async read is cancellation safe
            Ok(receiving_buf_size) = stream.read(&mut receiving_buf) => {
                // let receiving_buf_size = receiving_buf.len();
                let mut receiving_buf = receiving_buf[..receiving_buf_size].to_vec();
                info!("read {} bytes", receiving_buf_size);

                let mut header_payload_pair = vec!();

                // last time it has incomplete buffer to complete
                if need_more_data_for_previous_header {
                    let read_payload_size = remaining_gdp_payload.len() + receiving_buf_size;
                    if remaining_gdp_header.action == GdpAction::Noop {
                        warn!("last time it has incomplete buffer to complete, the action is Noop.");
                        // receiving_buf.append(&mut remaining_gdp_payload.clone());
                        remaining_gdp_payload.append(&mut receiving_buf[..receiving_buf_size].to_vec());
                        receiving_buf = remaining_gdp_payload.clone();
                        reset_counter += 1;
                        if reset_counter >5 {
                            error!("unable to match the buffer, reset the connection");
                            receiving_buf = vec!();
                            remaining_gdp_payload = vec!();
                            reset_counter = 0;
                        }
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
                        // info!("remaining_gdp_payload {:.unwrap()}", remaining_gdp_payload);

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
                            &fib_tx,  //used to send packet to fib
                            &channel_tx, // used to send GDPChannel to fib
                            &m_tx, //the sending handle of this connection
                            &rib_query_tx,
                            "".to_string(),
                        ).await;
                    }
                    else if deserialized.action == GdpAction::Advertise {
                        let packet = construct_gdp_advertisement_from_bytes(deserialized.destination, thread_name, payload);
                        proc_gdp_packet(packet,  // packet
                            &fib_tx,  //used to send packet to fib
                            &channel_tx, // used to send GDPChannel to fib
                            &m_tx, //the sending handle of this connection
                            &rib_query_tx,
                            format!("WebRTC Advertise {} from thread {}", deserialized.destination, thread_name),
                        ).await;
                    }
                    else if deserialized.action == GdpAction::RibGet {
                        let name_record:GDPNameRecord = serde_json::from_slice(&payload).unwrap();
                        info!("received RIB get request {:?}", name_record);
                        rib_query_tx.send(name_record).expect("send to rib failure");
                    }
                    else if deserialized.action == GdpAction::RibReply {
                        let name_record:GDPNameRecord = serde_json::from_slice(&payload).unwrap();
                        info!("received RIB get request {:?}", name_record);
                        rib_query_tx.send(name_record).expect("send to rib failure");
                    }
                    else if deserialized.action == GdpAction::AdvertiseResponse {
                        let packet = construct_gdp_advertisement_from_bytes(deserialized.destination, thread_name, payload);
                        proc_gdp_packet(packet,  // packet
                            &fib_tx,  //used to send packet to fib
                            &channel_tx, // used to send GDPChannel to fib
                            &m_tx, //the sending handle of this connection
                            &rib_query_tx,
                            format!("DTLS Advertise Response {} from thread {}", deserialized.destination, thread_name),
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

            Some(pkt_to_forward) = m_rx.recv() => {
                //info!("TCP packet to forward: {:.unwrap()}", pkt_to_forward);
                let transit_header = pkt_to_forward.get_header();
                let mut header_string = serde_json::to_string(&transit_header).unwrap();
                info!("the header size is {}", header_string.len());
                info!("the header to sent is {}", header_string);

                //insert the first null byte to separate the packet header
                header_string.push(0u8 as char);
                let header_string_payload = header_string.as_bytes();
                stream.write_all(&header_string_payload[..header_string_payload.len()]).await.unwrap();

                // stream.write_all(&packet.payload[..packet.payload.len()]).await.unwrap();
                if let Some(payload) = pkt_to_forward.payload {
                    info!("the payload length is {}", payload.len());
                    stream.write_all(&payload[..payload.len()]).await.unwrap();
                }

                if let Some(name_record) = pkt_to_forward.name_record {
                    let name_record_string = serde_json::to_string(&name_record).unwrap();
                    let name_record_buffer = name_record_string.as_bytes();
                    info!("the name record length is {}", name_record_buffer.len());
                    stream.write_all(&name_record_buffer[..name_record_buffer.len()]).await.unwrap();
                }
            }
        }
    }
    // loop {
    //     let n = dc.read(&mut buf).await.unwrap();
    //     println!("Read: \"{}\"", String::from_utf8_lossy(&buf[..n]));
    //     dc.write_all(b"Ping").await.unwrap();
    //     tokio::time::sleep(Duration::from_secs(2)).await;
    // }
}
