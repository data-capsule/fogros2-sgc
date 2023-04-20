# Guide for Writing Configuration File

The following is a talker example of the configuration file. The listener example simply flips the `local` to `sub` to subscribe to the local chatter topic and to publish to the remote topic.
```toml
# FogROS2-SGC configuration file
# debug = true | false 
# setting up debug to true will print out debug messages
debug = true
# log_level = "debug" | "info" | "warn" | "error" | "off"
log_level = "info"
# crypto_name = "test_cert" | ..." in /src/scripts/crypto
crypto_name="test_cert"
# address of the signaling server of webrtc 
# the default parameter is provided by a berkeley server
signaling_server_address = "ws://128.32.37.42:8000"
# automatic_topic_discovery = true | false
# if true, FogROS2-SGC will try to discover all topics
# if false, FogROS2-SGC will only discover topics defined in [[ros]] section
automatic_topic_discovery = false
# There can be multiple [[ros]] sections
# each section defines a ROS node name and its topic
[[ros]]
# action = "sub" | "pub | "noop"
# pub = subscribe to local topic and publish to remote topic
# sub = publish to local topic and subscribe to remote topic
# noop = do nothing, used to exclude certain topics in auto mode
action = "pub"
# topic_name of fogros2-sgc node that will publish/subscribe to
topic_name = "/chatter"
# topic type of fogros2-sgc node that will publish/subscribe to
topic_type = "std_msgs/msg/String"
```

### (Unstable) Automatic Topic Discovery
Use `automatic.toml` for automatic topic discovery. 

We note that this is not part of the FogROS2 SGC design, because this potentially 
exposes all the topics to other authorized machines and may create a lot of unnecessary traffic. FogROS2 SGC is designed to expose a limited number of public 
interfaces and keep the private topic to the robots. 
