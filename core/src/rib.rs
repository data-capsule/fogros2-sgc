extern crate multimap;
use multimap::MultiMap;
use utils::app_config::AppConfig;

use crate::structs::{GDPName, GDPNameRecord, GDPStatus, GDPNameRecordType::*};

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
        self.dump();
        Some(())
    }

    pub fn get(&self, key: GDPName) -> Option<&Vec<GDPNameRecord>> {
        self.routing_table.get_vec(&key)
    }

    pub fn dump(&self) {
        info!("dumping RIB");
        for (key, value) in self.routing_table.iter() {
            println!("key: {:?}, value: {:?}", key, value);
        }
    }
}

//
pub async fn local_rib_handler(
    mut rib_query_rx: tokio::sync::mpsc::UnboundedReceiver<GDPNameRecord>,
    rib_response_tx: tokio::sync::mpsc::UnboundedSender<GDPNameRecord>,
    stat_tx: tokio::sync::mpsc::UnboundedSender<GDPStatus>,   
) {
    // TODO: currently, we only take one rx due to select! limitation
    // will use FutureUnordered Instead
    let _receive_handle = tokio::spawn(async move {
        let mut rib_store = RoutingInformationBase::new();

        // loop polling from
        loop {
            tokio::select! {
                // GDP Name Record received
                Some(query) = rib_query_rx.recv() => {
                    match query.record_type {
                        EMPTY => {
                            warn!("received empty RIB query")
                        },
                        QUERY => {
                            info!("received RIB query for {:?}", query.gdpname);
                            match rib_store.get(query.gdpname) {
                                Some(records) => {
                                    for record in records {
                                        info!("sending RIB query response for {:?}", record.gdpname);
                                        let record_response = GDPNameRecord{
                                            record_type: INFO,
                                            gdpname: record.gdpname, 
                                            source_gdpname: query.source_gdpname, // so that we can send it back to the source
                                            webrtc_offer: record.webrtc_offer.clone(), 
                                            ip_address: record.ip_address.clone(), 
                                            indirect: record.indirect, 
                                            ros: record.ros.clone(),
                                        };
                                        rib_response_tx.send(record_response.clone()).expect(
                                            "failed to send RIB query response"
                                        );
                                    }
                                },
                                None => {
                                    warn!("received RIB query for non-existing name");
                                    rib_response_tx.send(
                                        GDPNameRecord{
                                            record_type: EMPTY,
                                            gdpname: query.gdpname, 
                                            source_gdpname: query.source_gdpname, // identify the source of the query
                                            webrtc_offer: None, 
                                            ip_address: None, 
                                            indirect: None, 
                                            ros: None,
                                        }
                                    ).expect(
                                        "failed to send RIB query response"
                                    );
                                }
                            }
                        },
                        UPDATE => {
                            rib_store.put(query.gdpname, query.clone());
                        },
                        _ => {
                            warn!("received RIB query with unknown record type {:?}", query.record_type);
                        }
                    }
                }
            }
        }
    });
}
