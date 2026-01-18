//! IPC client for connecting to egui applications

use egui_mcp_protocol::{
    ProtocolError, Request, Response, UiTree, default_socket_path, read_response, write_request,
};
use std::path::PathBuf;
use tokio::net::UnixStream;

/// IPC client for communicating with egui applications
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a new IPC client with default socket path
    pub fn new() -> Self {
        Self::with_socket_path(default_socket_path())
    }

    /// Create a new IPC client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Connect to the egui application and send a request
    async fn send_request(&self, request: &Request) -> Result<Response, ProtocolError> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (mut reader, mut writer) = stream.into_split();

        write_request(&mut writer, request).await?;
        let response = read_response(&mut reader).await?;

        Ok(response)
    }

    /// Ping the egui application
    pub async fn ping(&self) -> Result<bool, ProtocolError> {
        let response = self.send_request(&Request::Ping).await?;
        match response {
            Response::Pong => Ok(true),
            Response::Error { message } => {
                tracing::error!("Ping error: {}", message);
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// Get the UI tree from the egui application
    pub async fn get_ui_tree(&self) -> Result<UiTree, ProtocolError> {
        let response = self.send_request(&Request::GetUiTree).await?;
        match response {
            Response::UiTree(tree) => Ok(tree),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                message,
            ))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Check if the socket file exists (quick check without connecting)
    pub fn is_socket_available(&self) -> bool {
        self.socket_path.exists()
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}
