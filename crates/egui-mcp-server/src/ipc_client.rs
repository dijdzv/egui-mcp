//! IPC client for connecting to egui applications

use egui_mcp_protocol::{
    NodeInfo, ProtocolError, Request, Response, UiTree, default_socket_path, read_response,
    write_request,
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
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Find UI elements by label text
    pub async fn find_by_label(
        &self,
        pattern: &str,
        exact: bool,
    ) -> Result<Vec<NodeInfo>, ProtocolError> {
        let response = self
            .send_request(&Request::FindByLabel {
                pattern: pattern.to_string(),
                exact,
            })
            .await?;
        match response {
            Response::Elements(elements) => Ok(elements),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Find UI elements by role
    pub async fn find_by_role(&self, role: &str) -> Result<Vec<NodeInfo>, ProtocolError> {
        let response = self
            .send_request(&Request::FindByRole {
                role: role.to_string(),
            })
            .await?;
        match response {
            Response::Elements(elements) => Ok(elements),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Get a specific UI element by ID
    pub async fn get_element(&self, id: u64) -> Result<Option<NodeInfo>, ProtocolError> {
        let response = self.send_request(&Request::GetElement { id }).await?;
        match response {
            Response::Element(element) => Ok(element),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Take a screenshot of the egui application
    /// Returns (base64_data, format)
    pub async fn take_screenshot(&self) -> Result<(String, String), ProtocolError> {
        let response = self.send_request(&Request::TakeScreenshot).await?;
        match response {
            Response::Screenshot { data, format } => Ok((data, format)),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
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
