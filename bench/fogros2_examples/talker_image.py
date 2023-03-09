# Copyright 2022 The Regents of the University of California (Regents)
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# Copyright ©2022. The Regents of the University of California (Regents).
# All Rights Reserved. Permission to use, copy, modify, and distribute this
# software and its documentation for educational, research, and not-for-profit
# purposes, without fee and without a signed licensing agreement, is hereby
# granted, provided that the above copyright notice, this paragraph and the
# following two paragraphs appear in all copies, modifications, and
# distributions. Contact The Office of Technology Licensing, UC Berkeley, 2150
# Shattuck Avenue, Suite 510, Berkeley, CA 94720-1620, (510) 643-7201,
# otl@berkeley.edu, http://ipira.berkeley.edu/industry-info for commercial
# licensing opportunities. IN NO EVENT SHALL REGENTS BE LIABLE TO ANY PARTY
# FOR DIRECT, INDIRECT, SPECIAL, INCIDENTAL, OR CONSEQUENTIAL DAMAGES,
# INCLUDING LOST PROFITS, ARISING OUT OF THE USE OF THIS SOFTWARE AND ITS
# DOCUMENTATION, EVEN IF REGENTS HAS BEEN ADVISED OF THE POSSIBILITY OF SUCH
# DAMAGE. REGENTS SPECIFICALLY DISCLAIMS ANY WARRANTIES, INCLUDING, BUT NOT
# LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
# PARTICULAR PURPOSE. THE SOFTWARE AND ACCOMPANYING DOCUMENTATION, IF ANY,
# PROVIDED HEREUNDER IS PROVIDED "AS IS". REGENTS HAS NO OBLIGATION TO PROVIDE
# MAINTENANCE, SUPPORT, UPDATES, ENHANCEMENTS, OR MODIFICATIONS.

import socket

import rclpy
from sensor_msgs.msg import CompressedImage
from std_msgs.msg import String


def main(args=None):
    rclpy.init(args=args)

    node = rclpy.create_node("minimal_publisher")
    publisher_1 = node.create_publisher(CompressedImage, "image_1", 10)
    publisher_2 = node.create_publisher(CompressedImage, "image_2", 10)
    
    def listener_callback_1(msg):
        node.get_logger().info(
            'I am action_1. I heard: "%s"'
            % (msg.data)
        )

    def listener_callback_2(msg):
        node.get_logger().info(
            'I am action_2. I heard: "%s"'
            % (msg.data)
        )


    node.subscription = node.create_subscription(
        String, "/action_1", listener_callback_1, 10
    )
    node.subscription = node.create_subscription(
        String, "/action_2", listener_callback_2, 10
    )

    msg_1 = CompressedImage()
    with open("/home/gdpmobile8/fog_ws/test_img_1/test0.png", "rb") as f:
        msg_1.data = f.read()
    msg_1.format = "png"

    msg_2 = CompressedImage()
    with open("/home/gdpmobile8/fog_ws/test_img_2/test0.png", "rb") as f:
        msg_2.data = f.read()
    msg_2.format = "png"

    def timer_callback():
        node.get_logger().warning('topic_1: Publishing: "%s"' % len(msg_1.data))
        publisher_1.publish(msg_1)
        node.get_logger().warning('topic_2: Publishing: "%s"' % len(msg_2.data))
        publisher_2.publish(msg_2)

    timer_period = 1  # seconds
    timer = node.create_timer(timer_period, timer_callback)

    rclpy.spin(node)

    # Destroy the timer attached to the node explicitly
    # (optional - otherwise it will be done automatically
    # when the garbage collector destroys the node object)
    node.destroy_timer(timer)
    node.destroy_node()
    rclpy.shutdown()


if __name__ == "__main__":
    main()
