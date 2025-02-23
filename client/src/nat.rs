use std::{collections::HashMap, net::Ipv4Addr};

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct Network {
    ip: Ipv4Addr,
    netmask: u8,
}
impl Network {
    pub fn new(ip: Ipv4Addr, netmask: u8) -> Self {
        Self { ip, netmask }
    }
    pub fn is_in_network(&self, ip: Ipv4Addr) -> bool {
        (ip.to_bits() & (0xFFFFFFFF << (32 - self.netmask))) == self.ip.to_bits()
    }
}

pub struct NATTable {
    table: HashMap<Network, Network>,
}

impl NATTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn add_entry(&mut self, local: Network, remote: Network) {
        self.table.insert(local, remote);
    }

    /// 正引き
    pub fn convert(&self, ip: Ipv4Addr) -> Ipv4Addr {
        for (local, remote) in self.table.iter() {
            if local.is_in_network(ip) {
                return Ipv4Addr::from(
                    (ip.to_bits() & 0xFFFFFFFF >> (local.netmask)) | remote.ip.to_bits(),
                );
            }
        }
        ip
    }
    /// 逆引き
    pub fn reverse(&self, ip: Ipv4Addr) -> Ipv4Addr {
        for (local, remote) in self.table.iter() {
            if remote.is_in_network(ip) {
                return Ipv4Addr::from(
                    (ip.to_bits() & 0xFFFFFFFF >> (remote.netmask)) | local.ip.to_bits(),
                );
            }
        }
        ip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_in_network() {
        let network = Network::new(Ipv4Addr::new(10, 1, 0, 0), 24);
        let ip = Ipv4Addr::new(10, 1, 0, 1);
        assert!(network.is_in_network(ip));

        let ip = Ipv4Addr::new(10, 2, 0, 2);
        assert!(!network.is_in_network(ip));

        let ip = Ipv4Addr::new(1, 1, 1, 1);
        assert!(!network.is_in_network(ip));
    }

    #[test]
    fn test_nat_table_convert() {
        let mut nat_table = NATTable::new();
        nat_table.add_entry(
            Network::new(Ipv4Addr::new(10, 1, 0, 0), 24),
            Network::new(Ipv4Addr::new(10, 0, 0, 0), 24),
        );

        let ip = Ipv4Addr::new(10, 1, 0, 1);
        let converted = nat_table.convert(ip);
        assert_eq!(converted, Ipv4Addr::new(10, 0, 0, 1));

        let ip = Ipv4Addr::new(10, 0, 0, 2);
        let converted = nat_table.convert(ip);
        assert_eq!(converted, Ipv4Addr::new(10, 0, 0, 2));

        let ip = Ipv4Addr::new(1, 1, 1, 1);
        let converted = nat_table.convert(ip);
        assert_eq!(converted, Ipv4Addr::new(1, 1, 1, 1));
    }

    #[test]
    fn test_nat_table_reverse() {
        let mut nat_table = NATTable::new();
        nat_table.add_entry(
            Network::new(Ipv4Addr::new(10, 1, 0, 0), 24),
            Network::new(Ipv4Addr::new(10, 0, 0, 0), 24),
        );

        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let reversed = nat_table.reverse(ip);
        assert_eq!(reversed, Ipv4Addr::new(10, 1, 0, 1));

        let ip = Ipv4Addr::new(10, 1, 0, 2);
        let reversed = nat_table.reverse(ip);
        assert_eq!(reversed, Ipv4Addr::new(10, 1, 0, 2));

        let ip = Ipv4Addr::new(1, 1, 1, 1);
        let reversed = nat_table.reverse(ip);
        assert_eq!(reversed, Ipv4Addr::new(1, 1, 1, 1));
    }
}
