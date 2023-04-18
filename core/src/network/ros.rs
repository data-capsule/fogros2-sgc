use crate::pipeline::{
    construct_gdp_advertisement_from_structs, construct_gdp_forward_from_bytes, proc_gdp_packet,
};
use crate::structs::get_gdp_name_from_topic;
use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction, Packet};
use futures::stream::StreamExt;

#[cfg(feature = "ros")]
use r2r::QosProfile;
use r2r::{sensor_msgs::msg::CompressedImage, std_msgs::msg::Header};

use serde_json;

use crate::structs::GDPNameRecord;
use std::str;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedSender;

#[cfg(feature = "ros")]
pub async fn ros_publisher(
    fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>, node_name: String,
    topic_name: String, topic_type: String, certificate: Vec<u8>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let node_gdp_name = GDPName(get_gdp_name_from_topic(
        &node_name,
        &topic_type,
        &certificate,
    ));
    info!("ROS {} takes gdp name {:?}", node_name, node_gdp_name);

    let topic_gdp_name = GDPName(get_gdp_name_from_topic(
        &topic_name,
        &topic_type,
        &certificate,
    ));
    info!("topic {} takes gdp name {:?}", topic_name, topic_gdp_name);

    let (m_tx, mut m_rx) = unbounded_channel::<GDPPacket>();

    let ctx = r2r::Context::create().expect("context creation failure");
    let mut node = r2r::Node::create(ctx, &node_name, "namespace").expect("node creation failure");
    let publisher = node
        .create_publisher_untyped(&topic_name, &topic_type, QosProfile::default())
        .expect("publisher creation failure");

    let _handle = tokio::task::spawn_blocking(move || loop {
        node.spin_once(std::time::Duration::from_millis(100));
    });

    // note that different from other connectionfibs, we send advertisement ahead of time
    let node_advertisement = construct_gdp_advertisement_from_structs(
        topic_gdp_name,
        node_gdp_name,
        crate::structs::GDPNameRecord {
            record_type: crate::structs::GDPNameRecordType::UPDATE,
            gdpname: node_gdp_name,
            source_gdpname: node_gdp_name,
            webrtc_offer: None,
            ip_address: None,
            indirect: None,
            ros: Some((topic_name.clone(), topic_type.clone())),
        },
    );
    proc_gdp_packet(
        node_advertisement, // packet
        &fib_tx,            // used to send packet to fib
        &channel_tx,        // used to send GDPChannel to fib
        &m_tx,              // the sending handle of this connection
        &rib_query_tx,      // used to send GDPNameRecord to rib
        format!("ros publisher {}-{}", topic_name, topic_type),
    )
    .await;

    loop {
        tokio::select! {
            Some(pkt_to_forward) = m_rx.recv() => {
                if pkt_to_forward.action == GdpAction::Forward {
                    info!("new payload to publish ");
                    if pkt_to_forward.gdpname == topic_gdp_name {
                        let payload = pkt_to_forward.get_byte_payload().unwrap();
                        let ros_msg = serde_json::from_str(str::from_utf8(payload).unwrap()).expect("json parsing failure");
                        // info!("the decoded payload to publish is {:?}", ros_msg);
                        publisher.publish(ros_msg).unwrap();
                    } else{
                        info!("{:?} received a packet for name {:?}",pkt_to_forward.gdpname, topic_gdp_name);
                    }
                }
            },
        }
    }
}

