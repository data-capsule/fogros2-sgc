use crate::network::udpstream::{UdpListener, UdpStream};
use crate::pipeline::{populate_gdp_struct_from_bytes, proc_gdp_packet};
use std::{net::SocketAddr, pin::Pin, str::FromStr};

use openssl::{
    pkey::PKey,
    ssl::{Ssl, SslAcceptor, SslConnector, SslContext, SslMethod, SslVerifyMode},
    x509::X509,
};

use crate::structs::{GDPChannel, GDPPacket, Packet};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{self, UnboundedSender};
use std::time::{SystemTime, UNIX_EPOCH};

const UDP_BUFFER_SIZE: usize = 4096; // 17kb

static SERVER_CERT: &'static [u8] = include_bytes!("../../resources/router.pem");
static SERVER_KEY: &'static [u8] = include_bytes!("../../resources/router-private.pem");
const SERVER_DOMAIN: &'static str = "pourali.com";
fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
/// helper function of SSL
fn ssl_acceptor(certificate: &[u8], private_key: &[u8]) -> std::io::Result<SslContext> {
    let mut acceptor_builder = SslAcceptor::mozilla_intermediate(SslMethod::dtls())?;
    acceptor_builder.set_certificate(&&X509::from_pem(certificate)?)?;
    acceptor_builder.set_private_key(&&PKey::private_key_from_pem(private_key)?)?;
    acceptor_builder.check_private_key()?;
    let acceptor = acceptor_builder.build();
    Ok(acceptor.into_context())
}

/// handle one single session of dtls
/// 1. init and advertise the mpsc channel to connection rib
/// 2. select between
///         incoming dtls packets -> receive and send to rib
///         incomine packets from rib -> send to the tcp session
async fn handle_dtls_stream(
    socket: UdpStream, acceptor: SslContext, rib_tx: &UnboundedSender<GDPPacket>,
    channel_tx: &UnboundedSender<GDPChannel>,
) {
    let (m_tx, mut m_rx) = mpsc::unbounded_channel();
    let ssl = Ssl::new(&acceptor).unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, socket).unwrap();
    Pin::new(&mut stream).accept().await.unwrap();

    loop {
        // TODO:
        // Question: what's the bahavior here, will it keep allocating memory?
        let mut buf = vec![0u8; UDP_BUFFER_SIZE];
        // Wait for the UDP socket to be readable
        // or new data to be sent
        tokio::select! {
            Some(pkt_to_forward) = m_rx.recv() => {
                let pkt_to_forward: GDPPacket = pkt_to_forward;
                // stream.write_all(&packet.payload[..packet.payload.len()]).await.unwrap();
                let payload = pkt_to_forward.get_byte_payload().unwrap();
                stream.write_all(&payload[..payload.len()]).await.unwrap();
            }
            // _ = do_stuff_async()
            // async read is cancellation safe
            _ = stream.read(&mut buf) => {
                // NOTE: if we want real time system bound
                // let n = match timeout(Duration::from_millis(UDP_TIMEOUT), stream.read(&mut buf))
                let pkt = populate_gdp_struct_from_bytes(buf.to_vec());
                proc_gdp_packet(pkt,  // packet
                    rib_tx,  //used to send packet to rib
                    channel_tx, // used to send GDPChannel to rib
                    &m_tx //the sending handle of this connection
                ).await;
            },
        }
    }
}

pub async fn dtls_listener(
    addr: String, rib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>,
) {
    let listener = UdpListener::bind(SocketAddr::from_str(&addr).unwrap())
        .await
        .unwrap();
    let acceptor = ssl_acceptor(SERVER_CERT, SERVER_KEY).unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let rib_tx = rib_tx.clone();
        let channel_tx = channel_tx.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(
            async move { handle_dtls_stream(socket, acceptor, &rib_tx, &channel_tx).await },
        );
    }
}


pub async fn dtls_to_peer(addr: String, 
    rib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>) {
    let (m_tx, mut m_rx) = mpsc::unbounded_channel();
    let stream = UdpStream::connect(SocketAddr::from_str(&addr).unwrap()).await.unwrap();
    println!("{:?}", stream);

    // setup ssl
    let mut connector_builder = SslConnector::builder(SslMethod::dtls()).unwrap();
    connector_builder.set_verify(SslVerifyMode::NONE);
    let connector = connector_builder.build().configure().unwrap();
    let ssl = connector.into_ssl(SERVER_DOMAIN).unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, stream).unwrap();
    println!("{:?}", stream);
    Pin::new(&mut stream).connect().await.unwrap();
    //stream.connect().await.unwrap();

    // // read: separate thread
    // let _dtls_sender_handle = tokio::spawn(async move {
    //     loop {
    //         let mut buf = vec![0u8; 1024];
    //         let n = rd.read(&mut buf).await.unwrap();
    //         print!("-> {}", String::from_utf8_lossy(&buf[..n]));
    //     }
    // });
    // loop{
    //     let payload = "ADV,2".as_bytes();
    //     stream.write_all(&payload[..payload.len()]).await.unwrap();
    // }

        // split the stream into read half and write half
        let (mut rd, mut wr) = tokio::io::split(stream);

        // read: separate thread
        let _dtls_sender_handle = tokio::spawn(async move {
            loop {
                let mut buf = vec![0u8; 1024];
                let n = rd.read(&mut buf).await.unwrap();
                print!("-> {}", String::from_utf8_lossy(&buf[..n]));
            }
        });
    
        loop {
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer).unwrap();
            wr.write_all(buffer.as_bytes()).await.unwrap();
        }


    loop {
        // TODO:
        // Question: what's the bahavior here, will it keep allocating memory?
        let mut buf = vec![0u8; UDP_BUFFER_SIZE];
        // Wait for the UDP socket to be readable
        // or new data to be sent
        tokio::select! {
            Some(pkt_to_forward) = m_rx.recv() => {
                let pkt_to_forward: GDPPacket = pkt_to_forward;
                // stream.write_all(&packet.payload[..packet.payload.len()]).await.unwrap();
                let payload = pkt_to_forward.get_byte_payload().unwrap();
                stream.write_all(&payload[..payload.len()]).await.unwrap();
            }
            // _ = do_stuff_async()
            // async read is cancellation safe
            _ = stream.read(&mut buf) => {
                // NOTE: if we want real time system bound
                // let n = match timeout(Duration::from_millis(UDP_TIMEOUT), stream.read(&mut buf))
                let pkt = populate_gdp_struct_from_bytes(buf.to_vec());
                proc_gdp_packet(pkt,  // packet
                    &rib_tx,  //used to send packet to rib
                    &channel_tx, // used to send GDPChannel to rib
                    &m_tx //the sending handle of this connection
                ).await;
            },
        }
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
