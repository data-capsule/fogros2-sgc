use crate::structs::{GDPChannel, GDPName, GDPNameRecord, GDPPacket, GdpAction};
use tokio::sync::mpsc::UnboundedSender;

/// construct gdp struct from bytes
/// bytes is put as payload
pub fn construct_gdp_forward_from_bytes(
    destination: GDPName, source: GDPName, buffer: Vec<u8>,
) -> GDPPacket {
    GDPPacket {
        action: GdpAction::Forward,
        gdpname: destination,
        payload: Some(buffer),
        source: source,
        name_record: None,
    }
}

/// construct gdp struct from bytes
/// bytes is put as payload
pub fn construct_gdp_advertisement_from_structs(
    destination: GDPName, source: GDPName, name_record: GDPNameRecord,
) -> GDPPacket {
    GDPPacket {
        action: GdpAction::Advertise,
        gdpname: destination,
        source,
        payload: None,
        name_record: Some(name_record),
    }
}

pub fn construct_gdp_advertisement_from_bytes(
    destination: GDPName, source: GDPName, advertisement_packet: Vec<u8>,
) -> GDPPacket {
    if advertisement_packet.len() == 0 {
        warn!("advertisement packet is empty");
        return GDPPacket {
            action: GdpAction::Advertise,
            gdpname: destination,
            source: source,
            payload: None,
            name_record: None,
        };
    }
    GDPPacket {
        action: GdpAction::Advertise,
        gdpname: destination,
        source: source,
        payload: None,
        name_record: Some(
            serde_json::from_str::<GDPNameRecord>(
                std::str::from_utf8(&advertisement_packet).unwrap(),
            )
            .unwrap(),
        ),
    }
}

/// construct rib query from bytes
pub fn construct_rib_query_from_bytes(
    destination: GDPName, source: GDPName, name_record: GDPNameRecord,
) -> GDPPacket {
    GDPPacket {
        action: GdpAction::RibGet,
        gdpname: destination,
        source: source,
        payload: None,
        name_record: Some(name_record),
    }
}

/// parses the processsing received GDP packets
/// match GDP action and send the packets with corresponding actual actions
///
///  proc_gdp_packet(buf,  // packet
///               fib_tx.clone(),  //used to send packet to fib
///               channel_tx.clone(), // used to send GDPChannel to fib
///               m_tx.clone() //the sending handle of this connection
///  );
pub async fn proc_gdp_packet(
    gdp_packet: GDPPacket,
    fib_tx: &UnboundedSender<GDPPacket>, // used to send packet to fib
    channel_tx: &UnboundedSender<GDPChannel>, // used to send GDPChannel to fib
    m_tx: &UnboundedSender<GDPPacket>,   // the sending handle of this connection
    rib_tx: &UnboundedSender<GDPNameRecord>, // used to send packet to rib
    comment: String,
) {
    // Vec<u8> to GDP Packet
    // let gdp_packet = populate_gdp_struct(packet);
    let action = gdp_packet.action;
    let gdp_name = gdp_packet.gdpname;

    match action {
        GdpAction::Advertise => {
            // construct and send channel to RIB
            let channel = GDPChannel {
                gdpname: gdp_name,
                source: gdp_packet.source,
                channel: m_tx.clone(),
                comment,
            };
            if let Some(record) = gdp_packet.name_record {
                rib_tx.send(record).expect("send to rib failure");
            }
            // rib_tx.send(gdp_packet.name_record.unwrap()).expect("send to rib failure");
            channel_tx
                .send(channel)
                .expect("channel_tx channel closed!");
        }
        GdpAction::Forward => {
            // send the packet to RIB
            match fib_tx.send(gdp_packet) {
                Ok(_) => {}
                Err(_) => error!(
                    "Unable to forward the packet because connection channel orfib is closed"
                ),
            };
        }
        GdpAction::RibGet => {
            // handle fib query by responding with the RIB item
        }
        GdpAction::RibReply => {
            // update local fib with the fib reply
        }

        GdpAction::Noop => {
            // nothing to be done here
            println!("GDP Action Noop Detected, Make sure to update to 5 for forwarding");
        }
        _ => {
            // control, put, get
            println!("{:?} is not implemented yet", action)
        }
    }
}
