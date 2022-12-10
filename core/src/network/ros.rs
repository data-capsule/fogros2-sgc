use futures::future;
use futures::stream::StreamExt;
use futures::task::LocalSpawnExt;
use futures::{executor::LocalPool};
use pnet::packet;
use tokio::sync::mpsc::{self, Sender, Receiver};
use r2r::QosProfile;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_json; 
use crate::pipeline::{populate_gdp_struct_from_bytes, proc_gdp_packet};
use crate::structs::{GDPChannel, GDPPacket, Packet};

pub fn ros_sample() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = r2r::Context::create()?;
    let mut node = r2r::Node::create(ctx, "node", "namespace")?;
    let subscriber =
        node.subscribe::<r2r::std_msgs::msg::String>("/topic", QosProfile::default())?;
    let publisher =
        node.create_publisher::<r2r::std_msgs::msg::String>("/topic", QosProfile::default())?;
    let mut timer = node.create_wall_timer(std::time::Duration::from_millis(1000))?;

    // Set up a simple task executor.
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    // Run the subscriber in one task, printing the messages
    spawner.spawn_local(async move {
        subscriber
            .for_each(|msg| {
                println!("got new msg: {}", msg.data);
                future::ready(())
            })
            .await
    })?;

    // Run the publisher in another task
    spawner.spawn_local(async move {
        let mut counter = 0;
        loop {
            let _elapsed = timer.tick().await.unwrap();
            let msg = r2r::std_msgs::msg::String {
                data: format!("Hello, world! ({})", counter),
            };
            publisher.publish(&msg).unwrap();
            counter += 1;
        }
    })?;

    // Main loop spins ros.
    loop {
        node.spin_once(std::time::Duration::from_millis(100));
        pool.run_until_stalled();
    }
}

pub async fn ros_listener(rib_tx: Sender<GDPPacket>, channel_tx: Sender<GDPChannel>) {

    let (m_tx, mut m_rx) = mpsc::channel::<GDPPacket>(32);
    let ctx = r2r::Context::create().expect("context creation failure");
    let mut node =
        r2r::Node::create(ctx, "GDP_Router", "namespace").expect("node creation failure");
    let mut subscriber = node
        .subscribe_untyped("/topic", "std_msgs/msg/String", QosProfile::default())
        .expect("topic subscribing failure");
    let publisher = node.create_publisher_untyped("/topic", "std_msgs/msg/String", QosProfile::default()).expect("publisher creation failure");

    // subscriber
    //     .for_each(|msg| {
    //         println!("got new msg: {}", msg.data);
    //         future::ready(())
    //     })
    //     .await;
    // loop {
    //     node.spin_once(std::time::Duration::from_millis(100));
    // }

    let handle = tokio::task::spawn_blocking(move || loop {
        node.spin_once(std::time::Duration::from_millis(100));
    });

    // subscriber.for_each(|msg| {
    //     println!("topic: new msg: {}", msg.data);
    //     let packet = populate_gdp_struct_from_bytes( msg.data.into_bytes());
    //     // proc_gdp_packet(packet,  // packet
    //     //     &rib_tx.clone(),  //used to send packet to rib
    //     //     &channel_tx.clone(), // used to send GDPChannel to rib
    //     //     &m_tx //the sending handle of this connection
    //     // );
    //     future::ready(())
    // })
    // .await;

    loop {
        tokio::select! {
            Some(packet) = subscriber.next() => {
                println!("received a packet {:?}", packet);
                let ros_msg = serde_json::to_vec(&packet.unwrap()).unwrap();
                
                let packet = populate_gdp_struct_from_bytes(ros_msg);
                proc_gdp_packet(packet,  // packet
                    &rib_tx,  //used to send packet to rib
                    &channel_tx, // used to send GDPChannel to rib
                    &m_tx //the sending handle of this connection
                ).await;
            
            }
            Some(pkt_to_forward) = m_rx.recv() => {
                // okay this may have deadlock

                let payload = pkt_to_forward.get_byte_payload().unwrap();
                // let msg = r2r::std_msgs::msg::String {
                //     data: format!("Hello, world! ({:?})", payload),
                // };
                let json = format!("{{ \"data\": {:?} }}", payload);
                let ros_msg = serde_json::from_str(&json).unwrap();
                publisher.publish(ros_msg).unwrap();
            },


        }
    }

    // handle.await;

    // Run the subscriber in one task, printing the messages
    // spawner.spawn_local(async move {
    //     subscriber.for_each(|msg| {
    //         println!("got new msg: {}", msg.data);
    //         future::ready(())
    //     }).await
    // });
}
