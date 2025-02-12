
pub mod server;
pub mod quic;
pub mod grpc;


use std::net::SocketAddr;

pub async fn create_server(protocol: Protocol, address: &str) -> io::Result<Box<dyn Server>> {
    match protocol {
        Protocol::Tcp => Ok(Box::new(TcpServer::new(address))),
        Protocol::Quic => Ok(Box::new(QuicServer::new(address))),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], 9090));
    let gateway_service = GatewayService {};
    tonic::transport::Server::builder()
        .add_service(GatewayServer::new(gateway_service))
        .serve(grpc_addr)
        .await?;
    Ok(())
}
