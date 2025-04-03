pub mod freight_proto;
pub mod grpc;
pub mod quic;
pub mod rtlink;
pub mod server;
pub mod tcp;
mod config;

use common::protocol::{
    expect_frame, Frame, Protocol, RequestBody, ResponseBody, SessionState, TunnelCodec,
    USING_PROTOCOL,
};
use config::SERVER_CONFIG;
use freight_proto::{
    backend_client::BackendClient, AuthenticationToken, StartTunnelingSessionRequest,
};
use futures::{stream::SplitSink, FutureExt, SinkExt, StreamExt};
use grpc::{create_grpc_server, get_backend_client};
use quic::QuicServer;
use server::{AppSession, Server, SessionData, SessionHandler};
use std::{io, net::Ipv4Addr, sync::Arc};
use tcp::TcpServer;
use tokio::sync::Mutex;
use tokio_util::codec::{Framed, FramedRead, FramedWrite};
use tonic::transport::Channel;
use tun::{AsyncDevice, TunPacketCodec};

pub struct Network {
    pub network_id: String,
    pub tun_sink: Arc<Mutex<SplitSink<Framed<AsyncDevice, TunPacketCodec>, Vec<u8>>>>,
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

    pub async fn get_network(&self, network_id: &str) -> Option<Arc<Mutex<Network>>> {
        for network in &self.networks {
            if network.lock().await.network_id == network_id {
                return Some(network.clone());
            }
        }
        None
    }

    pub async fn remove_network(&mut self, network_id: &str) -> Option<Arc<Mutex<Network>>> {
        if let Some(pos) = self.networks.iter().position(|n| {
            async move {
                let network = n.lock().await;
                network.network_id == network_id
            }
            .now_or_never()
            .unwrap()
        }) {
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
    let network_manager = Arc::new(Mutex::new(NetworkManager::new()));
    let network_manager_clone = network_manager.clone();
    let sessions: Arc<Mutex<Vec<Arc<AppSession>>>> = Arc::new(Mutex::new(vec![])); // Corrected type from AppSession to Vec<Arc<Mutex<AppSession>>>
    println!("Starting server...");
    println!("Server config: \n{:#?}", SERVER_CONFIG.clone());
    let tunnel_config = SERVER_CONFIG.tunnel.clone();
    let server = create_server(
        tunnel_config.protocol,
        format!("{}:{}", tunnel_config.host, tunnel_config.port).as_str(),
        Box::new(AppSessionHandler {
            network_manager: network_manager.clone(),
            grpc_client: Arc::new(Mutex::new(get_backend_client().await)),
            sessions: sessions.clone(),
        }),
    )
    .await?;
    println!("Server starting...");
    tokio::spawn(async move {
        server.start().await.unwrap();
    });
    println!("GRPC server starting...");

    create_grpc_server(network_manager_clone, sessions.clone())
        .await
        .unwrap();
    Ok(())
}

struct AppSessionHandler {
    network_manager: Arc<Mutex<NetworkManager>>,
    grpc_client: Arc<Mutex<BackendClient<Channel>>>,
    sessions: Arc<Mutex<Vec<Arc<AppSession>>>>,
}

#[async_trait::async_trait]
impl SessionHandler for AppSessionHandler {
    async fn add_session(&self, session: AppSession) -> Result<Arc<AppSession>, String> {
        let mut sessions_guard = self.sessions.lock().await;
        /* let session = Arc::new(Mutex::new(session)); */
        let session = Arc::new(session);
        sessions_guard.push(session.clone());
        Ok(session.clone())
    }
    async fn handle_session(
        &self,
        read: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        write: Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
        session_data: Arc<Mutex<SessionData>>,
    ) -> Result<(), String> {
        let (mut framed_read, mut frame_send) = (
            FramedRead::new(read, TunnelCodec::new()),
            FramedWrite::new(write, TunnelCodec::new()),
        );
        // Create channel for sending frames to this session
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let mut session_mut = session_data.lock().await;
            session_mut.frame_sender = Some(tx);
        }
        let token = expect_frame(&mut framed_read, |f| match f {
            Frame::Request(RequestBody::AuthRequest { token }) => Some(token.clone()),
            _ => None,
        })
        .await?;

        let result = self
            .grpc_client
            .lock()
            .await
            .validate_authentication_token(AuthenticationToken {
                token: token.clone(),
            })
            .await
            .map_err(|e| e.to_string())?;
        let user_id = result.into_inner().user_id;
        frame_send
            .send(Frame::StateChanged(SessionState::Authenticated))
            .await
            .map_err(|e| e.to_string())?;
        {
            session_data.lock().await.state = SessionState::Authenticated;
        }

        let network_id = expect_frame(&mut framed_read, |f| match f {
            Frame::Request(RequestBody::ClientIPRequest { network_id }) => Some(network_id.clone()),
            _ => None,
        })
        .await?;
        let session = self
            .grpc_client
            .lock()
            .await
            .start_tunnel_session(StartTunnelingSessionRequest {
                user_id,
                network_id: network_id.clone(),
            })
            .await
            .map_err(|e| e.to_string())?
            .into_inner();
        {
            let mut session_mut = session_data.lock().await;
            session_mut.session_id = Some(session.session_id);
            session_mut.assigned_ip_addr =
                Some(Ipv4Addr::from_bits(session.client_ip_address as u32));
        }
        frame_send
            .send(Frame::Response(ResponseBody::ClientIPResponse {
                ip: std::net::IpAddr::V4(Ipv4Addr::from_bits(session.client_ip_address as u32)),
            }))
            .await
            .map_err(|e| e.to_string())?;

        expect_frame(&mut framed_read, |f| match f {
            Frame::Request(RequestBody::ReadyRequest) => Some(()),
            _ => None,
        })
        .await?;

        let tun_sink = {
            let manager = self.network_manager.lock().await;
            if let Some(network) = manager.get_network(network_id.as_str()).await {
                let nw = network.lock().await;
                nw.tun_sink.clone()
            } else {
                return Err("Network not found".to_string());
            }
        };

        frame_send
            .send(Frame::StateChanged(SessionState::Ready))
            .await
            .map_err(|e| e.to_string())?;
        {
            let mut session_data_lock = session_data.lock().await;
            session_data_lock.state = SessionState::Ready;
        }
        loop {
            let mut tun_sink = tun_sink.lock().await;
            tokio::select! {
                Some(from_quic) = framed_read.next() => {
                    match from_quic {
                        Ok(frame) => {
                            match frame {
                                Frame::Ping => {
                                    frame_send.send(Frame::OK).await
                                        .map_err(|e| e.to_string())?;
                                },
                                Frame::IPv4(items) => {
                                    tun_sink.send(items).await
                                        .map_err(|e| e.to_string())?;
                                },
                                _ => (),
                            }
                        }
                        Err(e) => return Err(e.to_string()),
                    }
                },
                Some(frame) = rx.recv() => {
                    println!("Received frame from channel");
                    frame_send.send(frame).await
                        .map_err(|e| e.to_string())?;
                },
                else => break,
            }
        }
        Ok(())
    }
}
