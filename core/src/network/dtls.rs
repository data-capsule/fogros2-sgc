use crate::network::udpstream::{UdpListener, UdpStream};
use crate::pipeline::{
    construct_gdp_advertisement_from_bytes, construct_gdp_advertisement_from_structs,
    proc_gdp_packet,
};
use openssl::{
    pkey::PKey,
    ssl::{Ssl, SslAcceptor, SslConnector, SslContext, SslMethod, SslVerifyMode},
    x509::X509,
};
use std::fs;
use std::{net::SocketAddr, pin::Pin, str::FromStr};
use tokio_openssl::SslStream;
use utils::app_config::AppConfig;

use crate::pipeline::construct_gdp_forward_from_bytes;
use crate::structs::GDPHeaderInTransit;
use crate::structs::{GDPChannel, GDPPacket, GdpAction, Packet};
use crate::structs::{GDPName, GDPNameRecord};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
const UDP_BUFFER_SIZE: usize = 17480; // 17kb

fn generate_random_gdp_name() -> GDPName {
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
pub fn parse_header_payload_pairs(
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

const SERVER_DOMAIN: &'static str = "not.verified";

/// helper function of SSL
fn ssl_acceptor(certificate: &[u8], private_key: &[u8]) -> std::io::Result<SslContext> {
    let config = AppConfig::fetch().unwrap();
    let ca_cert = format!("./scripts/crypto/{}/ca-root.pem", config.crypto_name);

    let mut acceptor_builder = SslAcceptor::mozilla_intermediate(SslMethod::dtls())?;
    acceptor_builder.set_certificate(&&X509::from_pem(certificate)?)?;
    acceptor_builder.set_private_key(&&PKey::private_key_from_pem(private_key)?)?;
    acceptor_builder.set_verify(
        openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
    );
    acceptor_builder.set_ca_file(ca_cert)?;
    acceptor_builder.check_private_key()?;
    let acceptor = acceptor_builder.build();
    Ok(acceptor.into_context())
}

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// handle one single session of dtls
/// 1. init and advertise the mpsc channel to connection fib
/// 2. select between
///         incoming dtls packets -> receive and send to fib
///         incomine packets from fib -> send to the tcp session
#[allow(unused_assignments)]
async fn handle_dtls_stream(
    mut stream: SslStream<UdpStream>, fib_tx: &UnboundedSender<GDPPacket>,
    channel_tx: &UnboundedSender<GDPChannel>, m_tx: UnboundedSender<GDPPacket>,
    mut m_rx: UnboundedReceiver<GDPPacket>, thread_name: GDPName,
    rib_query_tx: &UnboundedSender<GDPNameRecord>,
) {
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
                            fib_tx,  //used to send packet to fib
                            channel_tx, // used to send GDPChannel to fib
                            &m_tx, //the sending handle of this connection
                            &rib_query_tx,
                            "".to_string(),
                        ).await;
                    }
                    else if deserialized.action == GdpAction::Advertise {
                        info!("received advertise packet from {}", deserialized.destination);
                        let packet = construct_gdp_advertisement_from_bytes(deserialized.destination, thread_name, payload);
                        proc_gdp_packet(packet,  // packet
                            fib_tx,  //used to send packet to fib
                            channel_tx, // used to send GDPChannel to fib
                            &m_tx, //the sending handle of this connection
                            &rib_query_tx,
                            format!("DTLS Advertise {} from thread {}", deserialized.destination, thread_name),
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
                            fib_tx,  //used to send packet to fib
                            channel_tx, // used to send GDPChannel to fib
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
                //info!("TCP packet to forward: {:?}", pkt_to_forward);
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
}

pub async fn dtls_to_peer(
    addr: String, fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
    m_tx: UnboundedSender<GDPPacket>, m_rx: UnboundedReceiver<GDPPacket>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let stream = UdpStream::connect(SocketAddr::from_str(&addr).unwrap())
        .await
        .unwrap();
    println!("{:?}", stream);

    let config = AppConfig::fetch().unwrap();
    let ca_cert = format!("./scripts/crypto/{}/ca-root.pem", config.crypto_name);
    let my_cert = format!(
        "./scripts/crypto/{}/{}.pem",
        config.crypto_name, config.crypto_name
    );
    let my_key = format!(
        "./scripts/crypto/{}/{}-private.pem",
        config.crypto_name, config.crypto_name
    );

    // setup ssl
    let client_cert = X509::from_pem(&fs::read(my_cert).expect("file does not exist")).unwrap();
    let client_key =
        PKey::private_key_from_pem(&fs::read(my_key).expect("file does not exist")).unwrap();
    let mut connector_builder = SslConnector::builder(SslMethod::dtls()).unwrap();
    connector_builder.set_certificate(&client_cert).unwrap();
    connector_builder.set_private_key(&client_key).unwrap();
    connector_builder.set_ca_file(ca_cert).unwrap();
    connector_builder.set_verify(
        openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
    );
    let mut connector = connector_builder.build().configure().unwrap();
    connector.set_verify_hostname(false);
    let ssl = connector.into_ssl(SERVER_DOMAIN).unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, stream).unwrap();
    println!("{:?}", stream);
    Pin::new(&mut stream).connect().await.unwrap();

    let m_gdp_name = generate_random_gdp_name();
    info!("DTLS takes gdp name {:?}", m_gdp_name);

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
        &rib_query_tx,
        format!("DTLS to peer {}", addr),
    )
    .await;
    handle_dtls_stream(
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

/// does not go to fib when peering
pub async fn dtls_to_peer_direct(
    addr: String,
    fib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>,
    peer_tx: UnboundedSender<GDPPacket>,   // used
    peer_rx: UnboundedReceiver<GDPPacket>, // used to send packet over the network
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let stream = UdpStream::connect(SocketAddr::from_str(&addr).unwrap())
        .await
        .unwrap();
    println!("{:?}", stream);

    // setup ssl
    let config = AppConfig::fetch().unwrap();
    let ca_cert = format!("./scripts/crypto/{}/ca-root.pem", config.crypto_name);
    let my_cert = format!(
        "./scripts/crypto/{}/{}.pem",
        config.crypto_name, config.crypto_name
    );
    let my_key = format!(
        "./scripts/crypto/{}/{}-private.pem",
        config.crypto_name, config.crypto_name
    );

    let client_cert = X509::from_pem(&fs::read(my_cert).expect("file does not exist")).unwrap();
    let client_key =
        PKey::private_key_from_pem(&fs::read(my_key).expect("file does not exist")).unwrap();

    let mut connector_builder = SslConnector::builder(SslMethod::dtls()).unwrap();
    connector_builder.set_certificate(&client_cert).unwrap();
    connector_builder.set_private_key(&client_key).unwrap();
    connector_builder.set_ca_file(ca_cert).unwrap();
    connector_builder.set_verify(
        openssl::ssl::SslVerifyMode::PEER | openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT,
    );
    let mut connector = connector_builder.build().configure().unwrap();
    connector.set_verify_hostname(false);
    let ssl = connector.into_ssl(SERVER_DOMAIN).unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, stream).unwrap();
    println!("{:?}", stream);
    Pin::new(&mut stream).connect().await.unwrap();

    let m_gdp_name = generate_random_gdp_name();
    info!("dTLS connection takes gdp name {:?}", m_gdp_name);

    handle_dtls_stream(
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

pub async fn dtls_listener(
    addr: String, fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let config = AppConfig::fetch().unwrap();
    let _ca_cert = format!("./scripts/crypto/{}/ca-root.pem", config.crypto_name);
    let my_cert = format!(
        "./scripts/crypto/{}/{}.pem",
        config.crypto_name, config.crypto_name
    );
    let my_key = format!(
        "./scripts/crypto/{}/{}-private.pem",
        config.crypto_name, config.crypto_name
    );

    let listener = UdpListener::bind(SocketAddr::from_str(&addr).unwrap())
        .await
        .unwrap();
    let acceptor = ssl_acceptor(
        &fs::read(my_cert).expect("file does not exist"),
        &fs::read(my_key).expect("file does not exist"),
    )
    .expect("ssl acceptor failed");
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let fib_tx = fib_tx.clone();
        let channel_tx = channel_tx.clone();
        let rib_query_tx = rib_query_tx.clone();
        let acceptor = acceptor.clone();
        // TODO: loop here is not correct
        tokio::spawn(async move {
            let (m_tx, m_rx) = mpsc::unbounded_channel();
            let ssl = Ssl::new(&acceptor).unwrap();
            let mut stream = tokio_openssl::SslStream::new(ssl, socket).unwrap();
            Pin::new(&mut stream).accept().await.unwrap();
            let m_gdp_name = generate_random_gdp_name();
            info!("DTLS listener takes gdp name {:?}", m_gdp_name);
            handle_dtls_stream(
                stream,
                &fib_tx,
                &channel_tx,
                m_tx,
                m_rx,
                m_gdp_name,
                &rib_query_tx,
            )
            .await
        });
    }
}

#[tokio::main]
pub async fn dtls_test_client(addr: String) -> std::io::Result<SslContext> {
    let stream = UdpStream::connect(SocketAddr::from_str(&addr).unwrap()).await?;
    println!("{:?}", stream);
    // setup ssl
    let mut connector_builder = SslConnector::builder(SslMethod::dtls())?;
    connector_builder.set_verify(SslVerifyMode::NONE);
    let connector = connector_builder.build().configure().unwrap();
    let ssl = connector.into_ssl("128.32.37.48").unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, stream).unwrap();
    println!("{:?}", stream);
    Pin::new(&mut stream).connect().await.unwrap();

    // split the stream into read half and write half
    let (mut rd, mut wr) = tokio::io::split(stream);

    // read: separate thread
    let _dtls_sender_handle = tokio::spawn(async move {
        loop {
            let mut buf = vec![0u8; 1024];
            let n = rd.read(&mut buf).await.unwrap();
            print!("-> {}", String::from_utf8_lossy(&buf[..n]));
            println!("{}", get_epoch_ms());
        }
    });

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        wr.write_all(buffer.as_bytes()).await?;
        println!("{}", get_epoch_ms());
    }
}
