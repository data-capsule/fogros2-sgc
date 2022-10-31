from scapy.all import *
import threading
from utils import *
import time
from multiprocessing import Process


import socket
ETH_P_ALL = 3
interface = "eno1"
ETH_FRAME_LEN = 1514

s = socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.htons(ETH_P_ALL))
s.bind((interface, 0))

RECEIVER_IP = "128.32.37.48"  # gdpmobile8
LOCAL_IP = get_local_ip()


def send_raw(buf):
    s.sendall(raw(buf))

def sniff_raw(prn):
    while True:
        data = s.recv(ETH_FRAME_LEN)
        prn(Ether(data))

class GDP(Packet):
    name = "GDP"
    fields_desc = [
        BitField("src_gdpname", 0, 256),  # 32
        BitField("dst_gdpname", 0, 256),  # 32
        BitField("uuid", 0, 128),  # 16
        IntField("num_packets", 1),  # 4
        IntField("packet_no", 1),  # 4
        ShortField("data_len", 0),  # 2
        # 1        # 5 means advertise topic, 6 means topic message push
        ByteField("action", 1),
        ByteField("ttl", 64)  # 1
    ]


bind_layers(UDP, GDP, dport=31415)


def start_sniffing(for_each):
    sniff_raw(prn=for_each)


def packet_recv_timestamp(packet):

    if not packet.haslayer(GDP):
        return
    print("Received packet {} at {}".format(packet.load, time.time()))


def gen_packet(packet_number):
    uuid = generate_uuid()
    return Ether(dst='ff:ff:ff:ff:ff:ff') / \
        IP(src=LOCAL_IP, dst=RECEIVER_IP) / \
        UDP(sport=31415, dport=31415) / \
        GDP(
        src_gdpname=0,
        dst_gdpname=0,
        action=0,
        data_len=1,
        uuid=uuid,
        packet_no=1,
        num_packets=1
    ) / \
        str(packet_number)


def gen_echo(packet_number, src_ip):
    uuid = generate_uuid()
    return Ether(dst='ff:ff:ff:ff:ff:ff') / \
        IP(src=LOCAL_IP, dst=src_ip) / \
        UDP(sport=31415, dport=31415) / \
        GDP(
        src_gdpname=0,
        dst_gdpname=0,
        action=0,
        data_len=1,
        uuid=uuid,
        packet_no=1,
        num_packets=1
    ) / \
        "echo{}".format(packet_number)


def sender_workload(num_packets):
    for i in range(1, num_packets+1):  # send dummy packets
        p = gen_packet(i)
        print("Sending packet {} at {}".format(p.load, time.time()))
        send_raw(p)


def echo(packet):
    if not packet.haslayer(GDP) or packet.load.decode()[0:4] == "echo":
        return
    ip_layer = packet.getlayer(IP)
    echo_packet = gen_echo(packet.load.decode(), ip_layer.src)
    send_raw(echo_packet)


def receiver_workload():
    sniff_raw(prn=echo)


def main():
    if sys.argv[1] == 'sender':
        t = threading.Thread(target=start_sniffing, args=(
            lambda packet: packet_recv_timestamp(packet),))
        t.start()
        import time 
        
        while True:
            num_packets = 1
            time.sleep(1)
            sender_workload(num_packets)

    if sys.argv[1] == "receiver":
        receiver_workload()


if __name__ == '__main__':
    main()
