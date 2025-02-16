pub mod freight_proto;
pub mod grpc;
pub mod quic;
pub mod rtlink;
pub mod server;
pub mod tcp;

use common::{
    protocol::{Frame, Protocol, TunnelCodec, USING_PROTOCOL},
    tun::TunInterface,
};
use futures::{FutureExt, SinkExt, StreamExt};
use grpc::create_grpc_server;
use packet::ether;
use quic::QuicServer;
use server::{AppSession, Server, SessionHandler};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tcp::TcpServer;
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct Network {
    pub network_id: String,
    pub tun: TunInterface,
}

pub struct NetworkManager {
    pub networks: Vec<Arc<Mutex<Network>>>,
}

impl NetworkManager {
    pub fn new() -> Self {
        NetworkManager {
            networks: Vec::new(),
        }
    }

    pub fn add_network(&mut self, network: Network) {
        self.networks.push(Arc::new(Mutex::new(network))); // Wrap network in Arc<Mutex>
    }

    pub fn get_network(&self, network_id: &str) -> Option<Arc<Mutex<Network>>> {
        self.networks.iter().find_map(|n| {
            let network = n.lock().unwrap();
            if network.network_id == network_id {
                Some(Arc::clone(n))
            } else {
                None
            }
        })
    }

    pub fn remove_network(&mut self, network_id: &str) -> Option<Arc<Mutex<Network>>> {
        if let Some(pos) = self
            .networks
            .iter()
            .position(|n| {
                let network = n.lock().unwrap();
                network.network_id == network_id
            })
        {
            Some(self.networks.remove(pos))
        } else {
            None
        }
    }
}

pub async fn create_server(
    protocol: Protocol,
    address: &str,
    session_handler: Box<dyn SessionHandler + Sync>,
) -> io::Result<Box<dyn Server>> {
    match protocol {
        Protocol::Tcp => Ok(Box::new(TcpServer::new(address, session_handler))),
        Protocol::Quic => Ok(Box::new(QuicServer::new(address, session_handler))),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protocol = USING_PROTOCOL;
    let network_manager = Arc::new(Mutex::new(NetworkManager::new()));
    let network_manager_clone = network_manager.clone();
    let server = create_server(protocol,
         "0.0.0.0:8080",
         Box::new(AppSessionHandler{ network_manager: network_manager.clone() }),
        ).await?;

    

    tokio::spawn(async move {
        server.start().await.unwrap();
    });

    create_grpc_server(network_manager_clone).await.unwrap();
    Ok(())
}

struct AppSessionHandler {
    network_manager: Arc<Mutex<NetworkManager>>,
}
#[async_trait::async_trait]
impl SessionHandler for AppSessionHandler {
    async fn handle_session(&self, session: AppSession) -> io::Result<()> {
        let (mut tun_sink, mut tun_stream) = {
            let manager = self.network_manager.lock().unwrap();
            if let Some(network) = manager.get_network("test-network") {
                network.lock().unwrap().tun.device.into_framed().split()
            } else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Network not found"));
            }
        };


        let (mut frame_read, mut frame_send) = (
            FramedRead::new(session.read, TunnelCodec),
            FramedWrite::new(session.write, TunnelCodec),
        );

        loop {
            tokio::select! {
                Some(quic_to_tun) = frame_read.next() => {
                    match quic_to_tun {
                        Ok(frame) => {
                            if let Frame::IPv4(data) = frame {
                                tun_sink.send(data).await
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                            }
                        }
                        Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
                    }
                }
                Some(Ok(pkt)) = tun_stream.next() => {
                    if let Ok(pkt) = ether::Packet::new(pkt) {
                        let frame = Frame::IPv4(pkt.as_ref().to_vec());
                        frame_send.send(frame).await
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                    }
                }
                else => break,
            }
        }
        Ok(())
    }
}

