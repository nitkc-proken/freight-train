use std::{io, net::Ipv4Addr, sync::Arc};

use common::protocol::{Frame, SessionState};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};

#[async_trait::async_trait]
pub trait Server: Send + Sync {
    async fn start(&self) -> io::Result<()>;
}

/// コネクションを扱うためのセッション情報
pub struct AppSession {
    pub session_data: Arc<Mutex<SessionData>>,
}

impl AppSession {
    pub fn new() -> Self {
        Self {
            session_data: Arc::new(Mutex::new(SessionData::default())),
        }
    }
}

pub struct SessionData {
    pub state: SessionState,
    pub session_id: Option<String>,
    pub assigned_ip_addr: Option<Ipv4Addr>,
    pub frame_sender: Option<tokio::sync::mpsc::UnboundedSender<Frame>>,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            state: SessionState::Init,
            session_id: None,
            assigned_ip_addr: None,
            frame_sender: None,
        }
    }
}


#[async_trait::async_trait]
pub trait SessionHandler: Send + Sync {
    async fn add_session(&self, session: AppSession) -> Result<Arc<AppSession>, String>;
    async fn handle_session(
        &self,
        read: Box<dyn AsyncRead + Send + Unpin>,
        write: Box<dyn AsyncWrite + Send + Unpin>,
        session_data: Arc<Mutex<SessionData>>,
    ) -> Result<(), String>;
}
