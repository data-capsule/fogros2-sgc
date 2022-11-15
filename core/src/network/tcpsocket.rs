
use futures::future; // 0.3.19
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, channel, Sender},
    time,
}; // 1.16.1
use crate::structs::{GdpAction, GDPName, GDPPacket, GDPChannel};
use tokio::net::{TcpListener, TcpStream};
use std::io;

async fn process(stream: TcpStream, 
    rib_tx: &Sender<GDPPacket>, 
    channel_tx: &Sender<GDPChannel>) {
    // ...
    println!("got packets!!");
    let (m_tx, 
        mut m_rx) 
        = mpsc::channel(32);
    // TODO: placeholder, later replace with packet parsing 
    let mut advertised_to_rib = false;
    
    // TODO: we need a pipeline here
    if ! advertised_to_rib{
        let mut channel = GDPChannel{
            gdpname: GDPName([0; 4]),
            channel: m_tx.clone(),
        };
        advertised_to_rib = true;
        channel_tx.send(channel).await;
    }

    loop{
        // Wait for the TCP socket to be readable
        // or new data to be sent
        tokio::select! {        

            // new stuff from TCP!
            f = stream.readable() => {
                // Creating the buffer **after** the `await` prevents it from
                // being stored in the async task.
                let mut pkt = GDPPacket{
                    action: GdpAction::Noop,
                    gdpname: GDPName([0; 4]),
                    payload: [0; 2048],
                };
                // Try to read data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match stream.try_read(&mut pkt.payload) {
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

                //send the packet 
                rib_tx.send(pkt).await;
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
                    Err(e) => {
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
pub async fn tcp_listener(msg: &'static str, 
            rib_tx: Sender<GDPPacket>, 
            channel_tx: Sender<GDPChannel>)  {
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
