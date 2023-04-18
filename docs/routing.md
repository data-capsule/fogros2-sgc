
# Connection FIB and RIB Design Doc

### Design Considerations 
1. separate routing information management with data routing (e.g. announcing a ROS topic does not directly subscribe to the topic, the topic is subscribed only if there is an active subscriber in the network). This derives a design philosophy: *one can arbitrarily connect and advertise the names; the data is only exchanges conservatively and securely.* 
2. Use queries to replace unneeded flushing or broadcasting
3. Smooth Node shutdown 
4. allow indirections (point to another gdp name). after resolving, it will resolve to a piece of routing information or a specific channel of connection. 

This also implies the 256 bit name can be also mapped into an abstract entity, such as a virtual network interface or connection, or a running thread/process that handles the queries. 

### Primitives 
GDP Name records are stored in a format
```
// union in rust is unsafe, use struct instead
// name record is what being stored in RIB and used for routing
// one can resolve the GDPNameRecord using RIB put and get 
// it can be safely ported for another machine to connect
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GDPNameRecord {
    pub record_type: GDPNameRecordType,
    pub gdpname: GDPName,
    pub webrtc_offer: Option<String>,
    pub ip_address: Option<String>,
    pub ros: Option<(String, String)>,
    // indirect to another GDPName
    // this occurs if certain gdpname is hosted on a machine;
    // then we solve the GDP name to the machine's GDPName
    pub indirect: Option<GDPName>,  
}

// a CRDT friendly record type classification
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum GDPNameRecordType {
    EMPTY,
    INFO, // inform the existence of the record, does not replace if present
    QUERY, 
    UPDATE, // update the existing record by replacing the old one
    MERGE, // merge-able into the existing record
    DELETE,
}
```

### Architecture 

The current switching infrastructure is composed of 
* a connection **Forward Information Base** (FIB) that stores the mapping of 256 bit to ongoing connections, 
*  a **Routing Information Base** (RIB) that stores (semi) permanent records that facilitate connections. RIB does not route any packets.

When a packet comes in from one interface, FIB is consulted for the next hop; if the name is found, directly forward to the interface. Otherwise, FIB queries its local RIB. If the local RIB cannot find the name, broadcast a `RIBGET` query to all interfaces.

### Workflow
```
On a new ROS topic (publisher):
    (ROS Topic Manager): creates routing information struct for the name, advertise the name with RIB, register its status handle with FIB
    If status handle receives subscribe request:
        create a thread that subscribes to the ROS2 topic
        relay the ROS2 topic thread with the communication thread(DTLS/TCP/webRTC)

On a new ROS topic (subscriber):
    (ROS Topic Manager): advertise the name request and querying the RIB, register its status handle with FIB
    If status handle receives publish advertisement:
        create a thread that publishes to the ROS topic
        manager removes itself from subscribing request
        advertises the new thread that with name advertisement request
    Querying by broadcasting

On a new packet: 
match GDPAction with 
    Forward: 
        Check its FIB hash table 
        If name not found, 
            mark name as in query queue; send query to its own RIB and peers
            create a connection if the routing information is found
        If name is found and sink/peer is not empty, 
            forward the packet 
    Advertisement: 
        Store information to its corresponding connection FIB
        Match NameType with 
            Source: add source to source rib
                if sink_rib has the name record
                    inform ros topic manager
            Sink: add sink to sink_rib
                if source_rib has the name record
                    inform ros topic manager
            Peer: add peer to rib
            RIB: (a new RIB!) register the routing information with the FIB
    AdvertisementResponse: 
        route the query to the corresponding protocol handler
    RibGet: 
        forward a async message to RIB for the query
        search in RIB and get the IP address/port 
    RibResponse
        add and connect IP address/port by creating a new peering thread

```

### Q&A 
1. How does publish and subscribe eventually peered? Who will initiate this peering?

We are able to trace something like: [topic 256 bit name] is from [ROS local subscriber 256 bit name] which will be delivered from [DTLS peer 256 bit name].

2. currently we don't consider multiple path available. What needs to be done if we need to support OSPF? 

3. what if there is a ros subscriber before ros publisher

subscriber keeps querying the publisher?