# WebRTC Design Doc

As mentioned in the routing document, supporting a directory of routing rules with multiple receivers is hard. Instead of coming up with our own, we use webRTC, which should be able to satisfy the majority of the use case. The biggest difference is that, on WebRTC, we exchange the tokens with the existing architecture, and establish a direct connection from the tokens, and RIB stores the tokens of the webRTC. 

SGC publishes a node; advertises the mapping of 256 bit name with the Offer token of the node. The nodes in the middle can facilitate propagation of the 256 bit names. SGC needs to route the Answer token back. 

RIB: stores ROS node hosted machine <-> token mapping. 

One publisher, Multiple subscribers: every topic has an offer, subscriber supplies with the answer. create a separate peer to peer relationship by exchanging the offer and answer. To break the tie, we use publisher as the entity that issues and publishes offers, the subscriber poll/query the offers. 

On node create, create a RTC listener; advertise the node name with the RTC listener whenever there is a node.

### Q&A
> What is the tradeoff between this and the direct DTLS approach? 

For now webRTC helps us to reduce the hops requires in the middle to communicate by NAT traversing. Currently we need a router in the middle to directly route the data, but webRTC resolve this issue. 

### Extension
* Since webrtc is browser, potentially we can use that to render with a website visualization. 
* We also hope to use webRTC's H264 features