#[cfg(feature = "ros")]
pub async fn ros_subscriber(
    fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>, node_name: String,
    topic_name: String, topic_type: String, certificate: Vec<u8>,
    rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let node_gdp_name = GDPName(get_gdp_name_from_topic(
        &node_name,
        &topic_type,
        &certificate,
    ));
    info!("ROS {} takes gdp name {:?}", node_name, node_gdp_name);

    let topic_gdp_name = GDPName(get_gdp_name_from_topic(
        &topic_name,
        &topic_type,
        &certificate,
    ));
    info!("topic {} takes gdp name {:?}", topic_name, topic_gdp_name);

    let (m_tx, _m_rx) = unbounded_channel::<GDPPacket>();
    let ctx = r2r::Context::create().expect("context creation failure");
    let mut node = r2r::Node::create(ctx, &node_name, "namespace").expect("node creation failure");
    let mut subscriber = node
        .subscribe_untyped(&topic_name, &topic_type, QosProfile::default())
        .expect("topic subscribing failure");

    let _handle = tokio::task::spawn_blocking(move || loop {
        node.spin_once(std::time::Duration::from_millis(100));
    });

    // note that different from other connectionfibs, we send advertisement ahead of time
    let node_advertisement = construct_gdp_advertisement_from_structs(
        topic_gdp_name,
        node_gdp_name,
        crate::structs::GDPNameRecord {
            record_type: crate::structs::GDPNameRecordType::UPDATE,
            gdpname: node_gdp_name,
            source_gdpname: node_gdp_name,
            webrtc_offer: None,
            ip_address: None,
            indirect: None,
            ros: Some((topic_name.clone(), topic_type.clone())),
        },
    );
    proc_gdp_packet(
        node_advertisement, // packet
        &fib_tx,            // used to send packet to fib
        &channel_tx,        // used to send GDPChannel to fib
        &m_tx,              // the sending handle of this connection
        &rib_query_tx,      // used to send GDPNameRecord to rib
        format!("ros subscriber {}-{}", topic_name, topic_type),
    )
    .await;

    loop {
        tokio::select! {
            Some(packet) = subscriber.next() => {
                info!("received a packet {:?}", packet);
                let ros_msg = serde_json::to_vec(&packet.unwrap()).unwrap();

                let packet = construct_gdp_forward_from_bytes(topic_gdp_name, node_gdp_name, ros_msg );
                proc_gdp_packet(packet,  // packet
                    &fib_tx,  //used to send packet to fib
                    &channel_tx, // used to send GDPChannel to fib
                    &m_tx, //the sending handle of this connection
                    &rib_query_tx, // used to send GDPNameRecord to rib
                    "".to_string(),
                ).await;

            }
        }
    }
}

#[cfg(feature = "ros")]
pub async fn ros_subscriber_image(
    fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>, node_name: String,
    topic_name: String, certificate: Vec<u8>, rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let topic_type = "sensor_msgs/CompressedImage".to_string();
    let node_gdp_name = GDPName(get_gdp_name_from_topic(
        &node_name,
        &topic_type,
        &certificate,
    ));
    info!("ROS {} takes gdp name {:?}", node_name, node_gdp_name);

    let topic_gdp_name = GDPName(get_gdp_name_from_topic(
        &topic_name,
        &topic_type,
        &certificate,
    ));
    info!("topic {} takes gdp name {:?}", topic_name, topic_gdp_name);

    let (m_tx, _m_rx) = unbounded_channel::<GDPPacket>();
    let ctx = r2r::Context::create().expect("context creation failure");
    let mut node = r2r::Node::create(ctx, &node_name, "namespace").expect("node creation failure");
    let mut subscriber = node
        .subscribe::<r2r::sensor_msgs::msg::CompressedImage>(&topic_name, QosProfile::default())
        .expect("topic subscribing failure");

    let _handle = tokio::task::spawn_blocking(move || loop {
        node.spin_once(std::time::Duration::from_millis(100));
    });

    // note that different from other connectionfibs, we send advertisement ahead of time
    let node_advertisement = construct_gdp_advertisement_from_structs(
        topic_gdp_name,
        node_gdp_name,
        crate::structs::GDPNameRecord {
            record_type: crate::structs::GDPNameRecordType::UPDATE,
            gdpname: node_gdp_name,
            source_gdpname: node_gdp_name,
            webrtc_offer: None,
            ip_address: None,
            indirect: None,
            ros: Some((topic_name.clone(), topic_type.clone())),
        },
    );
    proc_gdp_packet(
        node_advertisement, // packet
        &fib_tx,            // used to send packet to fib
        &channel_tx,        // used to send GDPChannel to fib
        &m_tx,              // the sending handle of this connection
        &rib_query_tx,
        format!("ros subscriber image {}-{}", topic_name, topic_type),
    )
    .await;

    loop {
        tokio::select! {
            Some(packet) = subscriber.next() => {

                let ros_msg = packet.data;
                info!("received a ROS packet");

                let packet = construct_gdp_forward_from_bytes(topic_gdp_name, node_gdp_name, ros_msg );
                proc_gdp_packet(packet,  // packet
                    &fib_tx,  //used to send packet to fib
                    &channel_tx, // used to send GDPChannel to fib
                    &m_tx, //the sending handle of this connection
                    &rib_query_tx,
                    format!("ros subscriber image {}-{}", topic_name, topic_type),
                ).await;

            }
        }
    }
}

