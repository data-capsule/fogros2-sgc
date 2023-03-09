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
from rclpy.node import Node
from sensor_msgs.msg import CompressedImage
from std_msgs.msg import String

class MinimalSubscriber(Node):
    def __init__(self):
        super().__init__("minimal_subscriber")
        self.host_name = socket.gethostname()
        self.host_ip = socket.gethostbyname(self.host_name)

        self.subscription_1 = self.create_subscription(
            CompressedImage, "image_1", self.listener_callback_1, 10
        )
        self.subscription_2 = self.create_subscription(
            CompressedImage, "image_2", self.listener_callback_2, 10
        )

        self.publisher_1 = self.create_publisher(String, "action_1", 10)
        self.publisher_2 = self.create_publisher(String, "action_2", 10)
        self.i = 0

    def listener_callback_1(self, msg):
        with open(f"/home/gdpmobile8/fog_ws/test_img_1/test{self.i}.png", "wb+") as f:
            f.write(msg.data)
        self.i += 1
        # node.get_logger().warning('Publishing: "%s"' % len(msg.data))
        # publisher.publish(msg)
        self.get_logger().warning(
            'I am %s (%s). I heard: "%s"'
            % ("#1", self.host_ip, len(msg.data))
        )
        ret_msg = String()
        ret_msg.data = "#1 receive message"
        self.publisher_1.publish(ret_msg)  


    def listener_callback_2(self, msg):
        with open(f"/home/gdpmobile8/fog_ws/test_img_2/test{self.i}.png", "wb+") as f:
            f.write(msg.data)
        self.i += 1
        # node.get_logger().warning('Publishing: "%s"' % len(msg.data))
        # publisher.publish(msg)
        self.get_logger().warning(
            'I am %s (%s). I heard: "%s"'
            % ("#2", self.host_ip, len(msg.data))
        )
        ret_msg = String()
        ret_msg.data = "#2 receive message"
        self.publisher_2.publish(ret_msg)  


def main(args=None):
    rclpy.init(args=args)

    minimal_subscriber = MinimalSubscriber()

    rclpy.spin(minimal_subscriber)

    # Destroy the node explicitly
    # (optional - otherwise it will be done automatically
    # when the garbage collector destroys the node object)
    minimal_subscriber.destroy_node()
    rclpy.shutdown()


if __name__ == "__main__":
    main()
