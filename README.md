# FogROS G

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**

- [Installation](#installation)
- [Install dependencies](#install-dependencies)
  - [Install Rust](#install-rust)
  - [(Optional) ROS](#optional-ros)
- [Build the repo](#build-the-repo)
- [Run DAgger Demo](#run-dagger-demo)
- [Known issues](#known-issues)
- [Run Demo Image Transport](#run-demo-image-transport)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->


### Installation 

### Install dependencies 
sudo apt update
sudo apt install build-essential curl pkg-config libssl-dev protobuf-compiler clang

#### Install Rust 
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -y
```
and run 
```
source "$HOME/.cargo/env"
```
to configure the environment variables. 

#### (Optional) ROS 
If you do not plan to use ROS on the machine and only use the machine as a router or proxy, you need 
to change the `default = ["ros"]` to `default = []` in `./core/Cargo.toml`. 

Currently we use ROS2 rolling for development. 
```
sudo apt update && sudo apt install curl
sudo curl -sSL https://raw.githubusercontent.com/ros/rosdistro/master/ros.key -o /usr/share/keyrings/ros-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/ros-archive-keyring.gpg] http://packages.ros.org/ros2/ubuntu $(. /etc/os-release && echo $UBUNTU_CODENAME) main" | sudo tee /etc/apt/sources.list.d/ros2.list > /dev/null
sudo apt update
sudo apt install ros-rolling-desktop
```
Then use 
```
source /opt/ros/rolling/setup.bash
```
whenever you launch a new terminal. 


### Build the repo 
```
cargo build
```

### Run DAgger Demo 

First run the cloud side (gateway, known IP address): 
```
GDP_CONFIG=demo_cloud.toml cargo run router
```

Then run robot side (no need to configure IP address): 
```
GDP_CONFIG=demo_robot.toml cargo run router
```
Note that you need to configure `default_gateway` to be the IP address of the cloud machine in `./src/demo_robot.toml`.

### Known issues 
1. segmentation fault / node creation failure: not caused by our project but our underlying framework or ROS rcl itself. Restart the program and the problem should be fixed. The hypothesis is asynchronous error when the nodes are created too fast in parallel. 

### Run Demo Image Transport
In the example, we use two machines A and B to show the image transport capability. 

On machine A (publisher), the configuration file in `./src/default_config.toml` should be like 
```
debug = true
log_level = "info"
channel_size = 100
tcp_port = "9997"
dtls_port = "9232"
grpc_port = "50051"
peer_with_gateway = true
default_gateway = "128.32.37.48:9997"
[[ros]]
local = "pub"
node_name = "GDP_Router"
topic_name = "/chatter"
topic_type = "std_msgs/msg/String"
[[ros]]
local = "pub"
node_name = "GDP_Router_for_image"
topic_name = "/Image"
topic_type = "sensor_msgs/msg/CompressedImage"
[database]
url = "custom database url"
```
Note that you need to configure `default_gateway` to be the IP address of the second machine. 

On machine B (subscriber), the configuration file in `./src/default_config.toml` should be like 
```
debug = true
log_level = "info"
channel_size = 100
tcp_port = "9997"
dtls_port = "9232"
grpc_port = "50051"
peer_with_gateway = false
default_gateway = "128.32.37.48:9997"
[[ros]]
local = "sub"
node_name = "GDP_Router"
topic_name = "/chatter"
topic_type = "std_msgs/msg/String"
[[ros]]
local = "sub"
node_name = "GDP_Router_for_image"
topic_name = "/Image"
topic_type = "sensor_msgs/msg/CompressedImage"
[database]
url = "custom database url"
```
Note that the `local` in both topics are changed to `sub` instead of `pub` to reflect the fact that we are 
acting as a subscriber. We also need to change `peer_with_gateway` because we do not want this node to actively
peer with other nodes. 

On machine A, run 
```
ros2 run demo_nodes_cpp talker
```

On machine B, run 
```
ros2 run demo_nodes_cpp listener
```
should broadcast the chatter topic. 

It should also broadcast the `/Image` topic at the same time. 


