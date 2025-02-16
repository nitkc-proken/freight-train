use std::net::{AddrParseError, Ipv4Addr};

use futures::TryStreamExt;
use netlink_packet_route::link::{InfoData, InfoKind, InfoVrf, LinkAttribute, LinkInfo};
use tokio::runtime::Runtime;

pub struct RtnetlinkWrapper {
    rt: Runtime,
    handle: rtnetlink::Handle,
}

impl RtnetlinkWrapper {
    pub fn new() -> Self {
        let rt = Runtime::new().expect("Failed to create runtime");
        let (connection, handle, _) = rtnetlink::new_connection().unwrap();

        rt.spawn(connection);
        Self { rt, handle }
    }

    pub async fn get_interface_index(&self, name: String) -> Result<u32, String> {
        let mut links = self.handle.link().get().match_name(name).execute();
        let link = links.try_next();
        match link.await {
            Ok(Some(link)) => Ok(link.header.index),
            Ok(None) => Err("No link found on".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn add_bridge(&self, name: String) -> Result<(), String> {
        let request = self.handle.link().add().bridge(name);
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn add_veth(&self, name: String, peer: String) -> Result<(), String> {
        let request = self.handle.link().add().veth(name, peer);
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn add_vrf(&self, name: String, table_id: u32) -> Result<(), String> {
        let info = vec![InfoVrf::TableId(table_id)];

        let mut link_info: Vec<LinkInfo> = vec![];
        link_info.push(LinkInfo::Kind(InfoKind::Vrf));
        link_info.push(LinkInfo::Data(InfoData::Vrf(info)));

        let mut request = self.handle.link().add().name(name);
        request
            .message_mut()
            .attributes
            .push(LinkAttribute::LinkInfo(link_info));
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn set_interface_master(
        &self,
        interface: String,
        master: String,
    ) -> Result<(), String> {
        let request = self
            .handle
            .link()
            .set(self.get_interface_index(interface).await.unwrap())
            .controller(self.get_interface_index(master).await.unwrap());
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn set_interface_up(&self, name: String) -> Result<(), String> {
        let request = self
            .handle
            .link()
            .set(self.get_interface_index(name).await.unwrap())
            .up();
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn set_interface_down(&self, name: String) -> Result<(), String> {
        let request = self
            .handle
            .link()
            .set(self.get_interface_index(name).await.unwrap())
            .down();
        request.execute().await.map_err(|e| e.to_string())
    }

    pub async fn add_ip_address(
        &self,
        name: String,
        address: u32,
        subnet_length: u8,
    ) -> Result<(), String> {
        let request = self.handle.address().add(
            self.get_interface_index(name).await?,
            Ipv4Addr::from_bits(address).into(),
            subnet_length,
        );
        request.execute().await.map_err(|e| e.to_string())
    }
    pub async fn add_default_route(
        &self,
        destination: String,
        gateway: String,
        interface: String,
    ) -> Result<(), String> {
        let request = self.handle.route().add().v4();
        request.execute().await.map_err(|e| e.to_string())

    }

    pub async fn set_ns(&self, name: String, ns_fd:i32) -> Result<(), String> {
        let request = self
            .handle
            .link()
            .set(self.get_interface_index(name).await?)
            .setns_by_fd(ns_fd);
        request.execute().await.map_err(|e| e.to_string())
    }
}
