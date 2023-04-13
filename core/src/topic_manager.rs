use crate::network::dtls::dtls_to_peer_direct;
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher, ros_subscriber};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher_image, ros_subscriber_image};
use crate::network::tcp::tcp_to_peer_direct;
use crate::structs::{GDPChannel, GDPPacket, GDPNameRecord, GdpAction};

use serde::{Deserialize, Serialize};
use tokio::select;
use std::collections::HashMap;

use tokio::process::Command;
use tokio::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use tokio::time::sleep;
use tokio::time::Duration;
use utils::app_config::AppConfig;
// use r2r::Node::get_topic_names_and_types;

/// function that creates a new thread for each topic
/// and spawns a new thread that peers with the gateway
pub async fn topic_creator(
    peer_with_gateway: bool, default_gateway_addr: String, node_name: String, protocol: String,
    topic_name: String, topic_type: String, action: String, fib_tx: UnboundedSender<GDPPacket>,
    channel_tx: UnboundedSender<GDPChannel>, certificate: Vec<u8>,
) {
    if action == "noop" {
        info!("noop for topic {}", topic_name);
        return;
    }
    info!(
        "topic creator for topic {}, type {}, action {}",
        topic_name, topic_type, action
    );

    // This sender handle is a specific connection for ROS
    // this is used to differentiate different channels in ROS topics
    let (mut m_tx, m_rx) = mpsc::unbounded_channel();
    if peer_with_gateway {
        if protocol == "dtls" {
            let _ros_peer = tokio::spawn(dtls_to_peer_direct(
                default_gateway_addr.clone().into(),
                fib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
            ));
        } else if protocol == "tcp" {
            let _ros_peer = tokio::spawn(tcp_to_peer_direct(
                default_gateway_addr.clone().into(),
                fib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
            ));
        }
    } else {
        // reasoning here:
        // m_tx is the next hop that the ros sends messages
        // if we don't peer with another router directly
        // we just forward tofib
        m_tx = fib_tx.clone();
    }

    let _ros_handle = match action.as_str() {
        "sub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_subscriber_image(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                certificate,
            )),
            _ => tokio::spawn(ros_subscriber(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                topic_type,
                certificate,
            )),
        },
        "pub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_publisher_image(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                certificate,
            )),
            _ => tokio::spawn(ros_publisher(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                topic_type,
                certificate,
            )),
        },
        _ => panic!("unknown action"),
    };
}

