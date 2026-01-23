//! Common protocol definitions for egui-mcp
//!
//! This crate defines the shared types and protocols used for IPC communication
//! between the MCP server and egui client applications.
//!
//! Note: UI tree access, element search, and click/text input operations are
//! handled via AT-SPI on Linux. This protocol is only used for features that
//! require direct client integration (screenshots, coordinate-based input, etc.).

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

/// Information about a UI node (used for AT-SPI responses)
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

/// UI tree containing all nodes (used for AT-SPI responses)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiTree {
    /// Root node IDs
    pub roots: Vec<u64>,
    /// All nodes in the tree
    pub nodes: Vec<NodeInfo>,
}

/// Mouse button for click operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Request types for IPC communication
///
/// These are operations that require direct client integration and cannot be
/// performed via AT-SPI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Ping the client to check connection
    Ping,

    /// Request a screenshot of the application window
    TakeScreenshot,

    /// Click at specific screen coordinates
    ClickAt {
        /// X coordinate (relative to window)
        x: f32,
        /// Y coordinate (relative to window)
        y: f32,
        /// Mouse button to click
        button: MouseButton,
    },

    /// Send keyboard input
    KeyboardInput {
        /// Key to press (e.g., "Enter", "Tab", "a", "Ctrl+C")
        key: String,
    },

    /// Scroll at specific coordinates
    Scroll {
        /// X coordinate (relative to window)
        x: f32,
        /// Y coordinate (relative to window)
        y: f32,
        /// Horizontal scroll delta
        delta_x: f32,
        /// Vertical scroll delta
        delta_y: f32,
    },

    /// Move mouse to specific coordinates (for hover effects)
    MoveMouse {
        /// X coordinate (relative to window)
        x: f32,
        /// Y coordinate (relative to window)
        y: f32,
    },

    /// Drag from one position to another
    Drag {
        /// Start X coordinate
        start_x: f32,
        /// Start Y coordinate
        start_y: f32,
        /// End X coordinate
        end_x: f32,
        /// End Y coordinate
        end_y: f32,
        /// Mouse button to use
        button: MouseButton,
    },

    /// Double click at specific screen coordinates
    DoubleClick {
        /// X coordinate (relative to window)
        x: f32,
        /// Y coordinate (relative to window)
        y: f32,
        /// Mouse button to click
        button: MouseButton,
    },
}

/// Response types for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Pong response to Ping
    Pong,

    /// Screenshot response
    Screenshot {
        /// Base64 encoded PNG data
        data: String,
        /// Image format (always "png")
        format: String,
    },

    /// Success response (for operations without data)
    Success,

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

    #[test]
    fn test_click_at_request() {
        let req = Request::ClickAt {
            x: 100.0,
            y: 200.0,
            button: MouseButton::Left,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("ClickAt"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_keyboard_input_request() {
        let req = Request::KeyboardInput {
            key: "Enter".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("KeyboardInput"));
        assert!(json.contains("Enter"));
    }
}
