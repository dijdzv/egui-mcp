//! Library to embed in egui apps for MCP integration
//!
//! This crate provides client-side integration for egui applications
//! to support MCP automation features that require direct application access:
//! - Screenshots
//! - Coordinate-based input (clicks, drags)
//! - Keyboard input
//! - Scroll events
//!
//! Note: UI tree access and element-based interactions are handled via AT-SPI
//! on the server side and don't require this client library.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use egui_mcp_protocol::{MouseButton, Request, Response};

mod server;

pub use server::IpcServer;

/// Pending input event to be processed by the egui application
#[derive(Debug, Clone)]
pub enum PendingInput {
    /// Click at coordinates
    Click { x: f32, y: f32, button: MouseButton },
    /// Move mouse to coordinates
    MoveMouse { x: f32, y: f32 },
    /// Keyboard input
    Keyboard { key: String },
    /// Scroll at coordinates
    Scroll {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    },
    /// Drag operation
    Drag {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        button: MouseButton,
    },
}

/// Shared state for the MCP client
#[derive(Clone)]
pub struct McpClient {
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    socket_path: PathBuf,
    /// Screenshot data (PNG encoded)
    screenshot_data: Option<Vec<u8>>,
    /// Flag to request a screenshot
    screenshot_requested: bool,
    /// Pending input events to be processed by the egui app
    pending_inputs: Vec<PendingInput>,
}

impl McpClient {
    /// Create a new MCP client with default socket path
    pub fn new() -> Self {
        Self::with_socket_path(egui_mcp_protocol::default_socket_path())
    }

    /// Create a new MCP client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClientState {
                socket_path,
                screenshot_data: None,
                screenshot_requested: false,
                pending_inputs: Vec::new(),
            })),
        }
    }

    /// Get the socket path
    pub async fn socket_path(&self) -> PathBuf {
        self.state.read().await.socket_path.clone()
    }

    // Screenshot methods

    /// Set screenshot data (PNG encoded)
    pub async fn set_screenshot(&self, data: Vec<u8>) {
        self.state.write().await.screenshot_data = Some(data);
    }

    /// Get screenshot data (PNG encoded)
    pub async fn get_screenshot(&self) -> Option<Vec<u8>> {
        self.state.read().await.screenshot_data.clone()
    }

    /// Clear screenshot data
    pub async fn clear_screenshot(&self) {
        self.state.write().await.screenshot_data = None;
    }

    /// Request a screenshot (sets flag for the UI to capture)
    pub async fn request_screenshot(&self) {
        self.state.write().await.screenshot_requested = true;
    }

    /// Check if screenshot is requested and clear the flag
    pub async fn take_screenshot_request(&self) -> bool {
        let mut state = self.state.write().await;
        let requested = state.screenshot_requested;
        state.screenshot_requested = false;
        requested
    }

    // Input methods

    /// Queue an input event to be processed by the egui app
    pub async fn queue_input(&self, input: PendingInput) {
        self.state.write().await.pending_inputs.push(input);
    }

    /// Take all pending input events (clears the queue)
    pub async fn take_pending_inputs(&self) -> Vec<PendingInput> {
        std::mem::take(&mut self.state.write().await.pending_inputs)
    }

    /// Start the IPC server in a background task
    pub fn start_server(&self) -> tokio::task::JoinHandle<()> {
        let client = self.clone();
        tokio::spawn(async move {
            if let Err(e) = IpcServer::run(client).await {
                tracing::error!("IPC server error: {}", e);
            }
        })
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}
