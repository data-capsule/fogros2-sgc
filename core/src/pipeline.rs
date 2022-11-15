use crate::structs::{GDPChannel, GDPName, GDPPacket, GdpAction};
use tokio::sync::mpsc::{self, Sender};

/// construct a gdp packet struct
/// we may want to use protobuf later

fn populate_gdp_struct(buffer: Vec<u8>)
-> GDPPacket
{                
    let mut pkt = GDPPacket{
        action: GdpAction::Noop,
        gdpname: GDPName([0; 4]),
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
    rib_tx: &Sender<GDPPacket>, //used to send packet to rib
    channel_tx: &Sender<GDPChannel>, // used to send GDPChannel to rib
    m_tx: &Sender<GDPPacket>, //the sending handle of this connection
) {

    // Vec<u8> to GDP Packet 
    let gdp_packet = populate_gdp_struct(packet);

    // let mut advertised_to_rib = false;

    // // TODO: we need a pipeline here


    let action = gdp_packet.action;
    match action{
        GdpAction::Advertise => {
            //construct and send channel to RIB
            let channel = GDPChannel {
                gdpname: GDPName([0; 4]),
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
