//! IPC client for connecting to egui applications
//!
//! This client handles features that require direct application access:
//! - Screenshots
//! - Coordinate-based input (clicks, drags)
//! - Keyboard input
//! - Scroll events
//!
//! Note: UI tree access and element-based interactions are handled via AT-SPI.

#![allow(dead_code)] // Methods will be used when MCP tools for input are added

use egui_mcp_protocol::{
    MouseButton, ProtocolError, Request, Response, default_socket_path, read_response,
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

    /// Click at specific coordinates
    pub async fn click_at(&self, x: f32, y: f32, button: MouseButton) -> Result<(), ProtocolError> {
        let response = self
            .send_request(&Request::ClickAt { x, y, button })
            .await?;
        match response {
            Response::Success => Ok(()),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Send keyboard input
    pub async fn keyboard_input(&self, key: &str) -> Result<(), ProtocolError> {
        let response = self
            .send_request(&Request::KeyboardInput {
                key: key.to_string(),
            })
            .await?;
        match response {
            Response::Success => Ok(()),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Scroll at specific coordinates
    pub async fn scroll(
        &self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<(), ProtocolError> {
        let response = self
            .send_request(&Request::Scroll {
                x,
                y,
                delta_x,
                delta_y,
            })
            .await?;
        match response {
            Response::Success => Ok(()),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Move mouse to specific coordinates
    pub async fn move_mouse(&self, x: f32, y: f32) -> Result<(), ProtocolError> {
        let response = self.send_request(&Request::MoveMouse { x, y }).await?;
        match response {
            Response::Success => Ok(()),
            Response::Error { message } => Err(ProtocolError::Io(std::io::Error::other(message))),
            _ => Err(ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected response",
            ))),
        }
    }

    /// Drag from one position to another
    pub async fn drag(
        &self,
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        button: MouseButton,
    ) -> Result<(), ProtocolError> {
        let response = self
            .send_request(&Request::Drag {
                start_x,
                start_y,
                end_x,
                end_y,
                button,
            })
            .await?;
        match response {
            Response::Success => Ok(()),
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
