mod quic;
mod tcp;

use std::io;

use async_trait::async_trait;
use common::protocol::Protocol;
use quic::QuicTransport;
use tcp::TcpTransport;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait]
pub trait Transport: Send + Sync {
    fn split(
        self: Box<Self>,
    ) -> (
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncWrite + Send + Unpin>,
    );
}

pub async fn create_transport(protocol: Protocol, addr: &str) -> io::Result<Box<dyn Transport>> {
    match protocol {
        Protocol::Tcp => Ok(Box::new(TcpTransport::new(addr).await?)),
        Protocol::Quic => Ok(Box::new(QuicTransport::new(addr).await?)),
    }
}
