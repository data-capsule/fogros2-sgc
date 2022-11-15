use crate::structs::{GDPChannel, GDPName, GDPPacket};
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver, Sender};

/// receive, check, and route GDP messages
///
/// receive from a pool of receiver connections (one per interface)
/// use a hash table to figure out the corresponding
///     hash table <gdp_name, send_tx>
///     TODO: use future if the destination is unknown
/// forward the packet to corresponding send_tx
pub async fn connection_router(
    mut rib_rx: Receiver<GDPPacket>, mut channel_rx: Receiver<GDPChannel>,
) {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let receive_handle = tokio::spawn(async move {
        let mut coonection_rib_table: HashMap<GDPName, Sender<GDPPacket>> = HashMap::new();

        // loop polling from
        loop {
            tokio::select! {
                // GDP packet received
                // recv () -> find_where_to_route() -> route()
                Some(pkt) = rib_rx.recv() => {
                    println!("forwarder received: {pkt}");

                    // find where to route
                    match coonection_rib_table.get(&pkt.gdpname) {
                        Some(routing_dst) => {
                            println!("fwd!");
                            routing_dst.send(pkt).await;
                        }
                        None => {
                            println!("{:} is not there.", pkt.gdpname);
                        }
                    }
                }

                // rib advertisement received
                Some(channel) = channel_rx.recv() => {
                    println!("channel registry received {:}", channel.gdpname);
                    coonection_rib_table.insert(
                        channel.gdpname,
                        channel.channel
                    );
                },
            }
        }
    });
}
