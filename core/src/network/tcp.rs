use crate::pipeline::proc_gdp_packet;
use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction};
use std::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Sender};

const UDP_BUFFER_SIZE: usize = 17480; // 17kb TODO: make it formal

/// handle one single session of tcpstream
/// 1. init and advertise the mpsc channel to connection rib
/// 2. select between
///         incoming tcp packets -> receive and send to rib
///         incomine packets from rib -> send to the tcp session
async fn handle_tcp_stream(
    stream: TcpStream, rib_tx: &Sender<GDPPacket>, channel_tx: &Sender<GDPChannel>,
) {
    // ...
    let (m_tx, mut m_rx) = mpsc::channel(32);
    // TODO: placeholder, later replace with packet parsing

    loop {
        // Wait for the TCP socket to be readable
        // or new data to be sent
        tokio::select! {
            // new stuff from TCP!
            _f = stream.readable() => {
                // Creating the buffer **after** the `await` prevents it from
                // being stored in the async task.

                let mut buf = vec![0u8; UDP_BUFFER_SIZE];
                // Try to read data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        println!("read {} bytes", n);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(_e) => {
                        continue;
                    }
                }

                proc_gdp_packet(buf,  // packet
                    rib_tx,  //used to send packet to rib
                    channel_tx, // used to send GDPChannel to rib
                    &m_tx //the sending handle of this connection
                ).await;

            },

            // new data to send to TCP!
            Some(pkt_to_forward) = m_rx.recv() => {
                stream.writable().await; // okay this may have deadlock

                // Try to write data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_write(&pkt_to_forward.payload) {
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
            },
        }
    }
}

/// listen at @param address and process on tcp accept()
///     rib_tx: channel that send GDPPacket to rib
///     channel_tx: channel that advertise GDPChannel to rib
pub async fn tcp_listener(
    addr: &'static str, rib_tx: Sender<GDPPacket>, channel_tx: Sender<GDPChannel>,
) {
    let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let rib_tx = rib_tx.clone();
        let channel_tx = channel_tx.clone();

        // Process each socket concurrently.
        tokio::spawn(async move { 
            handle_tcp_stream(socket, &rib_tx, &channel_tx).await 
        });
    }
}
