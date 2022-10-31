
extern crate pnet;
extern crate pnet_macros_support;

use pnet::packet::{Packet, MutablePacket};
use pnet::datalink::{self, NetworkInterface, DataLinkSender};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::udp::UdpPacket;
use pnet::util::MacAddr;
use pnet_packet::ipv4::{MutableIpv4Packet, checksum};
use pnet_packet::udp::MutableUdpPacket;

use std::env;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::process;

use crate::protocol::GDP_protocol::{GdpProtocolPacket, MutableGdpProtocolPacket};
use utils::app_config::AppConfig;


const LEFT: Ipv4Addr = Ipv4Addr::new(128, 32, 37, 69);
const RIGHT: Ipv4Addr = Ipv4Addr::new(128, 32, 37, 41);

fn handle_gdp_packet(
        interface_name: &str, 
        source: IpAddr, destination: IpAddr, 
        packet: &[u8], udp: &UdpPacket, 
        tx: &Box<dyn DataLinkSender>, 
        config: &AppConfig) -> Option<Vec<u8>> {
    let gdp_protocol_packet = GdpProtocolPacket::new(packet);
    
    
    if let Some(gdp) = gdp_protocol_packet {
        println!("Received GDP Packet: {:?}\n", gdp);

        // create new gdp packet
        // let mut vec: Vec<u8> = vec![0; packet.len()];
        // let mut new_packet = MutableUdpPacket::new(&mut vec[..]).unwrap();
        // new_packet.clone_from(udp);
        let mut vec: Vec<u8> = vec![0; gdp.packet().len()+50];
        let mut res_gdp = MutableGdpProtocolPacket::new(&mut vec[..]).unwrap();
        res_gdp.clone_from(&gdp);
        res_gdp.set_src_gdpname(&gdp.get_dst_gdpname());
        res_gdp.set_dst_gdpname(&gdp.get_src_gdpname());
        res_gdp.set_payload(("echo".to_owned() +  &String::from_utf8(gdp.payload().to_vec()).unwrap()).as_bytes());
        // println!("{:?}", String::from_utf8(res_gdp.payload().to_vec()));
        // println!("The constructed gdp packet is = {:?}\n", res_gdp);
        // println!("The buffer for the above packet is = {:?}\n", vec);
        Some(vec)
    } else {
        println!("Malformed GDP Packet");
        None
    }
}

fn handle_udp_packet(
    interface_name: &str, 
    source: IpAddr, 
    destination: IpAddr, 
    packet: &[u8], 
    tx: &Box<dyn DataLinkSender>, 
    config: &AppConfig) -> Option<Vec<u8>>
    {
    let udp = UdpPacket::new(packet);

    if let Some(udp) = udp {
        if udp.get_destination() == 31415 {
            // Assume all packets on port 31415 are valid GDP packets
            let res = handle_gdp_packet(interface_name, source, destination, udp.payload(), &udp, tx, config);
            if let Some(payload) = res {
                let mut vec: Vec<u8> = vec![0; 20+payload.len()]; // 20 B is the size of a UDP header
                let mut res_udp = MutableUdpPacket::new(&mut vec[..]).unwrap();
                res_udp.clone_from(&udp);
                res_udp.set_payload(&payload);
                println!("Constructed UDP packet = {:?}", res_udp);
                Some(vec)
            } else {None}
        } else {None}
    } else {
        println!("[{}]: Malformed UDP Packet", interface_name);
        None
    }
}


fn handle_transport_protocol(
    interface_name: &str,
    source: IpAddr,
    destination: IpAddr,
    protocol: IpNextHeaderProtocol,
    packet: &[u8],
    tx: &Box<dyn DataLinkSender>,
    config: &AppConfig
) -> Option<Vec<u8>>
{
    match protocol {
        IpNextHeaderProtocols::Udp => {
            handle_udp_packet(interface_name, source, destination, packet, tx, config)

        }
        
        _ => {None}
    }
}