#[cfg(feature = "ros")]
pub async fn ros_publisher_image(
    fib_tx: UnboundedSender<GDPPacket>, channel_tx: UnboundedSender<GDPChannel>, node_name: String,
    topic_name: String, certificate: Vec<u8>, rib_query_tx: UnboundedSender<GDPNameRecord>,
) {
    let topic_type = "sensor_msgs/CompressedImage".to_string();
    let node_gdp_name = GDPName(get_gdp_name_from_topic(
        &node_name,
        &topic_type,
        &certificate,
    ));
    info!("ROS {} takes gdp name {:?}", node_name, node_gdp_name);

    let topic_gdp_name = GDPName(get_gdp_name_from_topic(
        &topic_name,
        &topic_type,
        &certificate,
    ));
    info!("topic {} takes gdp name {:?}", topic_name, topic_gdp_name);

    let (m_tx, mut m_rx) = unbounded_channel::<GDPPacket>();

    let ctx = r2r::Context::create().expect("context creation failure");
    let mut node = r2r::Node::create(ctx, &node_name, "namespace").expect("node creation failure");
    let publisher = node
        .create_publisher::<r2r::sensor_msgs::msg::CompressedImage>(
            &topic_name,
            QosProfile::default(),
        )
        .expect("publisher creation failure");

    let _handle = tokio::task::spawn_blocking(move || loop {
        node.spin_once(std::time::Duration::from_millis(100));
    });

    // note that different from other connection fibs, we send advertisement ahead of time
    let node_advertisement = construct_gdp_advertisement_from_structs(
        topic_gdp_name,
        node_gdp_name,
        crate::structs::GDPNameRecord {
            record_type: crate::structs::GDPNameRecordType::UPDATE,
            gdpname: node_gdp_name,
            source_gdpname: node_gdp_name,
            webrtc_offer: None,
            ip_address: None,
            indirect: None,
            ros: Some((topic_name.clone(), topic_type.clone())),
        },
    );
    proc_gdp_packet(
        node_advertisement, // packet
        &fib_tx,            // used to send packet to fib
        &channel_tx,        // used to send GDPChannel to fib
        &m_tx,              // the sending handle of this connection
        &rib_query_tx,
        format!("ros publisher image {}-{}", topic_name, topic_type),
    )
    .await;

    loop {
        tokio::select! {
            Some(pkt_to_forward) = m_rx.recv() => {
                if pkt_to_forward.action == GdpAction::Forward {
                    info!("new payload to publish locally");
                    if pkt_to_forward.gdpname == topic_gdp_name {
                        let payload = pkt_to_forward.get_byte_payload().unwrap();
                        let ros_msg:CompressedImage = CompressedImage{
                            header: Header{
                                stamp: r2r::builtin_interfaces::msg::Time{sec: 0, nanosec: 0},
                                frame_id: "0".to_owned()
                            },
                            format: "png".to_owned(),
                            data: payload.to_vec(),
                        };//serde_json::from_str(str::from_utf8(payload).unwrap()).expect("json parsing failure");
                        // info!("the decoded payload to publish is {:?}", ros_msg);
                        publisher.publish(&ros_msg).unwrap();
                    } else{
                        info!("{:?} received a packet for name {:?}",pkt_to_forward.gdpname, topic_gdp_name);
                    }
                } else {
                    info!("Received a packet of type {:?}, dont do anything", pkt_to_forward.action)
                }
            },
        }
    }
}
