# WebRTC Design Doc


### WebRTC: why is it good for firewalls  
When two endpoints (such as two browsers) want to communicate, they use a signaling protocol, such as WebSocket or SIP, to exchange information about the connection. This includes information such as network addresses and connection parameters.
WebRTC also includes NAT traversal techniques to allow for communication between endpoints behind firewalls and routers, which can often block direct connections. 

This is achieved through a technique called ICE (Interactive Connectivity Establishment), which uses multiple network paths to find the most efficient way for endpoints to communicate. ICE uses a process called candidate gathering. This process involves gathering information about the local network interface, such as its IP address, and creating a list of potential candidates for establishing a connection. These candidates include STUN (Session Traversal Utilities for NAT) and TURN (Traversal Using Relay NAT) servers. 

Once both endpoints have exchanged their candidate lists, they use a process called connectivity checks to determine which candidates are capable of establishing a direct communication channel. This involves sending test packets to each candidate to check if they can be reached directly. If a candidate is not reachable, the endpoint can try to use another candidate until a direct connection can be established.

### Gateways / Signaling servers
Having gateways is still in scope. FogROS robot and cloud needs to connect without knowing each other's IP address. What we do is that having robots to map the offer token to the cloud (FogROS) without knowing the IP address of the robot. Then we don't need to know the IP address of the robot and cloud. 

### How it works with ROS


### Q&A
> What is the tradeoff between this and the direct DTLS approach? 

For now webRTC helps us to reduce the hops requires in the middle to communicate by NAT traversing. Currently we need a router in the middle to directly route the data, but webRTC resolve this issue. 

Direct DTLS also relies on knowing the IP address. WebRTC can resolve this problem.

> Why not just use webRTC directly? 

Having single webRTC under one name/topic is not possible. We need to store the mapping of ROS-GDP name to the publishers. And have subscribers to subscribe to the publishers individually.

### Extension
* Since webrtc is browser, potentially we can use that to render with a website visualization. 
* We also hope to use webRTC's H264 features


### Workflow Demo

```
Client 3251357a connected
Client 4626b02b connected
Client 4626b02b << {"id":"3251357a","payload":{"sdp":"v=0\r\no=rtc 4075634777 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic:WMS *\r\na=setup:actpass\r\na=ice-ufrag:3EXV\r\na=ice-pwd:k3OioG5qa8hgezsqDJ4CiY\r\na=ice-options:ice2,trickle\r\na=fingerprint:sha-256 5A:75:FE:A5:CC:96:53:FA:88:E8:98:45:E6:3A:33:D8:17:08:13:BB:CC:21:C7:BF:69:0C:FD:A2:22:61:96:20\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=mid:0\r\na=sendrecv\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n","type":"offer"}}
Client 3251357a >> {"id":"4626b02b","payload":{"sdp":"v=0\r\no=rtc 4075634777 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic:WMS *\r\na=setup:actpass\r\na=ice-ufrag:3EXV\r\na=ice-pwd:k3OioG5qa8hgezsqDJ4CiY\r\na=ice-options:ice2,trickle\r\na=fingerprint:sha-256 5A:75:FE:A5:CC:96:53:FA:88:E8:98:45:E6:3A:33:D8:17:08:13:BB:CC:21:C7:BF:69:0C:FD:A2:22:61:96:20\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=mid:0\r\na=sendrecv\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n","type":"offer"}}
Client 3251357a << {"id":"4626b02b","payload":{"sdp":"v=0\r\no=rtc 2723413780 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic:WMS *\r\na=setup:active\r\na=ice-ufrag:izJy\r\na=ice-pwd:1T4+9vuq6x2iGCg2Whj7Q1\r\na=ice-options:ice2,trickle\r\na=fingerprint:sha-256 F1:52:FA:D2:16:05:37:38:1F:40:EC:4A:83:FF:E3:BC:74:60:02:D7:49:1E:59:D9:E7:74:25:7C:1B:4D:90:1C\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=mid:0\r\na=sendrecv\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n","type":"answer"}}
Client 4626b02b >> {"id":"3251357a","payload":{"sdp":"v=0\r\no=rtc 2723413780 0 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\na=group:BUNDLE 0\r\na=msid-semantic:WMS *\r\na=setup:active\r\na=ice-ufrag:izJy\r\na=ice-pwd:1T4+9vuq6x2iGCg2Whj7Q1\r\na=ice-options:ice2,trickle\r\na=fingerprint:sha-256 F1:52:FA:D2:16:05:37:38:1F:40:EC:4A:83:FF:E3:BC:74:60:02:D7:49:1E:59:D9:E7:74:25:7C:1B:4D:90:1C\r\nm=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\nc=IN IP4 0.0.0.0\r\na=mid:0\r\na=sendrecv\r\na=sctp-port:5000\r\na=max-message-size:262144\r\n","type":"answer"}}
Client 4626b02b << {"id":"3251357a","payload":{"candidate":"candidate:1 1 UDP 2122317823 10.5.0.6 53578 typ host","sdpMid":"0"}}
Client 3251357a >> {"id":"4626b02b","payload":{"candidate":"candidate:1 1 UDP 2122317823 10.5.0.6 53578 typ host","sdpMid":"0"}}
Client 3251357a << {"id":"4626b02b","payload":{"candidate":"candidate:1 1 UDP 2122317823 10.5.0.5 47510 typ host","sdpMid":"0"}}
Client 4626b02b >> {"id":"3251357a","payload":{"candidate":"candidate:1 1 UDP 2122317823 10.5.0.5 47510 typ host","sdpMid":"0"}}
Client 4626b02b << {"id":"3251357a","payload":{"candidate":"candidate:3 1 UDP 1686109695 128.32.37.48 53578 typ srflx raddr 0.0.0.0 rport 0","sdpMid":"0"}}
Client 3251357a >> {"id":"4626b02b","payload":{"candidate":"candidate:3 1 UDP 1686109695 128.32.37.48 53578 typ srflx raddr 0.0.0.0 rport 0","sdpMid":"0"}}
Client 3251357a << {"id":"4626b02b","payload":{"candidate":"candidate:2 1 UDP 1686109951 128.32.37.48 47510 typ srflx raddr 0.0.0.0 rport 0","sdpMid":"0"}}
Client 4626b02b >> {"id":"3251357a","payload":{"candidate":"candidate:2 1 UDP 1686109951 128.32.37.48 47510 typ srflx raddr 0.0.0.0 rport 0","sdpMid":"0"}}
Client 3251357a disconnected
Client 4626b02b disconnected
```