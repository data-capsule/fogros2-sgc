

extern crate multimap; 
use multimap::MultiMap;
use std::net::Ipv4Addr;

pub struct RoutingInformationBase {
    pub routing_table: MultiMap<Vec<u8>, Ipv4Addr>,  
} 


impl RoutingInformationBase {
    pub fn new() -> RoutingInformationBase {

        RoutingInformationBase{
            routing_table: MultiMap::new(),
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Ipv4Addr) -> Option<()> {
        self.routing_table.insert(key, value);
        Some(())
    }

    pub fn get(&mut self, key: Vec<u8>) -> Option<&Vec<Ipv4Addr>> {
        self.routing_table.get_vec(&key)
    }
}