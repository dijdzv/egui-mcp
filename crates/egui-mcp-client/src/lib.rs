//! Library to embed in egui apps for MCP integration
//!
//! This crate provides the client-side integration for egui applications
//! to expose their UI tree via MCP.

pub use egui_mcp_protocol::{NodeInfo, Rect, Request, Response, UiTree};

/// Placeholder for future client implementation
pub struct McpClient {
    // Will contain IPC connection, state, etc.
}

impl McpClient {
    /// Create a new MCP client
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}
