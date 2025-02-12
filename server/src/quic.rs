use crate::server::Server;
use quinn::{crypto::Session, Endpoint, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{error::Error, time::Duration};

use super::server::{AppSession, AsyncSessionHandler};

pub struct QuicServer {
    address: String,
}

impl QuicServer {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Server for QuicServer {
    async fn start(&self, session_handler: AsyncSessionHandler) -> io::Result<()> {
        let (endpoint, _server_cert) = make_server_endpoint(self.address.parse().unwrap()).unwrap();
        println!("QUIC server listening on {}", self.address);

        while let Some(conn) = endpoint.accept().await {
            let new_connection = conn.await?;
            println!("New QUIC connection: {:?}", new_connection.remote_address());

            tokio::spawn(async move {
                let (send, recv) = new_connection
                    .accept_bi()
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                if let Err(e) =
                    session_handler(AppSession::new(Box::new(recv), Box::new(send))).await
                {
                    eprintln!("Connection failed: {}", e);
                }
                Ok::<_, io::Error>(())
            });
        }
        Ok(())
    }
}

pub fn make_server_endpoint(
    bind_addr: SocketAddr,
) -> Result<(Endpoint, CertificateDer<'static>), Box<dyn Error + Send + Sync + 'static>> {
    let (server_config, server_cert) = configure_server()?;
    let endpoint = Endpoint::server(server_config, bind_addr)?;
    Ok((endpoint, server_cert))
}
/// Returns default server configuration along with its certificate.
fn configure_server(
) -> Result<(ServerConfig, CertificateDer<'static>), Box<dyn Error + Send + Sync + 'static>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = CertificateDer::from(cert.cert);
    let priv_key = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

    let mut server_config =
        ServerConfig::with_single_cert(vec![cert_der.clone()], priv_key.into())?;

    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());
    transport_config.keep_alive_interval(Some(Duration::from_secs(10)));

    Ok((server_config, cert_der))
}
