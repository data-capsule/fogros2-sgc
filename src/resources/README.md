# Guide for Writing Configuration File

The following is a talker example of the configuration file. The listener example simply flips the `local` to `sub` to subscribe to the local chatter topic and to publish to the remote topic.
```toml
# FogROS2-SGC configuration file
# debug = true | false 
# setting up debug to true will print out debug messages
debug = true
# log_level = "debug" | "info" | "warn" | "error" | "off"
log_level = "info"
# port for various protocols
# make sure to adapt your IP tables accordingly
tcp_port = "9997"
dtls_port = "9232"
grpc_port = "50051"
# ros_protocol = "tcp" | "dtls" | "grpc"
# this is overriden by the [[ros]] section
# this is used when no [[ros]] section is defined (auto mode)
# or routing infomation advertisement
ros_protocol="dtls"
# crypto_name = "test_cert" | ..." in /src/scripts/crypto
crypto_name="test_cert"
# peer_with_gateway = true | false
# if true, FogROS2-SGC will try to connect to the default_gateway
# if false, FogROS2-SGC will not try to connect to the gateway
# this option is overriden by GATEWAY_IP environment variable
peer_with_gateway = false
# default_gateway ip address
default_gateway = "10.5.0.6"
# There can be multiple [[ros]] sections
# each section defines a ROS node name and its topic
[[ros]]
# action = "sub" | "pub | "noop"
# pub = subscribe to local topic and publish to remote topic
# sub = publish to local topic and subscribe to remote topic
# noop = do nothing, used to exclude certain topics in auto mode
action = "sub"
# node_name of fogros2-sgc node that will be created
# doesn't quite matter what you put here as long as no name collision
node_name = "talker"
# this protocol will be used to communicate with the gateway
# this overrides 
protocol="dtls"
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
