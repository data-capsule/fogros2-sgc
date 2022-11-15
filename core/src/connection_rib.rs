
use tokio::{sync::mpsc::{self, Sender, Receiver}};
use crate::structs::{GDPPacket, GDPChannel};
use std::collections::HashMap;

/// receive, check, and route GDP messages 
/// 
/// receive from a pool of receiver connections (one per interface)
/// use a hash table to figure out the corresponding 
///     hash table <gdp_name, send_tx>
///     TODO: use future if the destination is unknown
/// forward the packet to corresponding send_tx
pub async fn connection_router(
    mut rib_rx: Receiver<GDPPacket>, 
    mut channel_rx: Receiver<GDPChannel>)  {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let receive_handle = tokio::spawn(async move {
        let mut coonection_rib_table = HashMap::new();

        // loop of recv () -> find_where_to_route() -> route()
        loop {
            let mut pkt:Option<GDPPacket> = None;

            tokio::select! {
                f = rib_rx.recv() => pkt = f,
                Some(channel) = channel_rx.recv() => {
                    coonection_rib_table.insert(
                        channel.name, 
                        channel.channel
                    );
                },
            }

            if let Some(pkt) = &pkt {
                println!("9999: {pkt}");
            }

            pkt = None;
        }
    });
}
