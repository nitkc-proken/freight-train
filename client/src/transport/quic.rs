use crate::transport::Transport;
use quinn::crypto::rustls::QuicClientConfig;
use quinn::{ClientConfig, Endpoint};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::io;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct QuicTransport {
    connection: quinn::Connection,
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

impl QuicTransport {
    pub async fn new(server_addr: &str) -> io::Result<Self> {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
        let mut client_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(SkipServerVerification::new())
            .with_no_client_auth();
        client_config.key_log = Arc::new(rustls::KeyLogFile::new());
        //client_config.key_log = Arc::new(KeyLogFile::new());
        endpoint.set_default_client_config(ClientConfig::new(Arc::new(
            QuicClientConfig::try_from(client_config).unwrap(),
        )));

        let connection = endpoint
            .connect(server_addr.parse().unwrap(), "localhost")
            .unwrap()
            .await?;
        let (send, recv) = connection.open_bi().await?;
        println!("[client] connected: addr={}", connection.remote_address());
        Ok(Self {
            connection,
            recv,
            send,
        })
    }
}

#[async_trait::async_trait]
impl Transport for QuicTransport {
    fn split(
        self: Box<Self>,
    ) -> (
        Box<dyn AsyncRead + Send + Unpin>,
        Box<dyn AsyncWrite + Send + Unpin>,
    ) {
        let recv = self.recv;
        let send = self.send;

        (Box::new(recv), Box::new(send))
    }
}

/// Dummy certificate verifier that treats any certificate as valid.
/// NOTE, such verification is vulnerable to MITM attacks, but convenient for testing.
#[derive(Debug)]
struct SkipServerVerification(Arc<rustls::crypto::CryptoProvider>);

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self(Arc::new(rustls::crypto::ring::default_provider())))
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
