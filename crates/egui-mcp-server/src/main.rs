//! MCP server for egui UI automation
//!
//! This server provides MCP tools for interacting with egui applications.

mod ipc_client;

use anyhow::Result;
use ipc_client::IpcClient;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use serde_json::json;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// egui-mcp server handler
#[derive(Clone)]
struct EguiMcpServer {
    tool_router: ToolRouter<Self>,
    ipc_client: Arc<IpcClient>,
}

impl EguiMcpServer {
    fn new() -> Self {
        let tool_router = Self::tool_router();
        let ipc_client = Arc::new(IpcClient::new());
        Self {
            tool_router,
            ipc_client,
        }
    }
}

#[tool_router]
impl EguiMcpServer {
    /// Ping the server to check if it's running
    #[tool(description = "Ping the egui-mcp server to verify it's running")]
    async fn ping(&self) -> String {
        "pong".to_string()
    }

    /// Check connection to the egui application
    #[tool(description = "Check if the egui application is connected and responding")]
    async fn check_connection(&self) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "connected": false,
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        match self.ipc_client.ping().await {
            Ok(true) => json!({
                "connected": true,
                "message": "egui application is connected and responding"
            })
            .to_string(),
            Ok(false) => json!({
                "connected": false,
                "message": "egui application did not respond correctly"
            })
            .to_string(),
            Err(e) => json!({
                "connected": false,
                "message": format!("Failed to connect: {}", e)
            })
            .to_string(),
        }
    }

    /// Get the UI tree from the connected egui application
    #[tool(description = "Get the full UI tree from the egui application as JSON")]
    async fn get_ui_tree(&self) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        match self.ipc_client.get_ui_tree().await {
            Ok(tree) => serde_json::to_string_pretty(&tree).unwrap_or_else(|e| {
                json!({
                    "error": "serialization_error",
                    "message": format!("Failed to serialize UI tree: {}", e)
                })
                .to_string()
            }),
            Err(e) => json!({
                "error": "connection_error",
                "message": format!("Failed to get UI tree: {}", e)
            })
            .to_string(),
        }
    }
}

#[tool_handler]
impl ServerHandler for EguiMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "egui-mcp server provides tools for UI automation of egui applications. \
                 Use 'ping' to verify the server is running, 'check_connection' to verify \
                 the egui app is connected, and 'get_ui_tree' to inspect the UI structure."
                    .into(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (stdout is used for MCP communication)
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Starting egui-mcp server...");

    // Create and run the server
    let server = EguiMcpServer::new();
    let service = server.serve(stdio()).await?;

    tracing::info!("Server started, waiting for connections...");
    service.waiting().await?;

    Ok(())
}
