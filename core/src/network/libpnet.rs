extern crate pnet;
extern crate pnet_macros_support;

use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, DataLinkSender, NetworkInterface};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::udp::UdpPacket;
use pnet::packet::{MutablePacket, Packet};
use pnet::util::MacAddr;
use pnet_packet::ipv4::{checksum, MutableIpv4Packet};
use pnet_packet::udp::MutableUdpPacket;
use std::net::Ipv4Addr;

use crate::pipeline::gdp_pipeline;
use crate::protocol::GDP_protocol::{GdpProtocolPacket, MutableGdpProtocolPacket};
use crate::rib::RoutingInformationBase;
use utils::app_config::AppConfig;
use utils::conversion::str_to_ipv4;

pub fn pnet_proc_loop() {
    let config = AppConfig::fetch();
    println!("Running with the following config: {:#?}", config);

    let iface_config = config.expect("Cannot find the config");
    let iface_name = iface_config.net_interface.clone();

    println!("Running with interface: {}", iface_name);
    let interface_names_match = |iface: &NetworkInterface| iface.name == iface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .filter(interface_names_match)
        .next()
        .unwrap_or_else(|| panic!("No such network interface: {}", iface_name));

    // Create a channel to receive on
    let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("packetdump: unhandled channel type"),
        Err(e) => panic!("packetdump: unable to create channel: {}", e),
    };

    //TODO: is there any better way of putting the rib? How to make it thread safe?
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