fn handle_ipv4_packet(
    interface_name: &str, 
    ethernet: &EthernetPacket, 
    tx: &Box<dyn DataLinkSender>, 
    config: &AppConfig) -> Option<Vec<u8>>{
    let header = Ipv4Packet::new(ethernet.payload());
    if let Some(header) = header {
        // Filter packet not meant to be received (broadcast)
        let splitted_str:Vec<u8> = config.ip_local.split(".").map(|s| s.parse().unwrap()).collect();
        let m_ip = Ipv4Addr::new(splitted_str[0],splitted_str[1], splitted_str[2], splitted_str[3]);
        if header.get_destination() != m_ip {
            return None;
        }

        let res = handle_transport_protocol(
            interface_name,
            IpAddr::V4(header.get_source()),
            IpAddr::V4(header.get_destination()),
            header.get_next_level_protocol(),
            header.payload(),
            tx,
            config,
        );
        if let Some(payload) = res {
            let mut vec: Vec<u8> = vec![0; payload.len()+(header.get_header_length() as usize)*4]; // Multiply by 4 because ip header_length counting unit is word (4B)
            let mut res_ipv4 = MutableIpv4Packet::new(&mut vec[..]).unwrap();
            
            res_ipv4.set_total_length((payload.len()+(header.get_header_length() as usize)*4).try_into().unwrap());
            res_ipv4.set_payload(&payload);
            
            // Simple forwarding based on configuration
            res_ipv4.clone_from(&header);
            if header.get_source() == LEFT {
                res_ipv4.set_destination(RIGHT);
            } else if header.get_source() == RIGHT{
                res_ipv4.set_destination(LEFT);
            }
            // res_ipv4.set_destination(header.get_source());
            // res_ipv4.set_source(header.get_destination());
            
            let splitted_str:Vec<u8> = config.ip_local.split(".").map(|s| s.parse().unwrap()).collect();
            let m_ip = Ipv4Addr::new(splitted_str[0],splitted_str[1], splitted_str[2], splitted_str[3]);
            res_ipv4.set_source(m_ip);
            res_ipv4.set_checksum(checksum(&res_ipv4.to_immutable()));
            
            println!("Constructed IP packet = {:?}", res_ipv4);
            Some(vec)

        } else {None}
    } else {
        println!("[{}]: Malformed IPv4 Packet", interface_name);
        None
    }
}


// fn handle_ethernet_frame(interface: &NetworkInterface, ethernet: &EthernetPacket, tx: &mut Box<dyn DataLinkSender>, config: &AppConfig) {
//     let interface_name = &interface.name[..];
//     match ethernet.get_ethertype() {
//         EtherTypes::Ipv4 => {
//             let res = handle_ipv4_packet(interface_name, ethernet, tx, config);
//             if let Some(payload) = res {
//                 let mut vec: Vec<u8> = vec![0; 14 + payload.len()]; // 14 B is the size of an Ethernet header
//                 let mut res_ether = MutableEthernetPacket::new(&mut vec[..]).unwrap();
//                 res_ether.clone_from(ethernet);
//                 res_ether.set_payload(&payload);
//                 res_ether.set_destination(MacAddr::broadcast());
//                 res_ether.set_source(interface.mac.unwrap());
//                 println!("Constructed Ethernet packet = {:?}", res_ether);

//                 let result = tx.send_to(res_ether.packet(), None);
//                 match result {
//                     Some(Ok(temp))=>println!("{:?}", temp),
//                     _ => println!("")
//                 }
//             }
//         },
        
//         _ => {}
//     }
// }

pub fn pipeline() {
    use pnet::datalink::Channel::Ethernet;

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

    loop {
        match rx.next() {
            Ok(packet) => {
                //handle_ethernet_frame(&interface, &EthernetPacket::new(packet).unwrap(), &mut tx, &iface_config);
                let ethernet = EthernetPacket::new(packet).unwrap(); 
                if ethernet.get_ethertype() == EtherTypes::Ipv4 {
                    let res = handle_ipv4_packet("", &ethernet, &tx, &iface_config);
                    if let Some(payload) = res {
                        tx.build_and_send(1, 14 + payload.len(),
                            &mut |mut res_ether| {
                                let mut res_ether = MutableEthernetPacket::new(&mut res_ether).unwrap();

                                // Switch the source and destination
                                res_ether.clone_from(&ethernet);
                                res_ether.set_payload(&payload);
                                res_ether.set_destination(MacAddr::broadcast());
                                res_ether.set_source(interface.mac.unwrap());
                                println!("Constructed Ethernet packet = {:?}", res_ether);
                        });
                    }
                }
            }
            Err(e) => panic!("packetdump: unable to receive packet: {}", e),
        }
    }
}
