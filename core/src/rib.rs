

extern crate multimap; 
use multimap::MultiMap;
use std::net::Ipv4Addr;
use utils::app_config::AppConfig;
use utils::conversion::str_to_ipv4;

pub struct RoutingInformationBase {
    pub routing_table: MultiMap<Vec<u8>, Ipv4Addr>,  
    pub default_route: Vec<Ipv4Addr>,
} 


impl RoutingInformationBase {
    pub fn new(config: &AppConfig) -> RoutingInformationBase {
        // TODO: config can populate the RIB somehow 
        let m_gateway_addr = str_to_ipv4(&config.ip_gateway);

        RoutingInformationBase{
            routing_table: MultiMap::new(),
            default_route: Vec::from([m_gateway_addr])
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Ipv4Addr) -> Option<()> {
        self.routing_table.insert(key, value);
        Some(())
    }

    // rust doesn't support default param.
    pub fn get_no_default(& self, key: Vec<u8>) -> Option<&Vec<Ipv4Addr>> {
        self.routing_table.get_vec(&key)
    }

    pub fn get(&self, key: Vec<u8>)-> &Vec<Ipv4Addr> {
        match self.get_no_default(key) {
            Some(v) => v, 
            None => &self.default_route
        }
    }
    
}