use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::sleep;
use tokio::time::Duration;
use futures::future;
use futures::executor::LocalPool;
use r2r::QosProfile;
use tokio::process::Command;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, UnboundedSender};
use utils::app_config::AppConfig;
use crate::network::dtls::{dtls_listener, dtls_test_client, dtls_to_peer, dtls_to_peer_direct};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher, ros_subscriber};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher_image, ros_subscriber_image};
use crate::network::tcp::tcp_to_peer_direct;
use crate::structs::{GDPPacket, GDPChannel};
use futures::stream::{self, StreamExt};
// use r2r::Node::get_topic_names_and_types;

/// function that creates a new thread for each topic 
/// and spawns a new thread that peers with the gateway
pub async fn topic_creator(
    peer_with_gateway: bool, 
    default_gateway_addr: String, 
    node_name: String,
    protocol : String,
    topic_name: String,
    topic_type: String,
    action: String, 
    rib_tx: UnboundedSender<GDPPacket>, 
    channel_tx: UnboundedSender<GDPChannel>,
){
    if action == "noop" {
        info!("noop for topic {}", topic_name);
        return;
    }
    info!("topic creator for topic {}, type {}, action {}", topic_name, topic_type, action);

    // This sender handle is a specific connection for ROS
    // this is used to diffentiate different channels in ROS topics
    let (mut m_tx, m_rx) = mpsc::unbounded_channel();
    if  peer_with_gateway {
        if protocol == "dtls" {
            let _ros_peer = tokio::spawn(dtls_to_peer_direct(
                default_gateway_addr.clone().into(),
                rib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
            ));
        } else if protocol == "tcp" {
            let _ros_peer = tokio::spawn(tcp_to_peer_direct(
                default_gateway_addr.clone().into(),
                rib_tx.clone(),
                channel_tx.clone(),
                m_tx.clone(),
                m_rx,
            ));
        }
    } else {
        // reasoning here:
        // m_tx is the next hop that the ros sends messages
        // if we don't peer with another router directly
        // we just forward to rib
        m_tx = rib_tx.clone();
    }

    let _ros_handle = match action.as_str() {
        "sub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_subscriber_image(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
            )),
            _ => tokio::spawn(ros_subscriber(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                topic_type,
            )),
        },
        "pub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_publisher_image(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
            )),
            _ => tokio::spawn(ros_publisher(
                m_tx.clone(),
                channel_tx.clone(),
                node_name,
                topic_name,
                topic_type,
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
        .output().await.unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();
    info!("topic info of topic {}: {}", topic_name, output_str);
    if output_str.contains("Publisher count: 0") {
        info!("topic {} has no local publisher, mark as remote topic publisher", topic_name);
        return "pub".to_string();
    } else if output_str.contains("Subscription count: 0") {
        info!("topic {} has no local subscriber, mark as remote topic subscriber", topic_name);
        return "sub".to_string();
    }else {
        info!("topic {} has local publishers and subscribers, mark as noop", topic_name);
        return "noop".to_string();
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RosTopicStatus {
    pub action: String,
}

pub async fn ros_topic_manager(
    peer_with_gateway:bool, 
    default_gateway_addr:String, 
    rib_tx: UnboundedSender<GDPPacket>, 
    channel_tx: UnboundedSender<GDPChannel>,
){
    // get ros information from config file
    let config = AppConfig::fetch().expect("Failed to fetch config");
    // bookkeeping the status of ros topics
    let mut topic_status = HashMap::new();


    for topic in config.ros {
        let node_name = topic.node_name;
        let protocol = topic.protocol;
        let topic_name = topic.topic_name;
        let topic_type = topic.topic_type;
        let action = topic.local;
        topic_creator(
            peer_with_gateway, 
            default_gateway_addr.clone(), 
            node_name,
            protocol,
            topic_name.clone(),
            topic_type,
            action.clone(), 
            rib_tx.clone(), 
            channel_tx.clone(),
        ).await;

        topic_status.insert(topic_name, RosTopicStatus{action:action.clone()});
    }

    // if automatic topic discovery is disabled, return
    if ! config.automatic_topic_discovery {
        info!("automatic topic discovery is disabled, return");
        return;
    } else {
        info!("automatic topic discovery is enabled. May be unstable!");
    }

    let ctx = r2r::Context::create().expect("failed to create context");
    let node = r2r::Node::create(ctx, "ros_manager", "namespace").expect("failed to create node");
    // when a new topic is detected, create a new thread 
    // to handle the topic
    loop {
        let current_topics = node.get_topic_names_and_types().unwrap();
        let mut existing_topics = vec![];
        // check if there is a new topic by comparing current topics with 
        // the bookkeeping topics
        for topic in current_topics {
            if !topic_status.contains_key(&topic.0) {
                let topic_name = topic.0.clone();
                let action = determine_topic_action(topic_name.clone()).await;
                info!("detect a new topic {:?}", topic);
                topic_creator(
                    peer_with_gateway, 
                    default_gateway_addr.clone(), 
                    // TODO: currently we use a fixed node name with a random integer
                    format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                    config.ros_protocol.clone(),
                    topic_name.clone(),
                    topic.1[0].clone(),
                    action.clone(), 
                    rib_tx.clone(), 
                    channel_tx.clone(),
                ).await;
                topic_status.insert(topic_name.clone(), 
                        RosTopicStatus{action:action});
            } else {
                existing_topics.push(topic.0.clone());   
            }
        }
        info!("automatic new topic discovery: topics already exist {:?}", existing_topics);
        sleep(Duration::from_millis(5000)).await;
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

