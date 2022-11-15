
use tokio::{sync::mpsc::{self, channel, Sender, Receiver}};
use crate::structs::GDPPacket;

/// receive, check, and route GDP messages 
/// 
/// receive from a pool of receiver connections (one per interface)
/// use a hash table to figure out the corresponding 
///     hash table <gdp_name, send_tx>
///     TODO: use future if the destination is unknown
/// forward the packet to corresponding send_tx
pub async fn connection_router(mut rx: Receiver<GDPPacket>)  {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let receive_handle = tokio::spawn(async move {
        let mut pkt:Option<GDPPacket> = None;

        loop {
            tokio::select! {
                f = rx.recv() => pkt = f,
            }

            if let Some(pkt) = &pkt {
                println!("9999: {pkt}");
            }

            pkt = None;
        }
    });
}
