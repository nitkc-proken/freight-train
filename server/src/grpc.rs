pub mod freight_proto {
    tonic::include_proto!("gateway");
}

use freight_proto::gateway_server::{Gateway, GatewayServer};

pub struct GatewayService {}

#[tonic::async_trait]
impl Gateway for GatewayService {
    async fn initiate_network_request(
        &self,
        request: tonic::Request<freight_proto::InitNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitNetworkResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        Ok(tonic::Response::new(freight_proto::InitNetworkResponse {
            network: inner_request.network,
            tun_interface_name: "tun0".to_string(),
            vrf_interface_name: "vrf0".to_string(),
        }))
    }

    async fn force_close_tunnel_session(
        &self,
        request: tonic::Request<freight_proto::InitNetworkRequest>,
    ) -> Result<tonic::Response<freight_proto::InitNetworkResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        Ok(tonic::Response::new(freight_proto::InitNetworkResponse {
            network: inner_request.network,
            tun_interface_name: "tun0".to_string(),
            vrf_interface_name: "vrf0".to_string(),
        }))
    }
}