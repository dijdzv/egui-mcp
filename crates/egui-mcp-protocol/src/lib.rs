//! Common protocol definitions for egui-mcp
//!
//! This crate defines the shared types and protocols used for communication
//! between the MCP server and egui client applications.

use serde::{Deserialize, Serialize};

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
}
