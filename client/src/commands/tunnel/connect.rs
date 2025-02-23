use std::{net::IpAddr, process::exit, str::FromStr};

use cidr::{Cidr, Ipv4Cidr};
use common::protocol::{
    expect_frame, Frame, RequestBody, ResponseBody, SessionState, TunnelCodec, USING_PROTOCOL,
};
use futures::{SinkExt, StreamExt};
use packet::ip;
use tokio_util::codec::{FramedRead, FramedWrite};
use tun::Configuration;

use crate::{
    api_client::get_api_config,
    commands::{Args, Command},
    nat::{NATTable, Network},
    transport::create_transport,
    Config,
};

#[derive(clap::Parser, Debug)]
pub struct Connect {
    full_network_name: String,
    bind_network_cidr: Ipv4Cidr,

    #[arg(short, long)]
    server: Option<String>,
}

impl Command for Connect {
    async fn run(&self, _args: &Args) {
        let config = Config::load();
        // Validate the network name
        let parts: Vec<&str> = self.full_network_name.split('/').collect();
        if parts.len() != 2 {
            eprintln!("Invalid network name format. Expected format: 'owner/name'");
            return;
        }
        let owner_user_name = parts[0];
        let network_name = parts[1];

        let server_name = self
            .server
            .clone()
            .unwrap_or(config.default.server.unwrap_or_else(|| {
                eprintln!("No server specified. Use --server or set a default server.");
                exit(1);
            }));
        let server_conf = config.servers.get(&server_name).unwrap_or_else(|| {
            eprintln!("Server {} not found in config.", server_name);
            exit(1);
        });
        let mut conf = get_api_config(server_conf.url.to_string());
        conf.bearer_access_token = Some(server_conf.token.clone().unwrap_or_else(|| {
            eprintln!("No token found for server {}.", server_name);
            exit(1);
        }));
        let network = openapi::apis::default_api::api_networks_owner_name_get(
            &conf,
            owner_user_name,
            network_name,
        )
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error GET Network: {:?}", e);
            exit(1);
        })
        .data;
        dbg!(&network);
        let network_cidr =
            Ipv4Cidr::from_str(&network.network_address_with_mask).unwrap_or_else(|e| {
                eprintln!("Error parsing CIDR: {:?}", e);
                exit(1);
            });

        if self.bind_network_cidr.network_length() != network_cidr.network_length() {
            eprintln!(
                "Network CIDR length mismatch. Expected: {}, Got: {}",
                network_cidr.network_length(),
                self.bind_network_cidr.network_length()
            );
            exit(1);
        }

        connect_tunnel(
            server_conf.token.clone().unwrap(),
            *network,
            self.bind_network_cidr,
            network_cidr,
        )
        .await
        .unwrap();
    }
}

async fn connect_tunnel(
    token: String,
    network: openapi::models::Network,
    local_cidr: Ipv4Cidr,
    network_cidr: Ipv4Cidr,
) -> Result<(), String> {
    let mut nat_table = NATTable::new();

    nat_table.add_entry(
        Network::new(local_cidr.first_address(), local_cidr.network_length()),
        Network::new(network_cidr.first_address(), network_cidr.network_length()),
    );

    let transport = create_transport(USING_PROTOCOL, "192.168.2.64:8080")
        .await
        .map_err(|e| e.to_string())?;
    let (recv, send) = transport.split();

    let mut framed_read = FramedRead::new(recv, TunnelCodec::new());
    let mut frame_send = FramedWrite::new(send, TunnelCodec::new());

    let mut state = SessionState::Init;

    frame_send
        .send(Frame::Request(RequestBody::AuthRequest { token }))
        .await
        .map_err(|e| e.to_string())?;

    expect_frame(&mut framed_read, |f| match f {
        Frame::StateChanged(SessionState::Authenticated) => Some(()),
        _ => None,
    })
    .await
    .map_err(|e| e.to_string())?;
    state = SessionState::Authenticated;
    dbg!(&network);
    frame_send
        .send(Frame::Request(RequestBody::ClientIPRequest {
            network_id: network.id.to_string(),
        }))
        .await
        .map_err(|e| e.to_string())?;

    let ip_addr = expect_frame(&mut framed_read, |f| match f {
        Frame::Response(ResponseBody::ClientIPResponse { ip }) => match ip {
            IpAddr::V4(ip) => Some(ip),
            _ => None,
        },
        _ => None,
    })
    .await?;
    println!("ip_addr: {:?}", ip_addr);
    let mut tun_conf = Configuration::default();
    tun_conf
        .tun_name("tun0")
        // システム上のアドレスをローカルのやつへ
        .address(ip_addr)
        .netmask(local_cidr.mask())
        .up();
    let dev = tun::create_as_async(&tun_conf).map_err(|e| e.to_string())?;
    let (mut tun_sink, mut tun_stream) = dev.into_framed().split();

    frame_send
        .send(Frame::Request(RequestBody::ReadyRequest))
        .await
        .map_err(|e| e.to_string())?;

    expect_frame(&mut framed_read, |f| match f {
        Frame::StateChanged(SessionState::Ready) => Some(()),
        _ => None,
    })
    .await?;
    state = SessionState::Ready;

    loop {
        tokio::select! {
            Some(from_quic) = framed_read.next()=>{
                match from_quic {
                    Ok(frame)=>{
                    match frame {
                        Frame::Ping => {
                            frame_send.send(Frame::OK).await.unwrap();
                        },
                        Frame::OK => {
                            // do nothing
                        },
                        Frame::StateChanged(new_state) => {
                            state = new_state;
                        },
                        Frame::IPv4(data) => match ip::Packet::new(data) {
                            Ok(ip::Packet::V4(mut pkt)) => {
                                println!("pkt: {:?}", pkt);
                                /* let pkt = pkt
                                    .set_destination(nat_table.reverse(pkt.destination()))
                                    .unwrap(); */
                                let pkt_bytes = pkt.as_ref();
                                tun_sink.send(pkt_bytes.to_vec()).await.unwrap();
                            }
                            _ => {
                                eprintln!("something wrong");
                            }
                        },
                        _=>{}
                    };
                    }
                    Err(e)=>{
                        eprintln!("error: {:#?}", e);
                    }
                }
            }
            Some(tun_to_quic) = tun_stream.next() => {
                let pkt: Vec<u8> = tun_to_quic.unwrap();
                match ip::Packet::new(pkt) {
                    Ok(ip::Packet::V4(mut pkt)) => {
                       /*  let pkt = pkt.set_destination(pkt.destination())
                            .map_err(|e| e.to_string())?; */

                        let pkt_bytes = pkt.as_ref();
                        let frame = Frame::IPv4(pkt_bytes.to_vec());
                        frame_send.send(frame).await.unwrap();
                    }
                    Ok(ip::Packet::V6(_pkt)) => {
                        //eprintln!("IPv6 packet is not supported");
                    }
                    Err(e) => {
                        eprintln!("error:{:#?}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
