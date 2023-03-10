import rclpy
from rclpy.node import Node
from std_msgs.msg import String
from sensor_msgs.msg import CompressedImage
from time import time 
from time import sleep

def payload_generator(id, size = 1000):
    return b"1" * size

def payload_to_latency(payload):
    ts = float(payload.split(",")[1])
    return time() - ts

class MinimalPublisher(Node):

    def __init__(self):
        super().__init__('minimal_publisher')
        self.publisher_ = self.create_publisher(CompressedImage, 'benchmark', 10)
        self.timer = self.create_timer(0.001, self.timer_callback)
        self.i = 0

    def timer_callback(self):
        msg = CompressedImage()
        msg.data = payload_generator(self.i)
        self.publisher_.publish(msg)
        self.i += 1

    def listener_callback(self, msg):
        print(payload_to_latency(msg.data))


def main(args=None):
    rclpy.init(args=args)

    minimal_publisher = MinimalPublisher()

    rclpy.spin(minimal_publisher)

    minimal_publisher.destroy_node()
    rclpy.shutdown()


if __name__ == '__main__':
    main()
