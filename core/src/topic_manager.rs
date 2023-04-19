use crate::network::dtls::dtls_to_peer_direct;
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher, ros_subscriber};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher_image, ros_subscriber_image};
use crate::network::tcp::tcp_to_peer_direct;
use crate::network::webrtc::{register_webrtc_stream, webrtc_reader_and_writer};
use crate::pipeline::{construct_gdp_advertisement_from_structs, proc_gdp_packet};
use crate::structs::{
    gdp_name_to_string, generate_random_gdp_name, get_gdp_name_from_topic, GDPChannel, GDPName,
    GDPNameRecord, GDPNameRecordType, GDPPacket, GdpAction,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::select;

use tokio::process::Command;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use tokio::time::Duration;
use utils::app_config::AppConfig;
// use r2r::Node::get_topic_names_and_types;

pub async fn topic_creator_webrtc(
    stream: async_datachannel::DataStream, node_name: String, topic_name: String,
    topic_type: String, action: String, certificate: Vec<u8>,
) {
    info!("topic creator for topic {}, type {}, action {}", topic_name, topic_type, action);
    let (ros_tx, mut ros_rx) = mpsc::unbounded_channel();
    let (rtc_tx, mut rtc_rx) = mpsc::unbounded_channel();
    tokio::spawn(webrtc_reader_and_writer(
        stream,
        ros_tx.clone(),
        rtc_rx,
    ));

    let _ros_handle = match action.as_str() {
        "sub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_subscriber_image(
                node_name,
                topic_name,
                certificate,
                rtc_tx, // m_tx is the sender to the webrtc reader
            )),
            _ => tokio::spawn(ros_subscriber(
                node_name,
                topic_name,
                topic_type,
                certificate,
                rtc_tx, // m_tx is the sender to the webrtc reader
            )),
        },
        "pub" => match topic_type.as_str() {
            "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_publisher_image(
                node_name,
                topic_name,
                certificate,
                ros_rx
        
            )),
            _ => tokio::spawn(ros_publisher(
                node_name,
                topic_name,
                topic_type,
                certificate,
                ros_rx, // m_rx is the receiver from the webrtc writer
            )),
        },
        _ => panic!("unknown action"),
    };
}


async fn create_new_remote_publisher(
    topic_gdp_name: GDPName,
    topic_name: String,
    topic_type: String,
    certificate: Vec<u8>
){
    let webrtc_stream = register_webrtc_stream(
        gdp_name_to_string(topic_gdp_name),
        None
    ).await;
    info!("publisher registered webrtc stream");
    let _ros_handle = topic_creator_webrtc(
        webrtc_stream,
        format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
        topic_name.clone(),
        topic_type,
        "sub".to_string(),
        certificate.clone(),
    )
    .await;
}

async fn create_new_remote_subscriber(
    topic_gdp_name: GDPName,
    subscriber_listening_gdp_name: GDPName,
    topic_name: String,
    topic_type: String,
    certificate: Vec<u8>
){
    let webrtc_stream = register_webrtc_stream(
        gdp_name_to_string(subscriber_listening_gdp_name),
        Some(gdp_name_to_string(topic_gdp_name)),
    ).await;
    info!("subscriber registered webrtc stream");

    let _ros_handle = topic_creator_webrtc(
        webrtc_stream,
        format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
        topic_name.clone(),
        topic_type.clone(),
        "pub".to_string(),
        certificate.clone(),
    )
    .await;
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
) {
    // get ros information from config file
    let config = AppConfig::fetch().expect("Failed to fetch config");
    // bookkeeping the status of ros topics
    let mut topic_status = HashMap::new();
    let _ros_topic_manager_gdp_name = generate_random_gdp_name();

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
        // let _ros_handle = topic_creator(
        //     peer_with_gateway,
        //     default_gateway_addr.clone(),
        //     node_name,
        //     protocol,
        //     topic_name.clone(),
        //     topic_type,
        //     action.clone(),
        //     certificate.clone(),
        // )
        // .await;

        topic_status.insert(
            topic_name,
            RosTopicStatus {
                action: action.clone(),
            },
        );
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
            _ = sleep(Duration::from_millis(5000)) => {
                let current_topics = node.get_topic_names_and_types().unwrap();
                let mut existing_topics = vec![];
                let mut waiting_rib_handles = vec![];
                // check if there is a new topic by comparing current topics with
                // the bookkeeping topics
                for topic in current_topics {
                    if !topic_status.contains_key(&topic.0) {
                        let topic_name = topic.0.clone();
                        let topic_type = topic.1[0].clone(); // TODO: currently, broadcast only the first topic type
                        let action = determine_topic_action(topic_name.clone()).await;

                        let topic_gdp_name = GDPName(get_gdp_name_from_topic(
                            &topic_name,
                            &topic_type,
                            &certificate,
                        ));
                        info!("detected a new topic {:?} with action {:?}", topic, action);

                        match action.as_str() {
                            // locally subscribe, globally publish
                            "sub" => {
                                let topic_name = topic_name.clone();
                                let certificate = certificate.clone();
                                let handle = tokio::spawn(
                                    async move {
                                        create_new_remote_publisher(topic_gdp_name, topic_name, topic_type, certificate).await;
                                    }
                                );
                                waiting_rib_handles.push(handle);
                            }

                            // locally publish, globally subscribe
                            "pub" => {
                                // create a new thread to handle that listens for the topic
                                let topic_name = topic_name.clone();
                                let subscriber_listening_gdp_name = generate_random_gdp_name();
                                let certificate = certificate.clone();
                                let topic_type = topic_type.clone();
                                let handle = tokio::spawn(
                                    async move {
                                        create_new_remote_subscriber(topic_gdp_name, 
                                            subscriber_listening_gdp_name, 
                                            topic_name, 
                                            topic_type, 
                                            certificate).await;
                                    }
                                );
                                waiting_rib_handles.push(handle);
                            }
                            _ => {
                                warn!("unknown action {}", action);
                            }
                        }
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
}
