use serde_derive::{Deserialize, Serialize};
use std::net::IpAddr;
use tokio::io::AsyncRead;
use tokio_stream::StreamExt;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder, FramedRead};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Frame {
    Ping,
    OK,
    StateChanged(SessionState),
    Request(RequestBody),
    Response(ResponseBody),
    /// IPv4 Packet
    #[serde(with = "serde_bytes")]
    IPv4(Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum RequestBody {
    AuthRequest { token: String },
    ClientIPRequest { network_id: String },
    ReadyRequest,
    CloseRequest,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ResponseBody {
    ClientIPResponse { ip: IpAddr },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SessionState {
    Init,
    Authenticated,
    Ready,
    Established,
    Closed,
}

type AppFramedRead = FramedRead<Box<dyn AsyncRead + Send + Unpin>, TunnelCodec>;

/// Read a frame from `framed_read` and expect it to be matched `expected`.
/// If the frame is matched, return the matched value.
/// Otherwise, return an error.
pub async fn expect_frame<T>(
    framed_read: &mut AppFramedRead,
    expected: fn(Frame) -> Option<T>,
) -> Result<T, String> {
    let result = framed_read
        .next()
        .await
        .ok_or("Error while reading frame")?
        .map_err(|e| e.to_string())?;
    let result = expected(result).ok_or("Unexpected frame")?;

    Ok(result)
}

pub struct TunnelCodec {
    length_delimited_codec: tokio_util::codec::LengthDelimitedCodec,
}

impl TunnelCodec {
    pub fn new() -> Self {
        Self {
            length_delimited_codec: tokio_util::codec::LengthDelimitedCodec::new(),
        }
    }
}

impl Encoder<Frame> for TunnelCodec {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = serde_cbor::to_vec(&item)?;

        self.length_delimited_codec
            .encode(Bytes::from(bytes), dst)?;
        Ok(())
    }
}

impl Decoder for TunnelCodec {
    type Item = Frame;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let result = self.length_delimited_codec.decode(src)?;
        match result {
            Some(data) => Ok(serde_cbor::from_slice(&data)?),
            None => Ok(None),
        }
    }
}

pub enum Protocol {
    Tcp,
    Quic,
}

pub const USING_PROTOCOL: Protocol = Protocol::Tcp;
