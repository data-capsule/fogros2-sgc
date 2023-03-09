import rclpy
from rclpy.node import Node
from sensor_msgs.msg import CompressedImage
from std_msgs.msg import String
import time

class MinimalSubscriber(Node):

    def __init__(self):
        super().__init__('minimal_subscriber')
        self.start_time = time.time()
        self.received_msgs = 0
        self.subscription = self.create_subscription(
            CompressedImage,
            '/benchmark',
            self.callback,
            10)
        self.subscription  # prevent unused variable warning


    def callback(self, msg):
        self.received_msgs += 1
        if self.received_msgs == 1000:
            elapsed_time = time.time() - self.start_time
            msg_throughput = 1000 / elapsed_time
            print(f"{msg_throughput}")
            # self.get_logger().info(f'Received 100 messages in {elapsed_time:.2f} seconds, message throughput: {msg_throughput:.2f} messages/s')
            self.received_msgs = 0
            self.start_time = time.time()

def main(args=None):
    rclpy.init(args=args)

    minimal_subscriber = MinimalSubscriber()

    rclpy.spin(minimal_subscriber)

    # Destroy the node explicitly
    # (optional - otherwise it will be done automatically
    # when the garbage collector destroys the node object)
    minimal_subscriber.destroy_node()
    rclpy.shutdown()


if __name__ == '__main__':
    main()
