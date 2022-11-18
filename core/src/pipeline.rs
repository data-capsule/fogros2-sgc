use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction};
use tokio::sync::mpsc::{self, Sender};

/// construct a gdp packet struct
/// we may want to use protobuf later
/// this part is facilitate testing only

fn populate_gdp_struct(buffer: Vec<u8>) -> GDPPacket {
    let received_str: Vec<&str> = std::str::from_utf8(&buffer)
        .unwrap()
        .trim()
        .split(",")
        .collect();
    let m_gdp_action = match received_str[0] {
        "ADV" => GdpAction::Advertise,
        "FWD" => GdpAction::Forward,
        _ => GdpAction::Noop,
    };

    let m_gdp_name = match &received_str[1][0..1] {
        "1" => GDPName([1, 1, 1, 1]),
        "2" => GDPName([2, 2, 2, 2]),
        _ => GDPName([0, 0, 0, 0]),
    };

    let mut pkt = GDPPacket {
        action: m_gdp_action,
        gdpname: m_gdp_name,
        payload: buffer,
    };

    pkt
}

/// parses the processsing received GDP packets
/// match GDP action and send the packets with corresponding actual actions
///
///  proc_gdp_packet(buf,  // packet
///               rib_tx.clone(),  //used to send packet to rib
///               channel_tx.clone(), // used to send GDPChannel to rib
///               m_tx.clone() //the sending handle of this connection
///  );
///
pub async fn proc_gdp_packet(
    packet: Vec<u8>,
    rib_tx: &Sender<GDPPacket>,      //used to send packet to rib
    channel_tx: &Sender<GDPChannel>, // used to send GDPChannel to rib
    m_tx: &Sender<GDPPacket>,        //the sending handle of this connection
) {
    // Vec<u8> to GDP Packet
    let gdp_packet = populate_gdp_struct(packet);
    let action = gdp_packet.action;
    let gdp_name = gdp_packet.gdpname;

    match action {
        GdpAction::Advertise => {
            //construct and send channel to RIB
            let channel = GDPChannel {
                gdpname: gdp_name,
                channel: m_tx.clone(),
            };
            channel_tx.send(channel).await;
        }
        GdpAction::Forward => {
            //send the packet to RIB
            rib_tx.send(gdp_packet).await;
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