/// determine the action of a new topic
/// pub/sub/noop
/// Currently it uses cli to get the information
/// TODO: use r2r/rcl to get the information
async fn determine_topic_action(topic_name: String) -> String {
    let output = Command::new("ros2")
        .arg("topic")
        .arg("info")
        .arg(topic_name.as_str())
        .output()
        .await
        .unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();
    info!("topic info of topic {}: {}", topic_name, output_str);
    if output_str.contains("Publisher count: 0") {
        info!(
            "topic {} has no local publisher, mark as remote topic publisher",
            topic_name
        );
        return "pub".to_string();
    } else if output_str.contains("Subscription count: 0") {
        info!(
            "topic {} has no local subscriber, mark as remote topic subscriber",
            topic_name
        );
        return "sub".to_string();
    } else {
        info!(
            "topic {} has local publishers and subscribers, mark as noop",
            topic_name
        );
        return "noop".to_string();
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RosTopicStatus {
    pub action: String,
}

pub async fn ros_topic_manager(
    peer_with_gateway: bool, 
    default_gateway_addr: String, 
    fib_tx: UnboundedSender<GDPPacket>,
    mut ros_topic_manager_rx: UnboundedReceiver<GDPPacket>, // receiver of advertise-response
    channel_tx: UnboundedSender<GDPChannel>,
) {
    // get ros information from config file
    let config = AppConfig::fetch().expect("Failed to fetch config");
    // bookkeeping the status of ros topics
    let mut topic_status = HashMap::new();

    // read certificate from file in config
    let certificate = std::fs::read(format!(
        "./scripts/crypto/{}/{}-private.pem",
        config.crypto_name, config.crypto_name
    ))
    .expect("crypto file not found!");

    for topic in config.ros {
        let node_name = topic.node_name;
        let protocol = topic.protocol;
        let topic_name = topic.topic_name;
        let topic_type = topic.topic_type;
        let action = topic.action;
        let _ros_handle = topic_creator(
            peer_with_gateway,
            default_gateway_addr.clone(),
            node_name,
            protocol,
            topic_name.clone(),
            topic_type,
            action.clone(),
            fib_tx.clone(),
            channel_tx.clone(),
            certificate.clone(),
        )
        .await;

        topic_status.insert(topic_name, RosTopicStatus {
            action: action.clone(),
        });
    }

    // if automatic topic discovery is disabled, return
    if !config.automatic_topic_discovery {
        info!("automatic topic discovery is disabled");
        loop {
            // workaround to prevent the ros topic manager from returning
            // thus cleaning up the stack, etc.
            // TODO: is there any better way to do this?
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    } else {
        info!("automatic topic discovery is enabled. May be unstable!");
    }

    let ctx = r2r::Context::create().expect("failed to create context");
    let node = r2r::Node::create(ctx, "ros_manager", "namespace").expect("failed to create node");
    // when a new topic is detected, create a new thread
    // to handle the topic
    loop {
        select! {
            Some(packet) = ros_topic_manager_rx.recv() => {
                // get the gdpname record component of advertiseresponse
                
                match packet.action {
                    GdpAction::AdvertiseResponse => {
                        match packet.name_record {
                            Some(name_record) => {
                                
                                let topic_name = format!("{}", name_record.ros.clone().unwrap().0);
                                let topic_type = format!("{}", name_record.ros.unwrap().1);
                                info!("received advertise response for topic {}", topic_name);
                                let _ros_handle = topic_creator(
                                    peer_with_gateway,
                                    default_gateway_addr.clone(),
                                    // TODO: currently we use a fixed node name with a random integer
                                    format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                                    config.ros_protocol.clone(),
                                    topic_name,
                                    topic_type,
                                    "pub".to_string(), // only publisher needs advertiseResponse from the subscriber
                                    fib_tx.clone(),
                                    channel_tx.clone(),
                                    certificate.clone(),
                                )
                                .await;
                            }, 
                            None => {
                                warn!("received advertise response without name record");
                            }
                        }
                    }, 
                    _  => {
                        warn!("ros topic manager received a packet with action {:?}", packet.action);
                    }
                }
            }, 
            _ = sleep(Duration::from_millis(5000)) => {
                let current_topics = node.get_topic_names_and_types().unwrap();
                let mut existing_topics = vec![];
                // check if there is a new topic by comparing current topics with
                // the bookkeeping topics
                for topic in current_topics {
                    if !topic_status.contains_key(&topic.0) {
                        let topic_name = topic.0.clone();
                        let action = determine_topic_action(topic_name.clone()).await;
                        info!("detect a new topic {:?}", topic);

                        topic_status.insert(topic_name.clone(), RosTopicStatus { action: action });
                    } else {
                        existing_topics.push(topic.0.clone());
                    }
                }
                info!(
                    "automatic new topic discovery: topics already exist {:?}",
                    existing_topics
                );
            }
        }
    }

    // TODO: a better way to detect new ros topic is needed
    // the following is an intuition that doesn't work
    // we subscribe to /parameter_events, whenever a new node joins
    // it will publish some message to this topic, but it doesn't have
    // the topic information, so we run a topic detection
    // let mut param_subscriber = node.
    // subscribe_untyped("/parameter_events", "rcl_interfaces/msg/ParameterEvent", QosProfile::default())
    // .expect("subscribe failed");
    // loop {
    //     info!("in the loop!");
    //     tokio::select! {
    //         Some(packet) = param_subscriber.next() => {
    //             info!("detect a new node {:?}", packet);
    //
    //         }
    //     }
    // }
}
