//! Library to embed in egui apps for MCP integration
//!
//! This crate provides the client-side integration for egui applications
//! to expose their UI tree via MCP.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use egui_mcp_protocol::{NodeInfo, Rect, Request, Response, UiTree};

mod server;
mod tree;

pub use server::IpcServer;
pub use tree::UiTreeBuilder;

/// Shared state for the MCP client
#[derive(Clone)]
pub struct McpClient {
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    ui_tree: UiTree,
    socket_path: PathBuf,
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
                ui_tree: UiTree::default(),
                socket_path,
            })),
        }
    }

    /// Get the socket path
    pub async fn socket_path(&self) -> PathBuf {
        self.state.read().await.socket_path.clone()
    }

    /// Update the UI tree
    pub async fn update_ui_tree(&self, tree: UiTree) {
        self.state.write().await.ui_tree = tree;
    }

    /// Get the current UI tree
    pub async fn get_ui_tree(&self) -> UiTree {
        self.state.read().await.ui_tree.clone()
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
