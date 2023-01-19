use crate::gdp_proto::GdpUpdate;
use crate::structs::{GDPChannel, GDPName, GDPPacket};
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};

/// receive, check, and route GDP messages
///
/// receive from a pool of receiver connections (one per interface)
/// use a hash table to figure out the corresponding
///     hash table <gdp_name, send_tx>
///     TODO: use future if the destination is unknown
/// forward the packet to corresponding send_tx
pub async fn connection_router(
    mut rib_rx: Receiver<GDPPacket>, mut stat_rs: Receiver<GdpUpdate>,
    mut channel_rx: Receiver<GDPChannel>,
) {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let _receive_handle = tokio::spawn(async move {
        let mut coonection_rib_table: HashMap<GDPName, GDPChannel> = HashMap::new();

        // loop polling from
        loop {
            tokio::select! {
                // GDP packet received
                // recv () -> find_where_to_route() -> route()
                Some(pkt) = rib_rx.recv() => {
                    info!("forwarder received: {pkt}");

                    // find where to route
                    match coonection_rib_table.get(&pkt.gdpname) {
                        Some(routing_dst) => {
                            
                            //routing_dst.send(pkt).await.expect("RIB: remote connection closed");
                            for dst in coonection_rib_table.values(){
                                info!("data from {} send to {}", pkt.source, dst.advertisement.source);
                                if (dst.advertisement.source == pkt.source){
                                    continue;
                                }
                                dst.channel.send(pkt.clone()).await.expect("RIB: remote connection closed");
                            }
                        }
                        None => {
                            info!("{:} is not there, broadcasting...", pkt.gdpname);
                            for dst in coonection_rib_table.values(){
                                info!("data from {} send to {}", pkt.source, dst.advertisement.source);
                                if (dst.advertisement.source == pkt.source){
                                    continue;
                                }
                                dst.channel.send(pkt.clone()).await.expect("RIB: remote connection closed");
                            }
                        }
                    }
                }

                // connection rib advertisement received
                Some(channel) = channel_rx.recv() => {
                    info!("channel registry received {:}", channel.gdpname);
                    info!("broadcasting...");
                    for dst in coonection_rib_table.values(){
                        info!("advertisement of {} is sent to channel {}",dst.advertisement.source, channel.advertisement.source);
                        if (dst.advertisement.source == channel.advertisement.source){
                            continue;
                        }
                        dst.channel.send(channel.advertisement.clone()).await.expect("RIB: remote connection closed");
                    }
                    // coonection_rib_table.insert(
                    //     channel.gdpname,
                    //     channel.channel
                    // );
                    coonection_rib_table.insert(
                        channel.gdpname,
                        channel
                    );           
                },

                Some(update) = stat_rs.recv() => {
                    //TODO: update rib here
                }
            }
        }
    });
}
