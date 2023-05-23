#[cfg(feature = "ros")]
use crate::network::ros::{ros_publisher, ros_subscriber};
#[cfg(feature = "ros")]
use crate::network::webrtc::{self, register_webrtc_stream, webrtc_reader_and_writer};

use crate::structs::{
    gdp_name_to_string, generate_random_gdp_name, get_gdp_name_from_topic, GDPName,
};

use async_datachannel::DataStream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::select;
use tracing::subscriber;

use crate::db::*;
use futures::{future, StreamExt};
use redis::{self, transaction, Client, Commands, PubSubCommands, RedisResult};
use redis_async::{client, resp::FromResp};
use tokio::process::Command;
use tokio::sync::mpsc::{self};
use tokio::time::Duration;
use tokio::time::{sleep, timeout};
use utils::app_config::AppConfig;

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

pub async fn ros_topic_creator(
    stream: async_datachannel::DataStream, node_name: String, topic_name: String,
    topic_type: String, action: String, certificate: Vec<u8>,
) {
    info!(
        "topic creator for topic {}, type {}, action {}",
        topic_name, topic_type, action
    );
    let (ros_tx, ros_rx) = mpsc::unbounded_channel();
    let (rtc_tx, rtc_rx) = mpsc::unbounded_channel();
    tokio::spawn(webrtc_reader_and_writer(stream, ros_tx.clone(), rtc_rx));

    let _ros_handle = match action.as_str() {
        "sub" => match topic_type.as_str() {
            _ => tokio::spawn(ros_subscriber(
                node_name,
                topic_name,
                topic_type,
                certificate,
                rtc_tx, // m_tx is the sender to the webrtc reader
            )),
        },
        "pub" => match topic_type.as_str() {
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
    topic_gdp_name: GDPName, topic_name: String, topic_type: String, certificate: Vec<u8>,
) {
    let redis_url = get_redis_url();
    allow_keyspace_notification(&redis_url);
    let publisher_listening_gdp_name = generate_random_gdp_name();

    // currently open another synchronous connection for put and get
    let publisher_topic = format!("{}-pub", gdp_name_to_string(topic_gdp_name));
    let subscriber_topic = format!("{}-sub", gdp_name_to_string(topic_gdp_name));

    let redis_addr_and_port = get_redis_address_and_port();
    let pubsub_con = client::pubsub_connect(redis_addr_and_port.0, redis_addr_and_port.1)
        .await
        .expect("Cannot connect to Redis");
    let topic = format!("__keyspace@0__:{}", subscriber_topic);
    let mut msgs = pubsub_con
        .psubscribe(&topic)
        .await
        .expect("Cannot subscribe to topic");

    let subscribers = get_entity_from_database(&redis_url, &subscriber_topic)
        .expect("Cannot get subscriber from database");
    info!("subscriber list {:?}", subscribers);

    let tasks = subscribers.clone().into_iter().map(|subscriber| {
        let topic_name_clone = topic_name.clone();
        let topic_type_clone = topic_type.clone();
        let certificate_clone = certificate.clone();
        let publisher_topic = publisher_topic.clone();
        let redis_url = redis_url.clone();
        let publisher_listening_gdp_name_clone = publisher_listening_gdp_name.clone();

        tokio::spawn(async move {
            info!("subscriber {}", subscriber);
            let publisher_url = format!(
                "{},{},{}",
                gdp_name_to_string(topic_gdp_name),
                gdp_name_to_string(publisher_listening_gdp_name_clone),
                subscriber
            );
            info!("publisher listening for signaling url {}", publisher_url);

            add_entity_to_database_as_transaction(&redis_url, &publisher_topic, &publisher_url)
                .expect("Cannot add publisher to database");
            info!(
                "publisher {} added to database of channel {}",
                &publisher_url, publisher_topic
            );

            let webrtc_stream = register_webrtc_stream(&publisher_url, None).await;
            info!("publisher registered webrtc stream");
            let _ros_handle = ros_topic_creator(
                webrtc_stream,
                format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                topic_name_clone,
                topic_type_clone,
                "sub".to_string(),
                certificate_clone,
            )
            .await;
        })
    });
    let mut tasks = tasks.collect::<Vec<_>>();

    let message_handling_task_handle = tokio::spawn(async move {
        loop {
            let message = msgs.next().await;
            match message {
                Some(message) => {
                    let received_operation = String::from_resp(message.unwrap()).unwrap();
                    info!("KVS {}", received_operation);
                    if (received_operation != "lpush") {
                        info!("the operation is not lpush, ignore");
                        continue;
                    }
                    let updated_subscribers =
                        get_entity_from_database(&redis_url, &subscriber_topic)
                            .expect("Cannot get subscriber from database");
                    info!(
                        "get a list of subscribers from KVS {:?}",
                        updated_subscribers
                    );
                    let subscriber = updated_subscribers.first().unwrap(); //first or last?
                    if subscribers.clone().contains(subscriber) {
                        warn!("subscriber {} already in the list, ignore", subscriber);
                        continue;
                    }
                    let publisher_url = format!(
                        "{},{},{}",
                        gdp_name_to_string(topic_gdp_name),
                        gdp_name_to_string(publisher_listening_gdp_name),
                        subscriber
                    );
                    info!("publisher listening for signaling url {}", publisher_url);

                    add_entity_to_database_as_transaction(
                        &redis_url,
                        &publisher_topic,
                        &publisher_url,
                    )
                    .expect("Cannot add publisher to database");
                    info!(
                        "publisher {} added to database of channel {}",
                        &publisher_url, publisher_topic
                    );

                    let webrtc_stream = register_webrtc_stream(&publisher_url, None).await;
                    info!("publisher registered webrtc stream");
                    let _ros_handle = ros_topic_creator(
                        webrtc_stream,
                        format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                        topic_name.clone(),
                        topic_type.clone(),
                        "sub".to_string(),
                        certificate.clone(),
                    )
                    .await;
                }
                None => {
                    info!("message is none");
                }
            }
        }
    });
    tasks.push(message_handling_task_handle);

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;
    info!("all the subscribers are checked!");
}

async fn create_new_remote_subscriber(
    topic_gdp_name: GDPName, topic_name: String, topic_type: String, certificate: Vec<u8>,
) {
    let subscriber_listening_gdp_name = generate_random_gdp_name();
    let redis_url = get_redis_url();
    allow_keyspace_notification(&redis_url);
    // currently open another synchronous connection for put and get
    let publisher_topic = format!("{}-pub", gdp_name_to_string(topic_gdp_name));
    let subscriber_topic = format!("{}-sub", gdp_name_to_string(topic_gdp_name));

    let redis_addr_and_port = get_redis_address_and_port();
    let pubsub_con = client::pubsub_connect(redis_addr_and_port.0, redis_addr_and_port.1)
        .await
        .expect("Cannot connect to Redis");
    let topic = format!("__keyspace@0__:{}", publisher_topic);
    let mut msgs = pubsub_con
        .psubscribe(&topic)
        .await
        .expect("Cannot subscribe to topic");

    add_entity_to_database_as_transaction(
        &redis_url,
        &subscriber_topic,
        gdp_name_to_string(subscriber_listening_gdp_name).as_str(),
    )
    .expect("Cannot add publisher to database");
    info!("subscriber added to database");

    let publishers = get_entity_from_database(&redis_url, &publisher_topic)
        .expect("Cannot get subscriber from database");
    info!("publisher list {:?}", publishers);

    let tasks = publishers.clone().into_iter().map(|publisher| {
        let topic_name_clone = topic_name.clone();
        let topic_type_clone = topic_type.clone();
        let certificate_clone = certificate.clone();
        let subscriber_listening_gdp_name_clone = subscriber_listening_gdp_name.clone();
        if !publisher.ends_with(&gdp_name_to_string(subscriber_listening_gdp_name_clone)) {
            info!(
                "find publisher mailbox {} doesn not end with subscriber {}",
                publisher,
                gdp_name_to_string(subscriber_listening_gdp_name_clone)
            );
            let handle = tokio::spawn(async move {});
            return handle;
        } else {
            info!(
                "find publisher mailbox {} ends with subscriber {}",
                publisher,
                gdp_name_to_string(subscriber_listening_gdp_name_clone)
            );
        }
        let publisher = publisher
            .split(',')
            .skip(4)
            .take(4)
            .collect::<Vec<&str>>()
            .join(",");

        tokio::spawn(async move {
            info!("publisher {}", publisher);
            // subscriber's address
            let my_signaling_url = format!(
                "{},{},{}",
                gdp_name_to_string(topic_gdp_name),
                gdp_name_to_string(subscriber_listening_gdp_name_clone),
                publisher
            );
            // publisher's address
            let peer_dialing_url = format!(
                "{},{},{}",
                gdp_name_to_string(topic_gdp_name),
                publisher,
                gdp_name_to_string(subscriber_listening_gdp_name)
            );
            // let subsc = format!("{}/{}", gdp_name_to_string(publisher_listening_gdp_name), subscriber);
            info!(
                "subscriber uses signaling url {} that peers to {}",
                my_signaling_url, peer_dialing_url
            );
            // workaround to prevent subscriber from dialing before publisher is listening
            tokio::time::sleep(Duration::from_millis(1000)).await;
            info!("subscriber starts to register webrtc stream");
            let webrtc_stream =
                register_webrtc_stream(&my_signaling_url, Some(peer_dialing_url)).await;
            info!("subscriber registered webrtc stream");
            let _ros_handle = ros_topic_creator(
                webrtc_stream,
                format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                topic_name_clone,
                topic_type_clone,
                "pub".to_string(),
                certificate_clone,
            )
            .await;
        })
    });

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;
    info!("all the mailboxes are checked!");

    loop {
        tokio::select! {
            Some(message) = msgs.next() => {
                match message {
                    Ok(message) => {
                        let received_operation = String::from_resp(message).unwrap();
                        info!("KVS {}", received_operation);
                        if (received_operation != "lpush") {
                            info!("the operation is not lpush, ignore");
                            continue;
                        }

                        let updated_publishers = get_entity_from_database(&redis_url, &publisher_topic).expect("Cannot get publisher from database");
                        info!("get a list of publishers from KVS {:?}", updated_publishers);
                        let publisher = updated_publishers.first().unwrap(); //first or last?

                        if publishers.contains(publisher) {
                            warn!("publisher {} already exists", publisher);
                            continue;
                        }

                        if !publisher.ends_with(&gdp_name_to_string(subscriber_listening_gdp_name)) {
                            warn!("find publisher mailbox {} doesn not end with subscriber {}", publisher, gdp_name_to_string(subscriber_listening_gdp_name));
                            continue;
                        }
                        let publisher = publisher.split(',').skip(4).take(4).collect::<Vec<&str>>().join(",");

                        // subscriber's address
                        let my_signaling_url = format!("{},{},{}", gdp_name_to_string(topic_gdp_name),gdp_name_to_string(subscriber_listening_gdp_name), publisher);
                        // publisher's address
                        let peer_dialing_url = format!("{},{},{}", gdp_name_to_string(topic_gdp_name),publisher, gdp_name_to_string(subscriber_listening_gdp_name));
                        // let subsc = format!("{}/{}", gdp_name_to_string(publisher_listening_gdp_name), subscriber);
                        info!("subscriber uses signaling url {} that peers to {}", my_signaling_url, peer_dialing_url);
                        tokio::time::sleep(Duration::from_millis(1000)).await;
                        info!("subscriber starts to register webrtc stream");
                        // workaround to prevent subscriber from dialing before publisher is listening
                        let webrtc_stream = register_webrtc_stream(&my_signaling_url, Some(peer_dialing_url)).await;

                        info!("subscriber registered webrtc stream");

                        let _ros_handle = ros_topic_creator(
                            webrtc_stream,
                            format!("{}_{}", "ros_manager_node", rand::random::<u32>()),
                            topic_name.clone(),
                            topic_type.clone(),
                            "pub".to_string(),
                            certificate.clone(),
                        )
                        .await;


                    },
                    Err(e) => {
                        eprintln!("ERROR: {}", e);
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RosTopicStatus {
    pub action: String,
}

pub async fn ros_topic_manager() {
    let mut waiting_rib_handles = vec![];
    // get ros information from config file
    let config = AppConfig::fetch().expect("Failed to fetch config");
    // bookkeeping the status of ros topics
    let mut topic_status = HashMap::new();
    let _ros_topic_manager_gdp_name = generate_random_gdp_name();

    // read certificate from file in config
    for topic in config.ros {
        let topic_name = format!("{}", topic.topic_name);
        let topic_type = topic.topic_type;
        let action = topic.action;
        let certificate = std::fs::read(format!(
            "./scripts/crypto/{}/{}-private.pem",
            config.crypto_name, config.crypto_name
        ))
        .expect("crypto file not found!");
        let topic_gdp_name = GDPName(get_gdp_name_from_topic(
            &topic_name.clone(),
            &topic_type,
            &certificate,
        ));

        // clear_topic_key(&gdp_name_to_string(topic_gdp_name));

        match action.as_str() {
            "sub" => {
                let handle = tokio::spawn(async move {
                    create_new_remote_publisher(
                        topic_gdp_name,
                        topic_name.clone(),
                        topic_type,
                        certificate,
                    )
                    .await;
                    info!("exited");
                });

                waiting_rib_handles.push(handle);
            }
            "pub" => {
                let handle = tokio::spawn(async move {
                    create_new_remote_subscriber(
                        topic_gdp_name,
                        topic_name.clone(),
                        topic_type,
                        certificate,
                    )
                    .await;
                    info!("exited");
                });

                waiting_rib_handles.push(handle);
            }
            _ => {
                info!("topic {} has no action", topic_name);
            }
        }
        let topic_name = format!("{}", topic.topic_name);
        topic_status.insert(
            topic_name,
            RosTopicStatus {
                action: action.clone(),
            },
        );
    }

    let certificate = std::fs::read(format!(
        "./scripts/crypto/{}/{}-private.pem",
        config.crypto_name, config.crypto_name
    ))
    .expect("crypto file not found!");
    let ctx = r2r::Context::create().expect("failed to create context");
    let node = r2r::Node::create(ctx, "ros_manager", "namespace").expect("failed to create node");
    // when a new topic is detected, create a new thread
    // to handle the topic
    loop {
        select! {
            _ = sleep(Duration::from_millis(5000)) => {

                if !config.automatic_topic_discovery {
                    info!("automatic topic discovery is disabled");
                    continue;
                } else {
                    info!("automatic topic discovery is enabled. May be unstable!");
                }

                let current_topics = node.get_topic_names_and_types().unwrap();

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
                        info!("detected a new topic {:?} with action {:?}, topic gdpname {:?}", topic, action, topic_gdp_name);
                        topic_status.insert(topic_name.clone(), RosTopicStatus { action: action.clone() });

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
                                // subscribe to a pattern that matches the key you're interested in
                                // create a new thread to handle that listens for the topic
                                let topic_name = topic_name.clone();
                                let certificate = certificate.clone();
                                let topic_type = topic_type.clone();
                                let handle = tokio::spawn(
                                    async move {
                                        create_new_remote_subscriber(topic_gdp_name,
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

                    } else {
                        info!(
                            "automatic new topic {} discovery: topics already exist {:?}",
                            topic.0, topic_status
                        );
                    }
                }

            }
        }
    }
}
