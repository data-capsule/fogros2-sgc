/**
 * Rust signaling server example for libdatachannel
 * Copyright (c) 2020 Paul-Louis Ageneau
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

extern crate tokio;
extern crate tungstenite;
extern crate futures_util;
extern crate futures_channel;
extern crate json;
extern crate redis;

use std::env;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;
use tungstenite::handshake::server::{Request, Response};

use futures_util::{future, pin_mut, StreamExt};
use futures_util::stream::TryStreamExt;
use futures_channel::mpsc;

type Id = String;
type Tx = mpsc::UnboundedSender<Message>;
type ClientsMap = Arc<Mutex<HashMap<Id, Tx>>>;

fn remove_client_from_rib(client_id: &str){
    let client = redis::Client::open("redis://fogros2-sgc-lite-rib-1").expect("redis client open failed");
    let mut con = client.get_connection().unwrap();
    let topic_key_name = client_id.split(',').take(4).collect::<Vec<&str>>().join(",");
    let publisher_topic = format!("{}-pub", topic_key_name.clone());
    let subscriber_topic = format!("{}-sub", topic_key_name.clone());

    println!("Removing client {} from RIB", client_id);
    println!("client publisher name is {}", publisher_topic);
    let result: Result<String, redis::RedisError> = redis::transaction(&mut con, &[publisher_topic.clone()], |con, pipe| {
        pipe
            .lrem(publisher_topic.clone(), 0, client_id)
            .query(con)
    });
    let subscriber_name = client_id.split(',').skip(4).take(4).collect::<Vec<&str>>().join(",");
    println!("client subscriber name is {} with subscriber name {}", subscriber_topic, subscriber_name);
    let result: Result<String, redis::RedisError> = redis::transaction(&mut con, &[subscriber_topic.clone()], |con, pipe| {
        pipe
            .lrem(subscriber_topic.clone(), 0, subscriber_name.clone())
            .query(con)
    });
}

async fn handle(clients: ClientsMap, stream: TcpStream) {
    let mut client_id = Id::new();
    let callback = |req: &Request, response: Response| {
        let path: &str = req.uri().path();
        let tokens: Vec<&str> = path.split('/').collect();
        client_id = tokens[1].to_string();
        return Ok(response);
    };

    let websocket = tokio_tungstenite::accept_hdr_async(stream, callback)
        .await.expect("WebSocket handshake failed");
	println!("Client {} connected", &client_id);

    let (tx, rx) = mpsc::unbounded();
    clients.lock().unwrap().insert(client_id.clone(), tx);

    let (outgoing, incoming) = websocket.split();
    let forward = rx.map(Ok).forward(outgoing);
    let process = incoming.try_for_each(|msg| {
        if msg.is_text() {
            let text = msg.to_text().unwrap();
            println!("Client {} << {}", &client_id, &text);

            // Parse
            let mut content = json::parse(text).unwrap();
            let remote_id = content["id"].to_string();
            let mut locked = clients.lock().unwrap();

            match locked.get_mut(&remote_id) {
                Some(remote) => {
                    // Format
                    content.insert("id", client_id.clone()).unwrap();
                    let text = json::stringify(content);

                    // Send to remote
                    println!("Client {} >> {}", &remote_id, &text);
                    remote.unbounded_send(Message::text(text)).unwrap();
                },
                _ => println!("Client {} not found", &remote_id),
            }
        }
        future::ok(())
    });

    pin_mut!(process, forward);
    future::select(process, forward).await;

    println!("Client {} disconnected", &client_id);
    clients.lock().unwrap().remove(&client_id);
    remove_client_from_rib(&client_id);
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let service = env::args().nth(1).unwrap_or("8000".to_string());
    let endpoint = if service.contains(':') { service } else { format!("0.0.0.0:{}", service) };

	println!("Listening on {}", endpoint);

    let listener = TcpListener::bind(endpoint)
    	.await.expect("Listener binding failed");

    let clients = ClientsMap::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle(clients.clone(), stream));
    }

    return Ok(())
}

