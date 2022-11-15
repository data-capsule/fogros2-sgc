
use futures::future; // 0.3.19
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, channel, Sender},
    time,
}; // 1.16.1
use crate::structs::GDPPacket;
use tokio::net::{TcpListener, TcpStream};
use std::io;


async fn process(stream: TcpStream, foo_tx: &Sender<GDPPacket>) {
    // ...
    println!("got packets!!");
    let message = format!("hello");
    loop {
        // Wait for the socket to be readable
        stream.readable().await;

        // Creating the buffer **after** the `await` prevents it from
        // being stored in the async task.
        let mut pkt = GDPPacket{
            packet: [0; 2048],
        };

        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_read(&mut pkt.packet) {
            Ok(0) => break,
            Ok(n) => {
                println!("read {} bytes", n);
                foo_tx.send(pkt).await;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                continue;
            }   
        }
    }
}

pub async fn tcp_listener(msg: &'static str, foo_tx: Sender<GDPPacket>)  {
    let listener = TcpListener::bind(msg).await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let foo_tx = foo_tx.clone();
        // Process each socket concurrently.
        // TODO: currently, it can only handle one concurrent session
        tokio::spawn(async move {
            process(socket, &foo_tx).await
        });
    }
}
