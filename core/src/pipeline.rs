use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction};
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
    }
}

/// construct gdp struct from bytes
/// bytes is put as payload
pub fn construct_gdp_advertisement_from_bytes(
    destination: GDPName, 
    source: GDPName,
    advertisement: Option<Vec<u8>>
) -> GDPPacket {
    GDPPacket {
        action: GdpAction::Advertise,
        gdpname: destination,
        source: source,
        payload: advertisement,
    }
}

/// parses the processsing received GDP packets
/// match GDP action and send the packets with corresponding actual actions
///
///  proc_gdp_packet(buf,  // packet
///               rib_tx.clone(),  //used to send packet to rib
///               channel_tx.clone(), // used to send GDPChannel to rib
///               m_tx.clone() //the sending handle of this connection
///  );
pub async fn proc_gdp_packet(
    gdp_packet: GDPPacket,
    rib_tx: &UnboundedSender<GDPPacket>, // used to send packet to rib
    channel_tx: &UnboundedSender<GDPChannel>, // used to send GDPChannel to rib
    m_tx: &UnboundedSender<GDPPacket>,   // the sending handle of this connection
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
                name_record: gdp_packet, // TODO: what to put here
            };
            channel_tx
                .send(channel)
                .expect("channel_tx channel closed!");
        }
        GdpAction::Forward => {
            // send the packet to RIB
            match rib_tx.send(gdp_packet) {
                Ok(_) => {}
                Err(_) => error!(
                    "Unable to forward the packet because connection channel or rib is closed"
                ),
            };
        }
        GdpAction::RibGet => {
            // handle rib query by responding with the RIB item
        }
        GdpAction::RibReply => {
            // update local rib with the rib reply
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
