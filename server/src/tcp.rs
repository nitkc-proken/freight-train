use crate::server::{Server, SessionHandler};
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::server::{AppSession};

pub struct TcpServer {
    address: String,
    session_handler: Arc<Box<dyn SessionHandler + Sync>>,
}

impl TcpServer {
    pub fn new(address: &str, session_handler: Box<dyn SessionHandler + Sync>) -> Self {
        Self {
            address: address.to_string(),
            session_handler: Arc::new(session_handler),
        }
    }
}

#[async_trait::async_trait]
impl Server for TcpServer {
    async fn start(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("TCP server listening on {}", self.address);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("New connection from {}", addr);
            // Handle each client in a separate task
            
            let session_handler = self.session_handler.clone();
            tokio::spawn(async move {
                let (read, write) = stream.into_split();
                let session = AppSession::new(Box::new(read), Box::new(write));
                if let Err(e) = session_handler.handle_session(session).await {
                    eprintln!("Failed to handle client: {}", e);
                }
            });
        }
    }
}
