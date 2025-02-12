use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;
use tokio_util::bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use tun::{Reader, Writer};

#[derive(Serialize, Deserialize)]
struct Capsule {
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Frame {
    Hello,
    #[serde(with = "serde_bytes")]
    IPv4(Vec<u8>),
}

pub struct TunnelCodec;

impl Encoder<Frame> for TunnelCodec {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = serde_cbor::to_vec(&item)?;
        dst.put(bytes.as_slice());
        Ok(())
    }
}

impl Decoder for TunnelCodec {
    type Item = Frame;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let item = serde_cbor::from_slice(&src)?;
        Ok(item)
    }
}

pub enum Protocol {
    Tcp,
    Quic,
}

pub const USING_PROTOCOL: Protocol = Protocol::Quic;

pub async fn tun_to_udp(tun: &mut Reader, udp: &UdpSocket, peer_addr: &Option<SocketAddr>) {
    let mut buffer = [0u8; 1500];
    loop {
        if let None = peer_addr {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            continue;
        }
        match tun.read(&mut buffer) {
            Ok(n) => {
                info!("read {} bytes from TUN", n);
                let capsule = Capsule {
                    data: buffer[..n].to_vec(),
                };
                let serialized_data = bincode::serialize(&capsule).unwrap();
                udp.send_to(&serialized_data, peer_addr.unwrap())
                    .await
                    .unwrap();
            }
            Err(e) => {
                error!("TUN read error: {}", e);
            }
        }
    }
}

pub async fn udp_to_tun(
    tun: &mut Writer,
    udp: &UdpSocket,
    peer_addr: Option<&mut Option<SocketAddr>>,
) {
    let mut buffer = [0u8; 1500];
    loop {
        match udp.recv_from(&mut buffer).await {
            Ok((n, addr)) => {
                if let Some(&mut ref mut a) = peer_addr {
                    *(a) = Some(addr)
                }
                let capsule: Capsule = bincode::deserialize(&buffer[..n]).unwrap();
                info!("read {} bytes from UDP", n);
                tun.write_all(capsule.data.as_slice()).unwrap();
                tun.flush().unwrap();
            }
            Err(e) => {
                error!("UDP read error: {}", e);
            }
        }
    }
}
