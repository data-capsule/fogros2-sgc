
use futures::future; // 0.3.19
use std::time::Duration;
use tokio::{
    sync::mpsc::{self, channel, Sender},
    time,
}; // 1.16.1

use tokio::net::{TcpListener, TcpStream};
use std::io;

pub async fn message_sender(msg: &'static str, foo_tx: Sender<String>) {
    for count in 0.. {
        let message = format!("{msg}{count}");
        foo_tx.send(message);

        if msg == "foo" {
            time::sleep(Duration::from_millis(100)).await;
        } else {
            time::sleep(Duration::from_millis(300)).await;
        }
    }
}


async fn process(socket: TcpStream, foo_tx: &Sender<String>) {
    // ...
    println!("got packets!!");
    let message = format!("hello");
    foo_tx.send(message);
}

pub async fn tcp_listener(msg: &'static str, foo_tx: Sender<String>)  {
    let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        // Process each socket concurrently.
        // TODO: currently, it can only handle one concurrent session
        process(socket, &foo_tx).await
    }
}
