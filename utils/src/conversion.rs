use std::net::Ipv4Addr;

pub fn str_to_ipv4(ip_addr_str: &String) -> Ipv4Addr {
    let splitted_str: Vec<u8> = ip_addr_str.split('.').map(|s| s.parse().unwrap()).collect();

    Ipv4Addr::new(
        splitted_str[0],
        splitted_str[1],
        splitted_str[2],
        splitted_str[3],
    )
}
