
use futures::future; // 0.3.19
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, channel, Sender},
    time,
}; // 1.16.1
use crate::structs::{GdpAction, GDPPacket, GDPChannel};
use tokio::net::{TcpListener, TcpStream};
use std::io;

async fn process(stream: TcpStream, rib_tx: &Sender<GDPPacket>, channel_tx: &Sender<GDPChannel>) {
    // ...
    println!("got packets!!");
    let (m_tx, mut m_rx) = mpsc::channel(32);
    // TODO: placeholder, later replace with packet parsing 
    let mut placeholder_sent = false;

    loop {
        // Wait for the socket to be readable
        stream.readable().await;

        // Creating the buffer **after** the `await` prevents it from
        // being stored in the async task.
        let mut pkt = GDPPacket{
            action: GdpAction::Noop,
            packet: [0; 2048],
        };

        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_read(&mut pkt.packet) {
            Ok(0) => break,
            Ok(n) => {
                println!("read {} bytes", n);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                continue;
            }   
        }

        //parse the packet 
        
        // TODO: suppose it's an advertisement
        if !placeholder_sent{
            let mut channel = GDPChannel{
                name: "bla".to_owned(),
                channel: m_tx.clone(),
            };
    
            placeholder_sent = true;
            channel_tx.send(channel).await;
        }

        //send the packet 
        rib_tx.send(pkt).await;
    }
}


/// listen at @param address and process on tcp accept()
///     rib_tx: channel that send GDPPacket to rib
pub async fn tcp_listener(msg: &'static str, rib_tx: Sender<GDPPacket>, channel_tx: Sender<GDPChannel>)  {
    let listener = TcpListener::bind(msg).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let rib_tx = rib_tx.clone();
        let channel_tx = channel_tx.clone();
        // Process each socket concurrently.
        // TODO: currently, it can only handle one concurrent session
        tokio::spawn(async move {
            process(socket, &rib_tx, &channel_tx).await
        });
    }
}
