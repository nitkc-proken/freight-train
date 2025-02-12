use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

// L3をカプセル化するプロトコルの定義
pub trait TransportTunnelClientBackend {
    async fn connect(connect_config: ClientConnectionConfig) -> Self;
    async fn close_session(self);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientConnectionConfig {
    pub server_addr: SocketAddr,
    pub server_name: String,
}

pub trait TransportTunnelServerBackend {
    async fn serve(listen_addr: SocketAddr) -> Self;
    async fn accept(&self) -> Result<impl TransportTunnelSessionBackend, ()>;
    async fn shutdown(self);
}

pub trait TransportTunnelSessionBackend {
    async fn close_session(self);
    /*    async fn pipe_write(&mut self, write: impl AsyncWrite);
    async fn pipe_read(&mut self, read: impl AsyncRead);*/
}
