//! Common protocol definitions for egui-mcp
//!
//! This crate defines the shared types and protocols used for communication
//! between the MCP server and egui client applications.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Default socket path for IPC communication
pub fn default_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    runtime_dir.join("egui-mcp.sock")
}

/// Information about a UI node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique identifier for the node
    pub id: u64,
    /// Role of the node (e.g., "Button", "TextInput", "Window")
    pub role: String,
    /// Human-readable label
    pub label: Option<String>,
    /// Current value (for inputs, sliders, etc.)
    pub value: Option<String>,
    /// Bounding rectangle
    pub bounds: Option<Rect>,
    /// Child node IDs
    pub children: Vec<u64>,
    /// Whether the node is toggled (for checkboxes, toggles)
    pub toggled: Option<bool>,
    /// Whether the node is disabled
    pub disabled: bool,
    /// Whether the node has focus
    pub focused: bool,
}

/// A rectangle in screen coordinates
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// UI tree containing all nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTree {
    /// Root node IDs
    pub roots: Vec<u64>,
    /// All nodes in the tree
    pub nodes: Vec<NodeInfo>,
}

impl Default for UiTree {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            nodes: Vec::new(),
        }
    }
}

/// Request types for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Ping the client
    Ping,
    /// Get the full UI tree
    GetUiTree,
}

/// Response types for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Pong response
    Pong,
    /// UI tree response
    UiTree(UiTree),
    /// Error response
    Error { message: String },
}

/// Protocol errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
}

/// Maximum message size (1 MB)
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Read a length-prefixed message from a reader
pub async fn read_message<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<Vec<u8>, ProtocolError> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Err(ProtocolError::ConnectionClosed);
        }
        Err(e) => return Err(e.into()),
    }

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::MessageTooLarge(len));
    }

    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write a length-prefixed message to a writer
pub async fn write_message<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> Result<(), ProtocolError> {
    if data.len() > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::MessageTooLarge(data.len()));
    }

    let len = (data.len() as u32).to_be_bytes();
    writer.write_all(&len).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

/// Read and deserialize a request
pub async fn read_request<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<Request, ProtocolError> {
    let data = read_message(reader).await?;
    let request = serde_json::from_slice(&data)?;
    Ok(request)
}

/// Write and serialize a response
pub async fn write_response<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    response: &Response,
) -> Result<(), ProtocolError> {
    let data = serde_json::to_vec(response)?;
    write_message(writer, &data).await
}

/// Read and deserialize a response
pub async fn read_response<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<Response, ProtocolError> {
    let data = read_message(reader).await?;
    let response = serde_json::from_slice(&data)?;
    Ok(response)
}

/// Write and serialize a request
pub async fn write_request<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    request: &Request,
) -> Result<(), ProtocolError> {
    let data = serde_json::to_vec(request)?;
    write_message(writer, &data).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_request() {
        let req = Request::Ping;
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Ping"));
    }

    #[test]
    fn test_serialize_response() {
        let resp = Response::Pong;
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Pong"));
    }

    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains("egui-mcp.sock"));
    }
}
