import rclpy
from rclpy.node import Node
from std_msgs.msg import String
from time import time 

def payload_generator(id, size = 1):
    return str(id) + "," + str(time()) + "," + "1" * size

def payload_to_latency(payload):
    ts = float(payload.split(",")[1])
    return time() - ts

class MinimalPublisher(Node):

    def __init__(self):
        super().__init__('minimal_publisher')
        self.publisher_ = self.create_publisher(String, 'benchmark', 10)
        self.subscription = self.create_subscription(
            String,
            '/benchmark_echo',
            self.listener_callback,
            10)
        timer_period = 1  # seconds
        self.timer = self.create_timer(timer_period, self.timer_callback)
        self.i = 0

    def timer_callback(self):
        msg = String()
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
