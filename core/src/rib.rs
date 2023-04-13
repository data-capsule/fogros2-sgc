extern crate multimap;
use multimap::MultiMap;
use utils::app_config::AppConfig;

use crate::structs::{GDPName, GDPNameRecord};

// an interface to rib
// will support queries
// a CRDT based RIB
pub struct RoutingInformationBase {
    pub routing_table: MultiMap<GDPName, GDPNameRecord>,
}

impl RoutingInformationBase {
    pub fn new() -> RoutingInformationBase {
        // TODO: config can populate the RIB somehow

        RoutingInformationBase {
            routing_table: MultiMap::new()
        }
    }

    pub fn put(&mut self, key: GDPName, value: GDPNameRecord) -> Option<()> {
        self.routing_table.insert(key, value);
        Some(())
    }

    pub fn get(&self, key: GDPName) -> Option<&Vec<GDPNameRecord>> {
        self.routing_table.get_vec(&key)
    }
}

//
pub async fn local_rib_handler(
    mut rib_rx: tokio::sync::mpsc::UnboundedReceiver<GDPNameRecord>,
) {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let _receive_handle = tokio::spawn(async move {
        let mut rib_store = RoutingInformationBase::new();
        let mut counter = 0;

        // loop polling from
        loop {
            tokio::select! {
                // GDP Name Record received
                Some(pkt) = rib_rx.recv() => {
                }
            }
        }
    });
}
