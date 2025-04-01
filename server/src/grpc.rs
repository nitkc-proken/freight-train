use crate::{
    freight_proto::{
        self,
        backend_client::BackendClient,
        gateway_server::{Gateway, GatewayServer},
    },
    rtlink::RtnetlinkWrapper,
    server::AppSession,
    Network, NetworkManager,
};
use common::protocol::Frame;
use futures::StreamExt;
use ipnet::{IpSub, Ipv4Net};
use netns_rs::{get_from_current_thread, get_from_path};
use packet::ip;
use std::{mem, net::Ipv4Addr};
use std::{net::SocketAddr, os::fd::AsRawFd, sync::Arc};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tun::{AsyncDevice, Configuration};

pub struct GatewayService {
    rtnetlink: RtnetlinkWrapper,
    network_manager: Arc<Mutex<NetworkManager>>,
    sessions: Arc<Mutex<Vec<Arc<AppSession>>>>,
    vrf_table_id_counter: Arc<Mutex<u32>>,
}

impl GatewayService {
    pub fn new(
        network_manager: Arc<Mutex<NetworkManager>>,
        sessions: Arc<Mutex<Vec<Arc<AppSession>>>>,
    ) -> Self {
        Self {
            rtnetlink: RtnetlinkWrapper::new(),
            network_manager,
            sessions,
            vrf_table_id_counter: Arc::new(Mutex::new(1000)),
        }
    }
}

