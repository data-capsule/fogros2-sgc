use std::{sync::Arc, time::Duration};

use async_datachannel::{Message, PeerConnection, RtcConfig};
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


pub async fn webrtc_main(
    my_id: String,
    peer_to_dial: Option<String>,
) -> anyhow::Result<()> {
    // tracing_subscriber::fmt::init();

    let ice_servers = vec!["stun:stun.l.google.com:19302"];
    let conf = RtcConfig::new(&ice_servers);
    let (tx_sig_outbound, mut rx_sig_outbound) = mpsc::channel(32);
    let (mut tx_sig_inbound, rx_sig_inbound) = mpsc::channel(32);
    let listener = PeerConnection::new(&conf, (tx_sig_outbound, rx_sig_inbound))?;

    let signaling_uri = "ws://128.32.37.42:8000";
    let signaling_uri = format!("{}/{}", signaling_uri, my_id);
    info!("Trying to connect to {}", signaling_uri);

    let (mut write, mut read) = connect_async(&signaling_uri).await?.0.split();

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
    let mut dc = if peer_to_dial.is_some() { // here we are the initiator
        let mut dc = listener.dial("whatever").await?;
        info!("dial succeed");

        dc.write_all(b"Ping").await?;
        dc
    } else {
        let dc = listener.accept().await?;
        info!("accept succeed");
        dc
    };
    let mut buf = vec![0; 32];

    loop {
        let n = dc.read(&mut buf).await?;
        println!("Read: \"{}\"", String::from_utf8_lossy(&buf[..n]));
        dc.write_all(b"Ping").await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}