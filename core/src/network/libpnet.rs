extern crate pnet;
extern crate pnet_macros_support;

use crate::pipeline_pnet::gdp_pipeline;
use crate::rib::RoutingInformationBase;
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use utils::app_config::AppConfig;

pub fn pnet_proc_loop() {
    let config = AppConfig::fetch();
    println!("Running with the following config: {:#?}", config);

    let iface_config = config.expect("Cannot find the config");
    let iface_name = "eno0";

    println!("Running with interface: {}", iface_name);
    let interface_names_match = |iface: &NetworkInterface| iface.name == iface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(interface_names_match)
        .unwrap_or_else(|| panic!("No such network interface: {}", iface_name));

    // Create a channel to receive on
    let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("packetdump: unhandled channel type"),
        Err(e) => panic!("packetdump: unable to create channel: {}", e),
    };

    //TODO: is there any better way of putting thefib? How to make it thread safe?
    let mut gdp_rib = RoutingInformationBase::new(&iface_config);

    loop {
        match rx.next() {
            Ok(packet) => {
                match gdp_pipeline(packet, &mut gdp_rib, &interface, &mut tx, &iface_config) {
                    Some(_) => {}
                    None => continue,
                }
            }
            Err(e) => panic!("packetdump: unable to receive packet: {}", e),
        }
    }
}
