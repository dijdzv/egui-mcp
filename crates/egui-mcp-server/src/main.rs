//! MCP server for egui UI automation
//!
//! This server provides MCP tools for interacting with egui applications.
//! Architecture:
//! - AT-SPI (Linux accessibility): UI tree, element search, clicks, text input
//! - IPC (direct client): Screenshots, coordinate-based input, keyboard, scroll

mod ipc_client;

#[cfg(target_os = "linux")]
mod atspi_client;

use anyhow::Result;
use ipc_client::IpcClient;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Request for find_by_label tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FindByLabelRequest {
    #[schemars(description = "Pattern to match against labels (substring match)")]
    pattern: String,
}

/// Request for find_by_label_exact tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FindByLabelExactRequest {
    #[schemars(description = "Exact label text to match")]
    pattern: String,
}

/// Request for find_by_role tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FindByRoleRequest {
    #[schemars(
        description = "Role to search for (e.g., 'Button', 'TextInput', 'CheckBox', 'Label')"
    )]
    role: String,
}

/// Request for get_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetElementRequest {
    #[schemars(description = "Node ID to retrieve (as string)")]
    id: String,
}

/// Request for click_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ClickElementRequest {
    #[schemars(description = "Node ID of the element to click (as string)")]
    id: String,
}

/// Request for set_text tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetTextRequest {
    #[schemars(description = "Node ID of the text input element (as string)")]
    id: String,
    #[schemars(description = "Text content to set")]
    text: String,
}

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

    /// Get the UI tree from the connected egui application via AT-SPI
    #[tool(description = "Get the full UI tree from the egui application as JSON")]
    async fn get_ui_tree(&self) -> String {
        #[cfg(target_os = "linux")]
        {
            match atspi_client::get_ui_tree_blocking("demo") {
                Ok(Some(tree)) => {
                    return serde_json::to_string_pretty(&tree).unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize UI tree: {}", e)
                        })
                        .to_string()
                    });
                }
                Ok(None) => {
                    tracing::info!("AT-SPI did not find any matching application");
                }
                Err(e) => {
                    tracing::warn!("AT-SPI failed: {}", e);
                }
            }
        }

        json!({
            "error": "not_available",
            "message": "UI tree access requires AT-SPI on Linux. Make sure the egui app is running."
        })
        .to_string()
    }

    /// Find UI elements by their label text (substring match)
    #[tool(description = "Find UI elements by their label text (substring match)")]
    async fn find_by_label(
        &self,
        Parameters(FindByLabelRequest { pattern }): Parameters<FindByLabelRequest>,
    ) -> String {
        #[cfg(target_os = "linux")]
        {
            match atspi_client::find_by_label_blocking("demo", &pattern, false) {
                Ok(elements) => {
                    return serde_json::to_string_pretty(&json!({
                        "count": elements.len(),
                        "elements": elements
                    }))
                    .unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize elements: {}", e)
                        })
                        .to_string()
                    });
                }
                Err(e) => {
                    tracing::warn!("AT-SPI find_by_label failed: {}", e);
                }
            }
        }

        let _ = pattern; // suppress unused warning on non-Linux
        json!({
            "error": "not_available",
            "message": "Element search requires AT-SPI on Linux."
        })
        .to_string()
    }

    /// Find UI elements by their label text (exact match)
    #[tool(description = "Find UI elements by their label text (exact match)")]
    async fn find_by_label_exact(
        &self,
        Parameters(FindByLabelExactRequest { pattern }): Parameters<FindByLabelExactRequest>,
    ) -> String {
        #[cfg(target_os = "linux")]
        {
            match atspi_client::find_by_label_blocking("demo", &pattern, true) {
                Ok(elements) => {
                    return serde_json::to_string_pretty(&json!({
                        "count": elements.len(),
                        "elements": elements
                    }))
                    .unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize elements: {}", e)
                        })
                        .to_string()
                    });
                }
                Err(e) => {
                    tracing::warn!("AT-SPI find_by_label_exact failed: {}", e);
                }
            }
        }

        let _ = pattern;
        json!({
            "error": "not_available",
            "message": "Element search requires AT-SPI on Linux."
        })
        .to_string()
    }

    /// Find UI elements by their role
    #[tool(
        description = "Find UI elements by their role (e.g., 'Button', 'TextInput', 'CheckBox', 'Label')"
    )]
    async fn find_by_role(
        &self,
        Parameters(FindByRoleRequest { role }): Parameters<FindByRoleRequest>,
    ) -> String {
        #[cfg(target_os = "linux")]
        {
            match atspi_client::find_by_role_blocking("demo", &role) {
                Ok(elements) => {
                    return serde_json::to_string_pretty(&json!({
                        "count": elements.len(),
                        "elements": elements
                    }))
                    .unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize elements: {}", e)
                        })
                        .to_string()
                    });
                }
                Err(e) => {
                    tracing::warn!("AT-SPI find_by_role failed: {}", e);
                }
            }
        }

        let _ = role;
        json!({
            "error": "not_available",
            "message": "Element search requires AT-SPI on Linux."
        })
        .to_string()
    }

    /// Get detailed information about a specific UI element by ID
    #[tool(
        description = "Get detailed information about a specific UI element by its ID (as string)"
    )]
    async fn get_element(
        &self,
        Parameters(GetElementRequest { id }): Parameters<GetElementRequest>,
    ) -> String {
        let id: u64 = match id.parse() {
            Ok(id) => id,
            Err(_) => {
                return json!({
                    "error": "invalid_id",
                    "message": "ID must be a valid unsigned integer"
                })
                .to_string();
            }
        };

        #[cfg(target_os = "linux")]
        {
            match atspi_client::get_element_blocking("demo", id) {
                Ok(Some(element)) => {
                    return serde_json::to_string_pretty(&element).unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize element: {}", e)
                        })
                        .to_string()
                    });
                }
                Ok(None) => {
                    return json!({
                        "error": "not_found",
                        "message": format!("No element found with id {}", id)
                    })
                    .to_string();
                }
                Err(e) => {
                    tracing::warn!("AT-SPI get_element failed: {}", e);
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        let _ = id;

        json!({
            "error": "not_available",
            "message": "Element access requires AT-SPI on Linux."
        })
        .to_string()
    }

    /// Click an element by ID using AT-SPI Action interface
    #[tool(description = "Click a UI element by its ID (as string). Uses AT-SPI Action interface.")]
    async fn click_element(
        &self,
        Parameters(ClickElementRequest { id }): Parameters<ClickElementRequest>,
    ) -> String {
        let id: u64 = match id.parse() {
            Ok(id) => id,
            Err(_) => {
                return json!({
                    "error": "invalid_id",
                    "message": "ID must be a valid unsigned integer"
                })
                .to_string();
            }
        };

        #[cfg(target_os = "linux")]
        {
            match atspi_client::click_element_blocking("demo", id) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Clicked element with id {}", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Click action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "click_failed",
                    "message": format!("Failed to click element: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "Click action requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Set text content of a text input element
    #[tool(
        description = "Set text content of a text input element by its ID (as string). Uses AT-SPI EditableText interface."
    )]
    async fn set_text(
        &self,
        Parameters(SetTextRequest { id, text }): Parameters<SetTextRequest>,
    ) -> String {
        let id: u64 = match id.parse() {
            Ok(id) => id,
            Err(_) => {
                return json!({
                    "error": "invalid_id",
                    "message": "ID must be a valid unsigned integer"
                })
                .to_string();
            }
        };

        #[cfg(target_os = "linux")]
        {
            match atspi_client::set_text_blocking("demo", id, &text) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Set text on element with id {}", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Set text action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "set_text_failed",
                    "message": format!("Failed to set text: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            let _ = text;
            json!({
                "error": "not_available",
                "message": "Set text requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Take a screenshot of the egui application
    #[tool(
        description = "Take a screenshot of the egui application. Returns base64-encoded PNG image data."
    )]
    async fn take_screenshot(&self) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        match self.ipc_client.take_screenshot().await {
            Ok((data, format)) => json!({
                "format": format,
                "data": data,
                "encoding": "base64"
            })
            .to_string(),
            Err(e) => json!({
                "error": "screenshot_error",
                "message": format!("Failed to take screenshot: {}", e)
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
                 the egui app is connected, 'get_ui_tree' to inspect the full UI structure, \
                 'find_by_label' for substring search, 'find_by_label_exact' for exact match, \
                 'find_by_role' to search by role (e.g., Button, TextInput), \
                 'get_element' to get details by ID (pass ID as string), \
                 'click_element' to click an element by ID, \
                 'set_text' to input text into a text field by ID, and \
                 'take_screenshot' to capture the current UI."
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

    // Enable session accessibility on Linux
    // This tells accessible applications (like egui with AccessKit) that an AT client is present
    #[cfg(target_os = "linux")]
    {
        match atspi_connection::set_session_accessibility(true).await {
            Ok(()) => tracing::info!("Session accessibility enabled"),
            Err(e) => tracing::warn!("Failed to enable session accessibility: {}", e),
        }
    }

    // Create and run the server
    let server = EguiMcpServer::new();
    let service = server.serve(stdio()).await?;

    tracing::info!("Server started, waiting for connections...");
    service.waiting().await?;

    Ok(())
}
