

extern crate kv;

use kv::*;

pub struct RoutingInformationBase {
    pub routing_table : Store,   // for now, we assume all routing table maps to IP addr
} 


impl RoutingInformationBase {
    pub fn new(path: &String) -> RoutingInformationBase {
        let mut cfg = Config::new(path);

        // Open the key/value store
        let store = Store::new(cfg).unwrap();
    
        // A Bucket provides typed access to a section of the key/value store

        RoutingInformationBase{
            routing_table: store
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Error> {
        let test = self.routing_table.bucket::<Vec<u8>, Vec<u8>>(Some("test")).unwrap();
        test.set(&key, &value)?;
        Ok(())
    }

    pub fn get(&mut self, key: Vec<u8>) -> Option<Vec<u8>> {
        let test = self.routing_table.bucket::<Vec<u8>, Vec<u8>>(Some("test")).unwrap();
        match test.get(&key) {
            Ok(value) => value,
            _ => None,
        }
    }


}