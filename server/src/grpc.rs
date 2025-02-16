use crate::{
    freight_proto::{
        self,
        gateway_server::{Gateway, GatewayServer},
    },
    rtlink::RtnetlinkWrapper,
    Network, NetworkManager,
};
use common::{protocol::Frame, tun::TunInterface};
use futures::StreamExt;
use netns_rs::get_from_path;
use packet::{ether, ip};
use std::{
    net::SocketAddr,
    os::fd::AsRawFd,
    sync::{Arc, Mutex},
};

pub struct GatewayService {
    rtnetlink: RtnetlinkWrapper,
    network_manager: Arc<Mutex<NetworkManager>>,
}

impl GatewayService {
    pub fn new(network_manager: Arc<Mutex<NetworkManager>>) -> Self {
        Self {
            rtnetlink: RtnetlinkWrapper::new(),
            network_manager,
        }
    }
}

#[tonic::async_trait]
impl Gateway for GatewayService {
    async fn initiate_network(
        &self,
        request: tonic::Request<freight_proto::InitNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitNetworkResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        // ip link add {vrf_interface_name} type vrf table {vrf_route_table_id}
        self.rtnetlink
            .add_vrf(
                inner_request.vrf_interface_name.clone(),
                inner_request.vrf_route_table_id,
            )
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link set {vrf_interface_name} up
        self.rtnetlink
            .set_interface_up(inner_request.vrf_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link add {bridge_interface_name} type bridge
        self.rtnetlink
            .add_bridge(inner_request.bridge_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link set {bridge_interface_name} master {vrf_interface_name}
        self.rtnetlink
            .set_interface_master(
                inner_request.vrf_interface_name.clone(),
                inner_request.bridge_interface_name.clone(),
            )
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link set {bridge_interface_name} up
        self.rtnetlink
            .set_interface_up(inner_request.bridge_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(e))?;

        let tun = TunInterface::new("10.0.1.254".parse().unwrap(), "10.0.0.1".parse().unwrap());
        let (mut sink, mut stream) = tun.device.into_framed().split();
        tokio::spawn(async move {
            while let Some(Ok(pkt)) = stream.next().await {
                match ip::Packet::new(pkt) {
                    Ok(ip::Packet::V4(mut pkt)) => {
                        let destBits = pkt.destination().to_bits();
                        let frame = Frame::IPv4(pkt.as_ref().to_vec());
                    }
                    Err(e) => eprintln!("Failed to parse packet: {}", e),
                    _ => {
                        eprintln!("Unsupported packet type");
                    }
                }
            }
        });
        let network = Network {
            network_id: inner_request.network_id.clone(),
            tun: TunInterface::new("10.0.1.254".parse().unwrap(), "10.0.0.1".parse().unwrap()),
        };
        {
            self.network_manager.lock().unwrap().add_network(network);
        }

        Ok(tonic::Response::new(freight_proto::InitNetworkResponse {
            network_id: inner_request.network_id,
            vrf_interface_name: inner_request.vrf_interface_name,
            bridge_interface_name: inner_request.bridge_interface_name,
            tun_interface_name: inner_request.tun_interface_name,
            vrf_route_table_id: inner_request.vrf_route_table_id,
        }))
    }

    async fn force_close_tunnel_session(
        &self,
        _request: tonic::Request<freight_proto::InitNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitNetworkResponse>, tonic::Status> {
        todo!()
    }

    async fn initiate_container_network(
        &self,
        request: tonic::Request<freight_proto::InitContainerNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitContainerNetworkResponse>, tonic::Status> {
        let inner_request = request.into_inner();

        let netns_file = format!("/proc/{}/ns/net", inner_request.container_pid);
        let netns =
            get_from_path(netns_file).map_err(|e| tonic::Status::internal(e.to_string()))?;

        // ip link add {veth_interface_name} type veth peer name {veth_container_interface_name}
        self.rtnetlink
            .add_veth(
                inner_request.veth_interface_name.clone(),
                inner_request.veth_container_interface_name.clone(),
            )
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link set {veth_interface_name} master {bridge_interface_name}
        self.rtnetlink
            .set_interface_master(
                inner_request.veth_interface_name.clone(),
                inner_request.bridge_interface_name.clone(),
            )
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        // ip link set {veth_interface_name} up
        self.rtnetlink
            .set_interface_up(inner_request.veth_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(e))?;

        let fd = netns.file().as_raw_fd();
        // ip netns exec <netns> ip link set {veth_container_interface_name} netns {container_pid}
        self.rtnetlink
            .set_ns(inner_request.veth_container_interface_name.clone(), fd)
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        netns
            .run(|_| async {
                let rtnetlink = RtnetlinkWrapper::new();
                // ip link add {veth_peer_name} type veth peer name {veth_name}
                rtnetlink.add_ip_address(
                    inner_request.veth_container_interface_name.clone(),
                    inner_request.ip_address,
                    inner_request.subnet_mask as u8,
                ).await
            })
            .map_err(|e| tonic::Status::internal(e.to_string()))?
            .await;

        todo!()
    }
}

pub async fn create_grpc_server(
    network_manager: Arc<Mutex<NetworkManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], 50051));
    let gateway_service = GatewayService::new(network_manager);
    tonic::transport::Server::builder()
        .add_service(GatewayServer::new(gateway_service))
        .serve(grpc_addr)
        .await?;
    
    Ok(())
}
