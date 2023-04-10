
# Connection RIB Design Doc

### Primitives 
There are four types of peering relationship in FogROS SGC. 
1. Source: Unidirectionally publish the message (ROS subscriber)
2. Sink: Unidirectionally subscribe the message (ROS publisher)
3. Peer: Peering with each other; may forward packet or expect to receive packets (DTLS connection); may get RIB queries 
4. RIB: no data is sent to the RIB, only `GDPAction::Advertisement`, `GDPAction::RibGet` and `GDPAction::RibReply` are sent/received 

### Design Considerations 
1. separate routing information management with data routing (e.g. announcing a ROS topic does not directly subscribe to the topic, the topic is subscribed only if there is an active subscriber in the network)
2. Use queries instead of flushing. 
3. Smooth Node shutdown 

### Workflow
```
On a new ROS topic (publisher):
    (ROS Topic Manager): advertise the name, register its status handle with RIB
    If status handle receives subscribe request:
        create a thread that subscribes to the topic
        (Note: we don't need to remove the old record because Source does not generate additional routing information)

On a new ROS topic (subscriber):
    (ROS Topic Manager): advertise the name, register its status handle with RIB
    If status handle receives publish advertisement:
        create a thread that publishes to the topic
        manager removes itself from subscribing request
        advertises the new thread that with subscribing request


RIB: 
On a new packet: 
match GDPAction with 
    Forward: 
        Check its connection_rib hash table 
        If name not found, 
            mark name as in query, drop the messages (or buffer) 
            send query to its own RIB and peers
            (TODO: send Nack that RIB cannot find the name)
        If name is found and sink/peer is not empty, 
            forward the packet 
    Advertisement: 
        Store information to its corresponding connection RIB
        Match NameType with 
            Source: add source to source rib
                if sink_rib has the name record
                    inform ros topic manager
            Sink: add sink to sink_rib
                if source_rib has the name record
                    inform ros topic manager
            Peer: add peer to rib
            RIB: (a new RIB!) register the routing information with the RIB
    RibGet: 
        search in RIB and get the IP address/port 
    RibResponse
        add and connect IP address/port by creating a new peering thread

```

### Q&A 
1. How does publish and subscribe eventually peered? Who will initiate this peering?

We are able to trace something like: [topic 256 bit name] is from [ROS local subscriber 256 bit name] which will be delivered from [DTLS peer 256 bit name].

2. currently we don't consider multiple path available. What needs to be done if we need to support OSPF? 

