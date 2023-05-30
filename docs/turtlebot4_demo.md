# Turtlebot 4 Teleoperation Demo with SGC 

This document details the setup of FogROS2-SGC that remotely stream the camera from and teleoperates the turtlebot 4.

## Setup 

### Turtlebot 

#### Step 1: Basic Setup
Follow the official [tutorial](https://turtlebot.github.io/turtlebot4-user-manual/setup/basic.html) to setup the turtlebot with appropriate Create 3 and Raspberry Pi environment. Run `ros2 topic list` and you should see the topics from both raspberry pi and create 3. 

#### Step 2: H264 streaming

Install dependencies
```
source /opt/ros/humble/setup.bash 
sudo apt install -y libavdevice-dev libavformat-dev libavcodec-dev libavutil-dev  libswscale-dev libx264-dev ros-${ROS_DISTRO}-camera-calibration-parsers
```

Install `h264_image_transport` ros packages
```
mkdir -p ~/fog_ws/src/
cd ~/fog_ws/src
git clone https://github.com/KeplerC/h264_image_transport.git
colcon build
source install/setup.bash
```

#### Step 3: FogROS2 SGC 

```
source ~/fog_ws/install/setup.bash
cd ~
git clone https://github.com/data-capsule/fogros2-sgc.git
cargo build --release
```

### Client (Edge/Cloud/Laptop)
Assume client has appropriate `ros2` installed.

#### Step 1: Install dependencies
```
sudo apt update
sudo apt install ros-humble-teleop-twist-keyboard  ros-humble-rmw-cyclonedds-cpp
```

#### Step 1: H264 streaming

Install dependencies
```
source /opt/ros/humble/setup.bash 
sudo apt install -y libavdevice-dev libavformat-dev libavcodec-dev libavutil-dev  libswscale-dev libx264-dev ros-${ROS_DISTRO}-camera-calibration-parsers
```

Install `h264_image_transport` ros packages
```
mkdir -p ~/fog_ws/src/
cd ~/fog_ws/src
git clone https://github.com/KeplerC/h264_image_transport.git
colcon build
source install/setup.bash
```

#### Step 2: FogROS2 SGC 

```
source ~/fog_ws/install/setup.bash
cd ~
git clone https://github.com/data-capsule/fogros2-sgc.git
cargo build --release
```

#### (Optional) Step3: Visualize with Foxglove
```
sudo apt install nlohmann-json3-dev libasio-dev libwebsocketpp-dev
sudo apt install ros-$ROS_DISTRO-foxglove-bridge
```

### Certificate Exchange 

On robot: 
```
cd scripts
./generate_crypto.sh
```
and download the `~/fogros2-sgc/scripts/crypto` with the `~/fogros2-sgc/scripts/crypto` on the cloud/edge/laptop. This can be done with `rsync` or `vscode`.

## Run 

### Robot 

#### Terminal 1: Oak-D Publisher
A lot of options and calibrations can be used from [Here](https://turtlebot.github.io/turtlebot4-user-manual/software/sensors.html#oak-d).

```
ros2 launch depthai_examples mobile_publisher.launch.py
```
The only requirement is to match the output raw image topic with the one in the H264 encoder launch file. 

#### Terminal 2: H264 Encoder
```
cd fog_ws
source /opt/ros/humble/setup.bash
source install/setup.bash
ros2 launch h264_image_transport launch_encoder.py
```

#### Terminal 3: Run FogROS2-SGC
```
source ~/fog_ws/install/setup.bash
cd ~/fogros2-sgc/
SGC_CONFIG=turtlebot.toml  cargo run --release router
```


### Client Machine 
Note that the `CYCLONEDDS_URI` prevents the client machine from discovering other topics in the same local network. This is only useful if your robot is in the same local network and simply want to test the SGC. 

#### Terminal 1: H264 Decoder
```
cd ~/fog_ws
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp 
export CYCLONEDDS_URI=~/fogros2-sgc/bench/cyclonedds.ubuntu.$(lsb_release -rs | sed 's/\.//').xml
source install/setup.bash
ros2 launch h264_image_transport launch_decoder.py
```

#### Terminal 2: Controller 

```
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp 
export CYCLONEDDS_URI=~/fogros2-sgc/bench/cyclonedds.ubuntu.$(lsb_release -rs | sed 's/\.//').xml
ros2 run teleop_twist_keyboard teleop_twist_keyboard
```

#### Terminal 3: SGC router
```
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp 
export CYCLONEDDS_URI=~/fogros2-sgc/bench/cyclonedds.ubuntu.$(lsb_release -rs | sed 's/\.//').xml
source ~/fog_ws/install/setup.bash
cd ~/fogros2-sgc/
SGC_CONFIG=turtlebot_client.toml cargo run router
```

#### Terminal 4: Visualization
```
cd ~/fog_ws
export RMW_IMPLEMENTATION=rmw_cyclonedds_cpp 
export CYCLONEDDS_URI=~/fogros2-sgc/bench/cyclonedds.ubuntu.$(lsb_release -rs | sed 's/\.//').xml
source install/setup.bash
ros2 launch foxglove_bridge foxglove_bridge_launch.xml port:=8765
```
and launch the `foxglove-studio` to visualize the image topics. You can also choose other visualization tools such as rviz or opencv. 