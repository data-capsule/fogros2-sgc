use crate::{
    pipeline::construct_gdp_advertisement_from_bytes,
    structs::{GDPChannel, GDPName, GDPPacket, GDPStatus, GDPNameRecord},
};
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

async fn send_to_destination(destinations: Vec<GDPChannel>, packet: GDPPacket) {
    for dst in destinations {
        info!(
            "data {} from {} send to {}",
            packet.gdpname, packet.source, dst.source
        );
        if dst.source == packet.source {
            info!("Equal to the source, skipped!");
            continue;
        }
        let result = dst.channel.send(packet.clone());
        match result {
            Ok(_) => {}
            Err(_) => {
                warn!("Send Failure: channel sent to destination is closed");
            }
        }
    }
}

/// receive, check, and route GDP messages
///
/// receive from a pool of receiver connections (one per interface)
/// use a hash table to figure out the corresponding
///     hash table <gdp_name, send_tx>
///     TODO: use future if the destination is unknown
/// forward the packet to corresponding send_tx
pub async fn connection_fib(
    mut fib_rx: UnboundedReceiver<GDPPacket>, 
    rib_tx: UnboundedSender<GDPNameRecord>,
    mut stat_rs: UnboundedReceiver<GDPStatus>,
    mut channel_rx: UnboundedReceiver<GDPChannel>,
) {
    
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let _receive_handle = tokio::spawn(async move {
        let mut coonection_rib_table: HashMap<GDPName, Vec<GDPChannel>> = HashMap::new();
        let mut counter = 0;


        // loop polling from
        loop {
            tokio::select! {
                // GDP packet received
                // recv () -> find_where_to_route() -> route()
                Some(pkt) = fib_rx.recv() => {
                    counter += 1;
                    info!("RIB received the packet #{} with name {}", counter, &pkt.gdpname);


                    // find where to route
                    match coonection_rib_table.get(&pkt.gdpname) {
                        Some(routing_dsts) => {
                            send_to_destination(routing_dsts.clone(), pkt).await;
                            // for dst in coonection_rib_table.values(){
                            //     info!("data {} from {} send to {}", pkt.gdpname, pkt.source, dst.advertisement.source);
                            //     if dst.advertisement.source == pkt.source {
                            //         continue;
                            //     }
                            //     send_to_destination(dst.channel.clone(), pkt.clone()).await;s
                            // }
                        }
                        None => {
                            info!("{:} is not there, broadcasting...", pkt.gdpname);
                            for routing_dsts in coonection_rib_table.values(){
                                send_to_destination(routing_dsts.clone(), pkt.clone()).await;
                            }
                        }
                    }
                }

                // connectionfib advertisement received
                Some(channel) = channel_rx.recv() => {
                    info!("channel registry received {:?}", channel);
                    // broadcast_advertisement(&channel, &coonection_rib_table).await;


                    // coonection_rib_table.insert(
                    //     channel.gdpname,
                    //     channel.channel
                    // );
                    // if let Some(offer) = &channel.advertisement.payload {
                    //     m_webrtc_offer = Some(offer.clone());
                    // }
                    match  coonection_rib_table.get_mut(&channel.gdpname) {
                        Some(v) => {
                            info!("adding to connectionfib vec");
                            v.push(channel)
                        }
                        None =>{
                            info!("Creating a new entry of gdp name");
                            coonection_rib_table.insert(
                                channel.gdpname,
                                vec!(channel),
                            );
                        }
                    };

                },

                // TODO: update rib here, instead of fib
                Some(update) = stat_rs.recv() => {
                    // Note: incomplete implementation, only support flushing advertisement
                    let dst = update.sink;
                    for (name, channel) in &coonection_rib_table {
                        info!("flushing advertisement for {} to {:?}", name, dst);
                        let packet = construct_gdp_advertisement_from_bytes(
                            *name, 
                            *name,
                            None 
                        );

                        let result = dst.send(packet.clone());
                        match result {
                            Ok(_) => {}
                            Err(_) => {
                                warn!("Send Failure: channel sent to destination is closed");
                            }
                        }
                    }
                }
            }
        }
    });
}
