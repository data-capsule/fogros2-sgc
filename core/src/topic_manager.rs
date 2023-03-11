use futures::future;
use tokio::sync::mpsc::{self, UnboundedSender};
use utils::app_config::AppConfig;
use crate::network::dtls::{dtls_listener, dtls_test_client, dtls_to_peer, dtls_to_peer_direct};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher, ros_subscriber};
#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher_image, ros_subscriber_image};
use crate::network::tcp::tcp_to_peer_direct;
use crate::structs::{GDPPacket, GDPChannel};

/// function that creates a new thread for each topic 
/// and spawns a new thread that peers with the gateway
pub async fn peer_with_new_topic(
    
){

}


pub async fn ros_topic_manager(
    peer_with_gateway: bool, 
    default_gateway_addr: String, 
    rib_tx: UnboundedSender<GDPPacket>, 
    channel_tx: UnboundedSender<GDPChannel>,
){
    let config = AppConfig::fetch().expect("App config unable to load");
    let mut future_handles = Vec::new();

    for ros_config in config.ros {
        // This sender handle is a specific connection for ROS
        // this is used to diffentiate different channels in ROS topics
        let (mut m_tx, m_rx) = mpsc::unbounded_channel();
        if  peer_with_gateway {
            if config.ros_protocol == "dtls" {
                let ros_peer = tokio::spawn(dtls_to_peer_direct(
                    default_gateway_addr.clone().into(),
                    rib_tx.clone(),
                    channel_tx.clone(),
                    m_tx.clone(),
                    m_rx,
                ));
                future_handles.push(ros_peer);
            } else if config.ros_protocol == "tcp" {
                let ros_peer = tokio::spawn(tcp_to_peer_direct(
                    default_gateway_addr.clone().into(),
                    rib_tx.clone(),
                    channel_tx.clone(),
                    m_tx.clone(),
                    m_rx,
                ));
                future_handles.push(ros_peer);
            }
        } else {
            // reasoning here:
            // m_tx is the next hop that the ros sends messages
            // if we don't peer with another router directly
            // we just forward to rib
            m_tx = rib_tx.clone();
        }

        let ros_handle = match ros_config.local.as_str() {
            "pub" => match ros_config.topic_type.as_str() {
                "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_subscriber_image(
                    m_tx.clone(),
                    channel_tx.clone(),
                    ros_config.node_name,
                    ros_config.topic_name,
                )),
                _ => tokio::spawn(ros_subscriber(
                    m_tx.clone(),
                    channel_tx.clone(),
                    ros_config.node_name,
                    ros_config.topic_name,
                    ros_config.topic_type,
                )),
            },
            _ => match ros_config.topic_type.as_str() {
                "sensor_msgs/msg/CompressedImage" => tokio::spawn(ros_publisher_image(
                    m_tx.clone(),
                    channel_tx.clone(),
                    ros_config.node_name,
                    ros_config.topic_name,
                )),
                _ => tokio::spawn(ros_publisher(
                    m_tx.clone(),
                    channel_tx.clone(),
                    ros_config.node_name,
                    ros_config.topic_name,
                    ros_config.topic_type,
                )),
            },
        };
    }

}