#[tonic::async_trait]
impl Gateway for GatewayService {
    async fn clean_up_network(
        &self,
        request: tonic::Request<freight_proto::CleanNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::CleanNetworkResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        let network = self
            .network_manager
            .lock()
            .await
            .remove_network(&inner_request.network_id)
            .await
            .ok_or_else(|| tonic::Status::not_found("Network not found"))?;

        self.rtnetlink
            .delete_interface(inner_request.tun_interface_name)
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        self.rtnetlink
            .delete_interface(inner_request.bridge_interface_name)
            .await
            .map_err(|e| tonic::Status::internal(e))?;
        self.rtnetlink
            .delete_interface(inner_request.vrf_interface_name)
            .await
            .map_err(|e| tonic::Status::internal(e))?;

        Ok(tonic::Response::new(freight_proto::CleanNetworkResponse {
            network_id: inner_request.network_id,
        }))
    }
    async fn initiate_network(
        &self,
        request: tonic::Request<freight_proto::InitNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitNetworkResponse>, tonic::Status> {
        println!("Initiating network");
        dbg!(&request);
        let inner_request: freight_proto::InitNetworkRequest = request.into_inner();
        let container_network_cidr = Ipv4Net::new(
            inner_request.containers_network_address.into(),
            inner_request.containers_network_mask as u8,
        )
        .map_err(|e| tonic::Status::internal(e.to_string()))?;
        let client_network_cidr = Ipv4Net::new(
            inner_request.clients_network_address.into(),
            inner_request.clients_network_mask as u8,
        )
        .map_err(|e| tonic::Status::internal(e.to_string()))?;

        // ip link add {vrf_interface_name} type vrf table {vrf_route_table_id}
        let vrf_table_id;
        {
            let mut id = self.vrf_table_id_counter.lock().await;
            self.rtnetlink
                .add_vrf(inner_request.vrf_interface_name.clone(), *id)
                .await
                .map_err(|e| tonic::Status::internal(format!("Error Add VRF: {}", e)))?;
            vrf_table_id = *id;
            *id += 1;
        }
        // ip link set {vrf_interface_name} up
        self.rtnetlink
            .set_interface_up(inner_request.vrf_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(format!("Error Set VRF Interface Up: {}", e)))?;
        // ip link add {bridge_interface_name} type bridge
        self.rtnetlink
            .add_bridge(inner_request.bridge_interface_name.clone())
            .await
            .map_err(|e| tonic::Status::internal(format!("Error Add Bridge: {}", e)))?;
        self.rtnetlink
            .add_ip_address(
                inner_request.bridge_interface_name.clone(),
                container_network_cidr
                    .broadcast()
                    .saturating_sub(1)
                    .to_bits(),
                container_network_cidr.prefix_len(),
            )
            .await
            .map_err(|e| {
                tonic::Status::internal(format!("Error Add IP Address To Bridge: {}", e))
            })?;
        // ip link set {bridge_interface_name} master {vrf_interface_name}
        self.rtnetlink
            .set_interface_master(
                inner_request.bridge_interface_name.clone(),
                inner_request.vrf_interface_name.clone(),
            )
            .await
            .map_err(|e| {
                tonic::Status::internal(format!("Error Set Bridge Interface Master To VRF: {}", e))
            })?;
        // ip link set {bridge_interface_name} up
        self.rtnetlink
            .set_interface_up(inner_request.bridge_interface_name.clone())
            .await
            .map_err(|e| {
                tonic::Status::internal(format!("Error Set Bridge Interface Up: {}", e))
            })?;

        let mut tun_conf = Configuration::default();
        tun_conf
            .tun_name(inner_request.tun_interface_name.clone())
            .address(client_network_cidr.broadcast().saturating_sub(1))
            .netmask(client_network_cidr.netmask())
            .mtu(1200)
            .layer(tun::Layer::L3)
            .up();
        #[cfg(target_os = "linux")]
        tun_conf.platform_config(|config| {
            // requiring root privilege to acquire complete functions
            config.ensure_root_privileges(true);
        });
        let dev: AsyncDevice = tun::create_as_async(&tun_conf).unwrap();
        self.rtnetlink
            .set_interface_master(
                inner_request.tun_interface_name.clone(),
                inner_request.vrf_interface_name.clone(),
            )
            .await
            .map_err(|e| {
                tonic::Status::internal(format!("Error Set TUN Interface Master: {}", e))
            })?;

        let (sink, mut stream) = dev.into_framed().split();
        let sessions = self.sessions.clone();

        tokio::spawn(async move {
            while let Some(Ok(pkt)) = stream.next().await {
                match ip::Packet::new(pkt) {
                    Ok(ip::Packet::V4(pkt)) => {
                        let dest_ip = Ipv4Addr::from_bits(pkt.destination().to_bits());
                        let sessions_guard = sessions.lock().await;
                        for session in sessions_guard.iter() {
                            let session_data = session.session_data.lock().await;
                            if let Some(assigned_ip) = session_data.assigned_ip_addr {
                                println!("Assigned IP: {}", assigned_ip);
                                if assigned_ip == dest_ip {
                                    println!("Sending packet to session");
                                    if let Some(sender) = &session_data.frame_sender {
                                        println!("Sending packet to session");
                                        let frame = Frame::IPv4(pkt.as_ref().to_vec());
                                        if let Err(e) = sender.send(frame) {
                                            eprintln!("Failed to send packet to session: {}", e);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
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
            tun_sink: Arc::new(Mutex::new(sink)),
        };
        {
            self.network_manager.lock().await.add_network(network);
        }

        Ok(tonic::Response::new(freight_proto::InitNetworkResponse {
            network_id: inner_request.network_id,
            vrf_interface_name: inner_request.vrf_interface_name,
            bridge_interface_name: inner_request.bridge_interface_name,
            tun_interface_name: inner_request.tun_interface_name,
            vrf_route_table_id: vrf_table_id,
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

        let netns = get_from_path(inner_request.ns_path)
            .map_err(|e| tonic::Status::internal(e.to_string()))?;

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
            .map_err(|e| tonic::Status::internal(format!("Error setting namespace: {}", e)))?;
        {
            let src_ns = get_from_current_thread().map_err(|e| {
                tonic::Status::internal(format!("Error getting source namespace: {}", e))
            })?;
            netns
                .enter()
                .map_err(|e| tonic::Status::internal(format!("Error entering namespace: {}", e)))?;
            let rtnetlink = self.rtnetlink.child();
            rtnetlink
                .add_ip_address(
                    inner_request.veth_container_interface_name.clone(),
                    inner_request.ip_address,
                    inner_request.subnet_mask as u8,
                )
                .await
                .map_err(|e| {
                    tonic::Status::internal(format!(
                        "Error adding IP address to container interface: {}",
                        e
                    ))
                })?;
            rtnetlink
                .set_interface_up(inner_request.veth_container_interface_name.clone())
                .await
                .map_err(|e| {
                    tonic::Status::internal(format!("Error setting container interface up: {}", e))
                })?;
            rtnetlink
                .add_route(
                    inner_request.client_network_address,
                    inner_request.subnet_mask as u8,
                    inner_request.container_gateway_address,
                )
                .await
                .map_err(|e| tonic::Status::internal(format!("Error adding route: {}", e)))?;
            mem::forget(rtnetlink);
            src_ns.enter().map_err(|e| {
                tonic::Status::internal(format!("Error returning to source namespace: {}", e))
            })?;
        };

        Ok(tonic::Response::new(
            freight_proto::InitContainerNetworkResponse {
                network_id: inner_request.network_id,
                ip_address: inner_request.ip_address,
                subnet_mask: inner_request.subnet_mask,
            },
        ))
    }
}

pub async fn create_grpc_server(
    network_manager: Arc<Mutex<NetworkManager>>,
    sessions: Arc<Mutex<Vec<Arc<AppSession>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], 5051));
    let gateway_service = GatewayService::new(network_manager, sessions);
    tonic::transport::Server::builder()
        .add_service(GatewayServer::new(gateway_service))
        .serve(grpc_addr)
        .await?;

    Ok(())
}

pub async fn get_backend_client() -> BackendClient<Channel> {
    BackendClient::connect("http://192.168.2.4:5051")
        .await
        .unwrap()
}
