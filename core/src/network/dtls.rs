use crate::network::udpstream::{UdpListener, UdpStream};
use crate::pipeline::proc_gdp_packet;
use std::{net::SocketAddr, pin::Pin, str::FromStr, time::Duration};

use openssl::{
    pkey::PKey,
    ssl::{Ssl, SslAcceptor, SslConnector, SslContext, SslMethod, SslVerifyMode},
    x509::X509,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction};
use tokio::sync::mpsc::{self, Sender};

const UDP_BUFFER_SIZE: usize = 174; // 17kb
const UDP_TIMEOUT: u64 = 10000 * 1000; // 10000 sec

static SERVER_CERT: &'static [u8] = include_bytes!("../../resources/router.pem");
static SERVER_KEY: &'static [u8] = include_bytes!("../../resources/router-private.pem");
const SERVER_DOMAIN: &'static str = "pourali.com";

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
    socket: UdpStream, acceptor: SslContext, rib_tx: &Sender<GDPPacket>,
    channel_tx: &Sender<GDPChannel>,
) {
    let (m_tx, mut m_rx) = mpsc::channel(32);
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
            // _ = do_stuff_async()
            // async read is cancellation safe
            _ = stream.read(&mut buf) => {
                // NOTE: if we want real time system bound
                // let n = match timeout(Duration::from_millis(UDP_TIMEOUT), stream.read(&mut buf))
                proc_gdp_packet(buf.to_vec(),  // packet
                    rib_tx,  //used to send packet to rib
                    channel_tx, // used to send GDPChannel to rib
                    &m_tx //the sending handle of this connection
                ).await;
            },
        }
    }
}

pub async fn dtls_listener(
    addr: &'static str, rib_tx: Sender<GDPPacket>, channel_tx: Sender<GDPChannel>,
) {
    let listener = UdpListener::bind(SocketAddr::from_str(addr).unwrap())
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

#[tokio::main]
pub async fn dtls_test_client(addr: &'static str) -> std::io::Result<SslContext> {
    let stream = UdpStream::connect(SocketAddr::from_str(addr).unwrap()).await?;

    // setup ssl
    let mut connector_builder = SslConnector::builder(SslMethod::dtls())?;
    connector_builder.set_verify(SslVerifyMode::NONE);
    let connector = connector_builder.build().configure().unwrap();
    let ssl = connector.into_ssl(SERVER_DOMAIN).unwrap();
    let mut stream = tokio_openssl::SslStream::new(ssl, stream).unwrap();
    Pin::new(&mut stream).connect().await.unwrap();

    // split the stream into read half and write half
    let (mut rd, mut wr) = tokio::io::split(stream);

    // read: separate thread
    let dtls_sender_handle = tokio::spawn(async move {
        loop {
            let mut buf = vec![0u8; 1024];
            let n = rd.read(&mut buf).await.unwrap();
            print!("-> {}", String::from_utf8_lossy(&buf[..n]));
        }
    });

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        wr.write_all(buffer.as_bytes()).await?;
    }
}
