use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use crate::transport::Transport;
use async_trait::async_trait;
use std::io;

pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub async fn new(address: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(address).await?;
        Ok(Self { stream })
    }
}
#[async_trait]
impl Transport for TcpTransport {
    fn split(
        self: Box<Self>,
    ) -> (
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) {
        let (recv, send) = self.stream.into_split();
        (Box::new(recv), Box::new(send))
    }
}
