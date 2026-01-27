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
use clap::{Parser, Subcommand};
use egui_mcp_protocol::MouseButton;
use ipc_client::IpcClient;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// MCP server for egui UI automation
#[derive(Parser)]
#[command(name = "egui-mcp-server")]
#[command(version, about, long_about = None)]
#[command(subcommand_required = true, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as MCP server
    Serve,
    /// Show setup guide for MCP client and egui app integration
    Guide,
}

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

/// Request for click_at tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ClickAtRequest {
    #[schemars(description = "X coordinate")]
    x: f32,
    #[schemars(description = "Y coordinate")]
    y: f32,
    #[schemars(description = "Mouse button: 'left', 'right', or 'middle' (default: 'left')")]
    button: Option<String>,
}

/// Request for take_screenshot tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TakeScreenshotRequest {
    #[schemars(
        description = "If true, save screenshot to a temp file and return the path. If false (default), return base64-encoded data."
    )]
    save_to_file: Option<bool>,
}

/// Request for keyboard_input tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct KeyboardInputRequest {
    #[schemars(description = "Key to send (e.g., 'a', 'Enter', 'Escape', 'Tab')")]
    key: String,
}

/// Request for scroll tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScrollRequest {
    #[schemars(description = "X coordinate where to scroll")]
    x: f32,
    #[schemars(description = "Y coordinate where to scroll")]
    y: f32,
    #[schemars(description = "Horizontal scroll delta (positive = right)")]
    delta_x: Option<f32>,
    #[schemars(description = "Vertical scroll delta (positive = down)")]
    delta_y: Option<f32>,
}

/// Request for hover tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HoverRequest {
    #[schemars(description = "X coordinate to move mouse to")]
    x: f32,
    #[schemars(description = "Y coordinate to move mouse to")]
    y: f32,
}

/// Request for drag tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DragRequest {
    #[schemars(description = "Starting X coordinate")]
    start_x: f32,
    #[schemars(description = "Starting Y coordinate")]
    start_y: f32,
    #[schemars(description = "Ending X coordinate")]
    end_x: f32,
    #[schemars(description = "Ending Y coordinate")]
    end_y: f32,
    #[schemars(description = "Mouse button: 'left', 'right', or 'middle' (default: 'left')")]
    button: Option<String>,
}

/// Request for double_click tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DoubleClickRequest {
    #[schemars(description = "X coordinate")]
    x: f32,
    #[schemars(description = "Y coordinate")]
    y: f32,
    #[schemars(description = "Mouse button: 'left', 'right', or 'middle' (default: 'left')")]
    button: Option<String>,
}

// ============================================================================
// Priority 1 (remaining): drag_element
// ============================================================================

/// Request for drag_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DragElementRequest {
    #[schemars(description = "Node ID of the element to drag (as string)")]
    source_id: String,
    #[schemars(description = "Ending X coordinate")]
    end_x: f32,
    #[schemars(description = "Ending Y coordinate")]
    end_y: f32,
    #[schemars(description = "Mouse button: 'left', 'right', or 'middle' (default: 'left')")]
    button: Option<String>,
}

// ============================================================================
// Priority 2: Element Information (AT-SPI Component)
// ============================================================================

/// Request for get_bounds tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetBoundsRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for focus_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FocusElementRequest {
    #[schemars(description = "Node ID of the element to focus (as string)")]
    id: String,
}

/// Request for scroll_to_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScrollToElementRequest {
    #[schemars(description = "Node ID of the element to scroll into view (as string)")]
    id: String,
}

// ============================================================================
// Priority 3: Value Operations (AT-SPI Value)
// ============================================================================

/// Request for get_value tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetValueRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for set_value tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetValueRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
    #[schemars(description = "Value to set (number)")]
    value: f64,
}

// ============================================================================
// Priority 4: Selection Operations (AT-SPI Selection)
// ============================================================================

/// Request for select_item tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SelectItemRequest {
    #[schemars(description = "Node ID of the container element (as string)")]
    id: String,
    #[schemars(description = "Index of the item to select (0-based)")]
    index: i32,
}

/// Request for deselect_item tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeselectItemRequest {
    #[schemars(description = "Node ID of the container element (as string)")]
    id: String,
    #[schemars(description = "Index of the item to deselect (0-based)")]
    index: i32,
}

/// Request for get_selected_count tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetSelectedCountRequest {
    #[schemars(description = "Node ID of the container element (as string)")]
    id: String,
}

/// Request for select_all/clear_selection tools
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SelectionContainerRequest {
    #[schemars(description = "Node ID of the container element (as string)")]
    id: String,
}

// ============================================================================
// Priority 5: Text Operations (AT-SPI Text)
// ============================================================================

/// Request for get_text tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetTextRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for get_text_selection tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetTextSelectionRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for set_text_selection tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetTextSelectionRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
    #[schemars(description = "Start offset of the selection")]
    start: i32,
    #[schemars(description = "End offset of the selection")]
    end: i32,
}

/// Request for get_caret_position tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetCaretPositionRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for set_caret_position tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetCaretPositionRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
    #[schemars(description = "Offset position for the caret")]
    offset: i32,
}

// ============================================================================
// Phase 7: Advanced Features
// ============================================================================

/// Request for state check tools (is_visible, is_enabled, is_focused, is_checked)
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ElementIdOnlyRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
}

/// Request for screenshot_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScreenshotElementRequest {
    #[schemars(description = "Node ID of the element to screenshot (as string)")]
    id: String,
    #[schemars(
        description = "If true, save screenshot to a temp file and return the path. If false (default), return base64-encoded data."
    )]
    save_to_file: Option<bool>,
}

/// Request for screenshot_region tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScreenshotRegionRequest {
    #[schemars(description = "X coordinate of the region")]
    x: f32,
    #[schemars(description = "Y coordinate of the region")]
    y: f32,
    #[schemars(description = "Width of the region")]
    width: f32,
    #[schemars(description = "Height of the region")]
    height: f32,
    #[schemars(
        description = "If true, save screenshot to a temp file and return the path. If false (default), return base64-encoded data."
    )]
    save_to_file: Option<bool>,
}

/// Request for wait_for_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WaitForElementRequest {
    #[schemars(description = "Label pattern to match (substring match)")]
    pattern: String,
    #[schemars(
        description = "If true (default), wait for element to appear. If false, wait for element to disappear."
    )]
    appear: Option<bool>,
    #[schemars(description = "Timeout in milliseconds (default: 5000)")]
    timeout_ms: Option<u64>,
}

/// Request for wait_for_state tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WaitForStateRequest {
    #[schemars(description = "Node ID of the element (as string)")]
    id: String,
    #[schemars(description = "State to wait for: 'visible', 'enabled', 'focused', or 'checked'")]
    state: String,
    #[schemars(description = "Expected state value (default: true)")]
    expected: Option<bool>,
    #[schemars(description = "Timeout in milliseconds (default: 5000)")]
    timeout_ms: Option<u64>,
}

// ============================================================================
// Phase 8: Testing & Debugging Features
// ============================================================================

/// Request for compare_screenshots tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CompareScreenshotsRequest {
    #[schemars(description = "First screenshot as base64-encoded PNG")]
    base64_a: Option<String>,
    #[schemars(description = "Second screenshot as base64-encoded PNG")]
    base64_b: Option<String>,
    #[schemars(description = "Path to first screenshot file (alternative to base64_a)")]
    path_a: Option<String>,
    #[schemars(description = "Path to second screenshot file (alternative to base64_b)")]
    path_b: Option<String>,
    #[schemars(
        description = "Comparison algorithm: 'hybrid' (default), 'mssim' (structural), 'rms' (pixel-wise)"
    )]
    algorithm: Option<String>,
}

/// Request for diff_screenshots tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DiffScreenshotsRequest {
    #[schemars(description = "First screenshot as base64-encoded PNG")]
    base64_a: Option<String>,
    #[schemars(description = "Second screenshot as base64-encoded PNG")]
    base64_b: Option<String>,
    #[schemars(description = "Path to first screenshot file (alternative to base64_a)")]
    path_a: Option<String>,
    #[schemars(description = "Path to second screenshot file (alternative to base64_b)")]
    path_b: Option<String>,
    #[schemars(
        description = "If true, save diff image to a temp file and return the path. If false (default), return base64-encoded data."
    )]
    save_to_file: Option<bool>,
}

/// Request for highlight_element tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HighlightElementRequest {
    #[schemars(description = "Node ID of the element to highlight (as string)")]
    id: String,
    #[schemars(
        description = "Highlight color as hex string (e.g., '#ff0000' or '#ff000080' with alpha). Default: red"
    )]
    color: Option<String>,
    #[schemars(
        description = "Duration in milliseconds. 0 = highlight until cleared. Default: 3000"
    )]
    duration_ms: Option<u64>,
}

/// Request for save_snapshot tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SaveSnapshotRequest {
    #[schemars(description = "Name to identify this snapshot")]
    name: String,
}

/// Request for load_snapshot tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LoadSnapshotRequest {
    #[schemars(description = "Name of the snapshot to load")]
    name: String,
}

/// Request for diff_snapshots tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DiffSnapshotsRequest {
    #[schemars(description = "Name of the first snapshot")]
    name_a: String,
    #[schemars(description = "Name of the second snapshot")]
    name_b: String,
}

/// Request for diff_current tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DiffCurrentRequest {
    #[schemars(description = "Name of the snapshot to compare with current state")]
    name: String,
}

/// Request for get_logs tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetLogsRequest {
    #[schemars(
        description = "Minimum log level to return: 'TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR'. If omitted, returns all levels."
    )]
    level: Option<String>,
    #[schemars(description = "Maximum number of entries to return (default: all)")]
    limit: Option<usize>,
}

/// Request for start_perf_recording tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StartPerfRecordingRequest {
    #[schemars(
        description = "Duration to record in milliseconds. 0 = until get_perf_report is called (default: 0)"
    )]
    duration_ms: Option<u64>,
}

/// Stored snapshot data (serialized UiTree)
type SnapshotStore = Arc<std::sync::RwLock<std::collections::HashMap<String, String>>>;

/// egui-mcp server handler
#[derive(Clone)]
struct EguiMcpServer {
    tool_router: ToolRouter<Self>,
    ipc_client: Arc<IpcClient>,
    snapshots: SnapshotStore,
    app_name: String,
}

impl EguiMcpServer {
    fn new(app_name: String) -> Self {
        let tool_router = Self::tool_router();
        let ipc_client = Arc::new(IpcClient::new());
        let snapshots = Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
        Self {
            tool_router,
            ipc_client,
            snapshots,
            app_name,
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
            match atspi_client::get_ui_tree_blocking(&self.app_name) {
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
            match atspi_client::find_by_label_blocking(&self.app_name, &pattern, false) {
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
            match atspi_client::find_by_label_blocking(&self.app_name, &pattern, true) {
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
            match atspi_client::find_by_role_blocking(&self.app_name, &role) {
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
            match atspi_client::get_element_blocking(&self.app_name, id) {
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
            match atspi_client::click_element_blocking(&self.app_name, id) {
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
        description = "Set text content of a text input element by its ID (as string). Note: Does not work with egui (AccessKit limitation). Use keyboard_input instead. Uses AT-SPI EditableText interface."
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
            match atspi_client::set_text_blocking(&self.app_name, id, &text) {
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
    async fn take_screenshot(
        &self,
        Parameters(TakeScreenshotRequest { save_to_file }): Parameters<TakeScreenshotRequest>,
    ) -> Content {
        if !self.ipc_client.is_socket_available() {
            return Content::text(json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string());
        }

        match self.ipc_client.take_screenshot().await {
            Ok((data, _format)) => {
                if save_to_file.unwrap_or(false) {
                    // Decode base64 and save to file
                    use base64::Engine;
                    match base64::engine::general_purpose::STANDARD.decode(&data) {
                        Ok(png_bytes) => {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_millis())
                                .unwrap_or(0);
                            let file_path = format!("/tmp/egui-mcp-screenshot-{}.png", timestamp);

                            match std::fs::write(&file_path, png_bytes.as_slice()) {
                                Ok(()) => Content::text(
                                    json!({
                                        "file_path": file_path,
                                        "size_bytes": png_bytes.len()
                                    })
                                    .to_string(),
                                ),
                                Err(e) => Content::text(
                                    json!({
                                        "error": "file_write_error",
                                        "message": format!("Failed to write screenshot file: {}", e)
                                    })
                                    .to_string(),
                                ),
                            }
                        }
                        Err(e) => Content::text(
                            json!({
                                "error": "decode_error",
                                "message": format!("Failed to decode base64 data: {}", e)
                            })
                            .to_string(),
                        ),
                    }
                } else {
                    // Return as MCP ImageContent (ideal for AI models)
                    Content::image(data, "image/png")
                }
            }
            Err(e) => Content::text(
                json!({
                    "error": "screenshot_error",
                    "message": format!("Failed to take screenshot: {}", e)
                })
                .to_string(),
            ),
        }
    }

    /// Click at specific coordinates
    #[tool(description = "Click at specific coordinates in the egui application window")]
    async fn click_at(
        &self,
        Parameters(ClickAtRequest { x, y, button }): Parameters<ClickAtRequest>,
    ) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        let mouse_button = match button.as_deref() {
            Some("right") => MouseButton::Right,
            Some("middle") => MouseButton::Middle,
            _ => MouseButton::Left,
        };

        match self.ipc_client.click_at(x, y, mouse_button).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Clicked at ({}, {})", x, y)
            })
            .to_string(),
            Err(e) => json!({
                "error": "click_error",
                "message": format!("Failed to click: {}", e)
            })
            .to_string(),
        }
    }

    /// Send keyboard input
    #[tool(description = "Send keyboard input to the egui application")]
    async fn keyboard_input(
        &self,
        Parameters(KeyboardInputRequest { key }): Parameters<KeyboardInputRequest>,
    ) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        match self.ipc_client.keyboard_input(&key).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Sent key: {}", key)
            })
            .to_string(),
            Err(e) => json!({
                "error": "keyboard_error",
                "message": format!("Failed to send keyboard input: {}", e)
            })
            .to_string(),
        }
    }

    /// Scroll at specific coordinates
    #[tool(description = "Scroll at specific coordinates in the egui application window")]
    async fn scroll(
        &self,
        Parameters(ScrollRequest {
            x,
            y,
            delta_x,
            delta_y,
        }): Parameters<ScrollRequest>,
    ) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        let dx = delta_x.unwrap_or(0.0);
        let dy = delta_y.unwrap_or(0.0);

        match self.ipc_client.scroll(x, y, dx, dy).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Scrolled at ({}, {}) with delta ({}, {})", x, y, dx, dy)
            })
            .to_string(),
            Err(e) => json!({
                "error": "scroll_error",
                "message": format!("Failed to scroll: {}", e)
            })
            .to_string(),
        }
    }

    /// Move mouse to specific coordinates (hover)
    #[tool(
        description = "Move mouse to specific coordinates in the egui application window (hover)"
    )]
    async fn hover(&self, Parameters(HoverRequest { x, y }): Parameters<HoverRequest>) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        match self.ipc_client.move_mouse(x, y).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Moved mouse to ({}, {})", x, y)
            })
            .to_string(),
            Err(e) => json!({
                "error": "hover_error",
                "message": format!("Failed to move mouse: {}", e)
            })
            .to_string(),
        }
    }

    /// Drag from one point to another
    #[tool(description = "Drag from one point to another in the egui application window")]
    async fn drag(
        &self,
        Parameters(DragRequest {
            start_x,
            start_y,
            end_x,
            end_y,
            button,
        }): Parameters<DragRequest>,
    ) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        let mouse_button = match button.as_deref() {
            Some("right") => MouseButton::Right,
            Some("middle") => MouseButton::Middle,
            _ => MouseButton::Left,
        };

        match self.ipc_client.drag(start_x, start_y, end_x, end_y, mouse_button).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Dragged from ({}, {}) to ({}, {})", start_x, start_y, end_x, end_y)
            })
            .to_string(),
            Err(e) => json!({
                "error": "drag_error",
                "message": format!("Failed to drag: {}", e)
            })
            .to_string(),
        }
    }

    /// Double click at specific coordinates
    #[tool(description = "Double click at specific coordinates in the egui application window")]
    async fn double_click(
        &self,
        Parameters(DoubleClickRequest { x, y, button }): Parameters<DoubleClickRequest>,
    ) -> String {
        if !self.ipc_client.is_socket_available() {
            return json!({
                "error": "not_connected",
                "message": "No egui application socket found. Make sure the egui app is running with egui-mcp-client."
            }).to_string();
        }

        let mouse_button = match button.as_deref() {
            Some("right") => MouseButton::Right,
            Some("middle") => MouseButton::Middle,
            _ => MouseButton::Left,
        };

        match self.ipc_client.double_click(x, y, mouse_button).await {
            Ok(()) => json!({
                "success": true,
                "message": format!("Double clicked at ({}, {})", x, y)
            })
            .to_string(),
            Err(e) => json!({
                "error": "double_click_error",
                "message": format!("Failed to double click: {}", e)
            })
            .to_string(),
        }
    }

    // ========================================================================
    // Priority 1 (remaining): drag_element
    // ========================================================================

    /// Drag an element to a target position (combines get_bounds + drag)
    #[tool(
        description = "Drag a UI element to a target position. Gets element center via AT-SPI and drags to target coordinates via IPC."
    )]
    async fn drag_element(
        &self,
        Parameters(DragElementRequest {
            source_id,
            end_x,
            end_y,
            button,
        }): Parameters<DragElementRequest>,
    ) -> String {
        let id: u64 = match source_id.parse() {
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
            // First, get the element bounds
            match atspi_client::get_bounds_blocking(&self.app_name, id) {
                Ok(Some(bounds)) => {
                    // Calculate center of the element
                    let start_x = bounds.x + bounds.width / 2.0;
                    let start_y = bounds.y + bounds.height / 2.0;

                    // Now perform the drag via IPC
                    if !self.ipc_client.is_socket_available() {
                        return json!({
                            "error": "not_connected",
                            "message": "No egui application socket found."
                        })
                        .to_string();
                    }

                    let mouse_button = match button.as_deref() {
                        Some("right") => MouseButton::Right,
                        Some("middle") => MouseButton::Middle,
                        _ => MouseButton::Left,
                    };

                    match self
                        .ipc_client
                        .drag(start_x, start_y, end_x, end_y, mouse_button)
                        .await
                    {
                        Ok(()) => json!({
                            "success": true,
                            "message": format!("Dragged element {} from ({:.1}, {:.1}) to ({}, {})", id, start_x, start_y, end_x, end_y),
                            "source_bounds": {
                                "x": bounds.x,
                                "y": bounds.y,
                                "width": bounds.width,
                                "height": bounds.height
                            }
                        })
                        .to_string(),
                        Err(e) => json!({
                            "error": "drag_error",
                            "message": format!("Failed to drag: {}", e)
                        })
                        .to_string(),
                    }
                }
                Ok(None) => json!({
                    "error": "no_bounds",
                    "message": format!("Element {} does not have Component interface (no bounds available)", id)
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_bounds_error",
                    "message": format!("Failed to get element bounds: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, end_x, end_y, button);
            json!({
                "error": "not_available",
                "message": "drag_element requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Priority 2: Element Information (AT-SPI Component)
    // ========================================================================

    /// Get element bounding box
    #[tool(
        description = "Get the bounding box (position and size) of a UI element by ID. Uses AT-SPI Component interface."
    )]
    async fn get_bounds(
        &self,
        Parameters(GetBoundsRequest { id }): Parameters<GetBoundsRequest>,
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
            match atspi_client::get_bounds_blocking(&self.app_name, id) {
                Ok(Some(bounds)) => json!({
                    "x": bounds.x,
                    "y": bounds.y,
                    "width": bounds.width,
                    "height": bounds.height,
                    "center_x": bounds.x + bounds.width / 2.0,
                    "center_y": bounds.y + bounds.height / 2.0
                })
                .to_string(),
                Ok(None) => json!({
                    "error": "no_component",
                    "message": "Element does not have Component interface"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_bounds_error",
                    "message": format!("Failed to get bounds: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_bounds requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Focus an element
    #[tool(description = "Focus a UI element by ID. Uses AT-SPI Component interface.")]
    async fn focus_element(
        &self,
        Parameters(FocusElementRequest { id }): Parameters<FocusElementRequest>,
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
            match atspi_client::focus_element_blocking(&self.app_name, id) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Focused element with id {}", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Focus action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "focus_error",
                    "message": format!("Failed to focus element: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "focus_element requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Scroll element into view
    #[tool(description = "Scroll a UI element into view by ID. Uses AT-SPI Component interface.")]
    async fn scroll_to_element(
        &self,
        Parameters(ScrollToElementRequest { id }): Parameters<ScrollToElementRequest>,
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
            match atspi_client::scroll_to_element_blocking(&self.app_name, id) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Scrolled element {} into view", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Scroll action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "scroll_error",
                    "message": format!("Failed to scroll element into view: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "scroll_to_element requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Priority 3: Value Operations (AT-SPI Value)
    // ========================================================================

    /// Get value of an element (for sliders, progress bars, etc.)
    #[tool(
        description = "Get the current value, min, max, and increment of a value element (slider, progress bar, etc.). Uses AT-SPI Value interface."
    )]
    async fn get_value(
        &self,
        Parameters(GetValueRequest { id }): Parameters<GetValueRequest>,
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
            match atspi_client::get_value_blocking(&self.app_name, id) {
                Ok(Some(value_info)) => {
                    serde_json::to_string_pretty(&value_info).unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize value: {}", e)
                        })
                        .to_string()
                    })
                }
                Ok(None) => json!({
                    "error": "no_value",
                    "message": "Element does not have Value interface"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_value_error",
                    "message": format!("Failed to get value: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_value requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Set value of an element (for sliders, etc.)
    #[tool(
        description = "Set the value of a value element (slider, etc.). Uses AT-SPI Value interface."
    )]
    async fn set_value(
        &self,
        Parameters(SetValueRequest { id, value }): Parameters<SetValueRequest>,
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
            match atspi_client::set_value_blocking(&self.app_name, id, value) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Set value to {} on element {}", value, id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Set value action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "set_value_error",
                    "message": format!("Failed to set value: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, value);
            json!({
                "error": "not_available",
                "message": "set_value requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Priority 4: Selection Operations (AT-SPI Selection)
    // ========================================================================

    /// Select an item in a selection container
    #[tool(
        description = "Select an item by index in a selection container (list, combo box, etc.). Note: Does not work with egui ComboBox (items not exposed as children). Use click_at + keyboard_input instead. Uses AT-SPI Selection interface."
    )]
    async fn select_item(
        &self,
        Parameters(SelectItemRequest { id, index }): Parameters<SelectItemRequest>,
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
            match atspi_client::select_item_blocking(&self.app_name, id, index) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Selected item at index {} in element {}", index, id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Select action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "select_error",
                    "message": format!("Failed to select item: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, index);
            json!({
                "error": "not_available",
                "message": "select_item requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Deselect an item in a selection container
    #[tool(
        description = "Deselect an item by index in a selection container. Note: Does not work with egui ComboBox. Use click_at + keyboard_input instead. Uses AT-SPI Selection interface."
    )]
    async fn deselect_item(
        &self,
        Parameters(DeselectItemRequest { id, index }): Parameters<DeselectItemRequest>,
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
            match atspi_client::deselect_item_blocking(&self.app_name, id, index) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Deselected item at index {} in element {}", index, id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Deselect action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "deselect_error",
                    "message": format!("Failed to deselect item: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, index);
            json!({
                "error": "not_available",
                "message": "deselect_item requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Get count of selected items
    #[tool(
        description = "Get the number of selected items in a selection container. For egui ComboBox, checks name property (returns 0 or 1). Uses AT-SPI Selection interface."
    )]
    async fn get_selected_count(
        &self,
        Parameters(GetSelectedCountRequest { id }): Parameters<GetSelectedCountRequest>,
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
            match atspi_client::get_selected_count_blocking(&self.app_name, id) {
                Ok(count) => json!({
                    "count": count,
                    "message": format!("Element {} has {} selected items", id, count)
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_selected_count_error",
                    "message": format!("Failed to get selected count: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_selected_count requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Select all items in a selection container
    #[tool(
        description = "Select all items in a selection container. Note: Not useful for egui (only has single-selection widgets like ComboBox and RadioGroup). Uses AT-SPI Selection interface."
    )]
    async fn select_all(
        &self,
        Parameters(SelectionContainerRequest { id }): Parameters<SelectionContainerRequest>,
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
            match atspi_client::select_all_blocking(&self.app_name, id) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Selected all items in element {}", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Select all action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "select_all_error",
                    "message": format!("Failed to select all: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "select_all requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Clear all selections in a selection container
    #[tool(
        description = "Clear all selections in a selection container. Note: Not useful for egui (only has single-selection widgets like ComboBox and RadioGroup). Uses AT-SPI Selection interface."
    )]
    async fn clear_selection(
        &self,
        Parameters(SelectionContainerRequest { id }): Parameters<SelectionContainerRequest>,
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
            match atspi_client::clear_selection_blocking(&self.app_name, id) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Cleared all selections in element {}", id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Clear selection action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "clear_selection_error",
                    "message": format!("Failed to clear selection: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "clear_selection requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Priority 5: Text Operations (AT-SPI Text)
    // ========================================================================

    /// Get text content of an element
    #[tool(
        description = "Get the text content, length, and caret position of a text element. Uses AT-SPI Text interface."
    )]
    async fn get_text(
        &self,
        Parameters(GetTextRequest { id }): Parameters<GetTextRequest>,
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
            match atspi_client::get_text_blocking(&self.app_name, id) {
                Ok(Some(text_info)) => {
                    serde_json::to_string_pretty(&text_info).unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize text: {}", e)
                        })
                        .to_string()
                    })
                }
                Ok(None) => json!({
                    "error": "no_text",
                    "message": "Element does not have Text interface"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_text_error",
                    "message": format!("Failed to get text: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_text requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Get text selection range
    #[tool(
        description = "Get the current text selection range (start and end offsets). Returns -1 if element has no focus. Uses AT-SPI Text interface."
    )]
    async fn get_text_selection(
        &self,
        Parameters(GetTextSelectionRequest { id }): Parameters<GetTextSelectionRequest>,
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
            match atspi_client::get_text_selection_blocking(&self.app_name, id) {
                Ok(Some(selection)) => {
                    serde_json::to_string_pretty(&selection).unwrap_or_else(|e| {
                        json!({
                            "error": "serialization_error",
                            "message": format!("Failed to serialize selection: {}", e)
                        })
                        .to_string()
                    })
                }
                Ok(None) => json!({
                    "has_selection": false,
                    "message": "No text selection"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_text_selection_error",
                    "message": format!("Failed to get text selection: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_text_selection requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Set text selection range
    #[tool(
        description = "Set the text selection range (start and end offsets). Requires focus first (use focus_element). Uses AT-SPI Text interface."
    )]
    async fn set_text_selection(
        &self,
        Parameters(SetTextSelectionRequest { id, start, end }): Parameters<SetTextSelectionRequest>,
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
            match atspi_client::set_text_selection_blocking(&self.app_name, id, start, end) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Set text selection from {} to {} on element {}", start, end, id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Set text selection action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "set_text_selection_error",
                    "message": format!("Failed to set text selection: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, start, end);
            json!({
                "error": "not_available",
                "message": "set_text_selection requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Get caret (cursor) position
    #[tool(
        description = "Get the current caret (cursor) position in a text element. Returns -1 if element has no focus. Uses AT-SPI Text interface."
    )]
    async fn get_caret_position(
        &self,
        Parameters(GetCaretPositionRequest { id }): Parameters<GetCaretPositionRequest>,
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
            match atspi_client::get_caret_position_blocking(&self.app_name, id) {
                Ok(offset) => json!({
                    "offset": offset,
                    "message": format!("Caret at position {} in element {}", offset, id)
                })
                .to_string(),
                Err(e) => json!({
                    "error": "get_caret_position_error",
                    "message": format!("Failed to get caret position: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "get_caret_position requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Set caret (cursor) position
    #[tool(
        description = "Set the caret (cursor) position in a text element. Requires focus first (use focus_element). Uses AT-SPI Text interface."
    )]
    async fn set_caret_position(
        &self,
        Parameters(SetCaretPositionRequest { id, offset }): Parameters<SetCaretPositionRequest>,
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
            match atspi_client::set_caret_position_blocking(&self.app_name, id, offset) {
                Ok(true) => json!({
                    "success": true,
                    "message": format!("Set caret to position {} in element {}", offset, id)
                })
                .to_string(),
                Ok(false) => json!({
                    "success": false,
                    "message": "Set caret position action returned false"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "set_caret_position_error",
                    "message": format!("Failed to set caret position: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, offset);
            json!({
                "error": "not_available",
                "message": "set_caret_position requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Phase 7: Advanced Features - State Queries
    // ========================================================================

    /// Check if element is visible
    #[tool(description = "Check if a UI element is visible. Uses AT-SPI State interface.")]
    async fn is_visible(
        &self,
        Parameters(ElementIdOnlyRequest { id }): Parameters<ElementIdOnlyRequest>,
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
            match atspi_client::is_visible_blocking(&self.app_name, id) {
                Ok(visible) => json!({
                    "id": id,
                    "visible": visible
                })
                .to_string(),
                Err(e) => json!({
                    "error": "is_visible_error",
                    "message": format!("Failed to check visibility: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "is_visible requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Check if element is enabled
    #[tool(description = "Check if a UI element is enabled. Uses AT-SPI State interface.")]
    async fn is_enabled(
        &self,
        Parameters(ElementIdOnlyRequest { id }): Parameters<ElementIdOnlyRequest>,
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
            match atspi_client::is_enabled_blocking(&self.app_name, id) {
                Ok(enabled) => json!({
                    "id": id,
                    "enabled": enabled
                })
                .to_string(),
                Err(e) => json!({
                    "error": "is_enabled_error",
                    "message": format!("Failed to check enabled state: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "is_enabled requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Check if element is focused
    #[tool(description = "Check if a UI element is focused. Uses AT-SPI State interface.")]
    async fn is_focused(
        &self,
        Parameters(ElementIdOnlyRequest { id }): Parameters<ElementIdOnlyRequest>,
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
            match atspi_client::is_focused_blocking(&self.app_name, id) {
                Ok(focused) => json!({
                    "id": id,
                    "focused": focused
                })
                .to_string(),
                Err(e) => json!({
                    "error": "is_focused_error",
                    "message": format!("Failed to check focus state: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "is_focused requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Check if element is checked/pressed
    #[tool(
        description = "Check if a UI element is checked or pressed (for checkboxes, toggle buttons). Returns checked: true/false for checkable elements, or checked: null for non-checkable elements. Uses AT-SPI State interface."
    )]
    async fn is_checked(
        &self,
        Parameters(ElementIdOnlyRequest { id }): Parameters<ElementIdOnlyRequest>,
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
            match atspi_client::is_checked_blocking(&self.app_name, id) {
                Ok(checked) => json!({
                    "id": id,
                    "checked": checked,
                    "is_checkable": checked.is_some()
                })
                .to_string(),
                Err(e) => json!({
                    "error": "is_checked_error",
                    "message": format!("Failed to check checked state: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            json!({
                "error": "not_available",
                "message": "is_checked requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Phase 7: Advanced Features - Screenshot Enhancements
    // ========================================================================

    /// Take screenshot of a specific element
    #[tool(
        description = "Take a screenshot of a specific UI element by ID. Captures the full screen and crops to element bounds."
    )]
    async fn screenshot_element(
        &self,
        Parameters(ScreenshotElementRequest { id, save_to_file }): Parameters<
            ScreenshotElementRequest,
        >,
    ) -> Content {
        let id: u64 = match id.parse() {
            Ok(id) => id,
            Err(_) => {
                return Content::text(
                    json!({
                        "error": "invalid_id",
                        "message": "ID must be a valid unsigned integer"
                    })
                    .to_string(),
                );
            }
        };

        #[cfg(target_os = "linux")]
        {
            // First get element bounds
            let bounds = match atspi_client::get_bounds_blocking(&self.app_name, id) {
                Ok(Some(b)) => b,
                Ok(None) => {
                    return Content::text(json!({
                        "error": "no_bounds",
                        "message": "Element does not have Component interface (no bounds available)"
                    }).to_string());
                }
                Err(e) => {
                    return Content::text(
                        json!({
                            "error": "get_bounds_error",
                            "message": format!("Failed to get element bounds: {}", e)
                        })
                        .to_string(),
                    );
                }
            };

            if !self.ipc_client.is_socket_available() {
                return Content::text(
                    json!({
                        "error": "not_connected",
                        "message": "No egui application socket found."
                    })
                    .to_string(),
                );
            }

            // Take cropped screenshot directly from client
            match self
                .ipc_client
                .take_screenshot_region(bounds.x, bounds.y, bounds.width, bounds.height)
                .await
            {
                Ok((data, _format)) => {
                    if save_to_file.unwrap_or(false) {
                        self.save_screenshot_to_file(&data)
                    } else {
                        Content::image(data, "image/png")
                    }
                }
                Err(e) => Content::text(
                    json!({
                        "error": "screenshot_error",
                        "message": format!("Failed to take screenshot: {}", e)
                    })
                    .to_string(),
                ),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, save_to_file);
            Content::text(
                json!({
                    "error": "not_available",
                    "message": "screenshot_element requires AT-SPI on Linux."
                })
                .to_string(),
            )
        }
    }

    /// Take screenshot of a specific region
    #[tool(
        description = "Take a screenshot of a specific region. Captures the full screen and crops to the specified coordinates."
    )]
    async fn screenshot_region(
        &self,
        Parameters(ScreenshotRegionRequest {
            x,
            y,
            width,
            height,
            save_to_file,
        }): Parameters<ScreenshotRegionRequest>,
    ) -> Content {
        if !self.ipc_client.is_socket_available() {
            return Content::text(
                json!({
                    "error": "not_connected",
                    "message": "No egui application socket found."
                })
                .to_string(),
            );
        }

        // Take cropped screenshot directly from client
        match self
            .ipc_client
            .take_screenshot_region(x, y, width, height)
            .await
        {
            Ok((data, _format)) => {
                if save_to_file.unwrap_or(false) {
                    self.save_screenshot_to_file(&data)
                } else {
                    Content::image(data, "image/png")
                }
            }
            Err(e) => Content::text(
                json!({
                    "error": "screenshot_error",
                    "message": format!("Failed to take screenshot: {}", e)
                })
                .to_string(),
            ),
        }
    }

    // ========================================================================
    // Phase 7: Advanced Features - Wait/Polling Operations
    // ========================================================================

    /// Wait for element to appear or disappear
    #[tool(
        description = "Wait for a UI element to appear or disappear. Polls every 100ms until the condition is met or timeout."
    )]
    async fn wait_for_element(
        &self,
        Parameters(WaitForElementRequest {
            pattern,
            appear,
            timeout_ms,
        }): Parameters<WaitForElementRequest>,
    ) -> String {
        let timeout = timeout_ms.unwrap_or(5000);
        let appear = appear.unwrap_or(true);
        let start = std::time::Instant::now();

        #[cfg(target_os = "linux")]
        {
            loop {
                let results = atspi_client::find_by_label_blocking(&self.app_name, &pattern, false);
                let found = results.map(|r| !r.is_empty()).unwrap_or(false);

                if found == appear {
                    return json!({
                        "success": true,
                        "found": found,
                        "elapsed_ms": start.elapsed().as_millis()
                    })
                    .to_string();
                }

                if start.elapsed().as_millis() as u64 > timeout {
                    return json!({
                        "success": false,
                        "timeout": true,
                        "found": found,
                        "elapsed_ms": start.elapsed().as_millis()
                    })
                    .to_string();
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (pattern, appear, timeout, start);
            json!({
                "error": "not_available",
                "message": "wait_for_element requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Wait for element state to change
    #[tool(
        description = "Wait for a UI element's state to reach an expected value. Polls every 100ms until the condition is met or timeout. Supported states: 'visible', 'enabled', 'focused', 'checked'."
    )]
    async fn wait_for_state(
        &self,
        Parameters(WaitForStateRequest {
            id,
            state,
            expected,
            timeout_ms,
        }): Parameters<WaitForStateRequest>,
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

        let timeout = timeout_ms.unwrap_or(5000);
        let expected = expected.unwrap_or(true);
        let start = std::time::Instant::now();

        #[cfg(target_os = "linux")]
        {
            loop {
                let current_state = match state.to_lowercase().as_str() {
                    "visible" => atspi_client::is_visible_blocking(&self.app_name, id).ok(),
                    "enabled" => atspi_client::is_enabled_blocking(&self.app_name, id).ok(),
                    "focused" => atspi_client::is_focused_blocking(&self.app_name, id).ok(),
                    "checked" => atspi_client::is_checked_blocking(&self.app_name, id)
                        .ok()
                        .flatten(),
                    _ => {
                        return json!({
                            "error": "invalid_state",
                            "message": format!("Unknown state: '{}'. Supported: visible, enabled, focused, checked", state)
                        }).to_string();
                    }
                };

                if let Some(current) = current_state
                    && current == expected
                {
                    return json!({
                        "success": true,
                        "state": state,
                        "value": current,
                        "elapsed_ms": start.elapsed().as_millis()
                    })
                    .to_string();
                }

                if start.elapsed().as_millis() as u64 > timeout {
                    return json!({
                        "success": false,
                        "timeout": true,
                        "state": state,
                        "current_value": current_state,
                        "expected": expected,
                        "elapsed_ms": start.elapsed().as_millis()
                    })
                    .to_string();
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, state, expected, timeout, start);
            json!({
                "error": "not_available",
                "message": "wait_for_state requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // ========================================================================
    // Phase 8: Testing & Debugging Features
    // ========================================================================

    /// Helper to load image from either base64 or file path
    fn load_image_from_source(
        base64_data: Option<&str>,
        file_path: Option<&str>,
        name: &str,
    ) -> Result<image::RgbaImage, String> {
        use base64::Engine;

        if let Some(path) = file_path {
            // Load from file
            match std::fs::read(path) {
                Ok(bytes) => match image::load_from_memory(&bytes) {
                    Ok(img) => Ok(img.to_rgba8()),
                    Err(e) => Err(format!("Failed to load {} image from file: {}", name, e)),
                },
                Err(e) => Err(format!("Failed to read {} file '{}': {}", name, path, e)),
            }
        } else if let Some(b64) = base64_data {
            // Load from base64
            match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(bytes) => match image::load_from_memory(&bytes) {
                    Ok(img) => Ok(img.to_rgba8()),
                    Err(e) => Err(format!("Failed to load {} image: {}", name, e)),
                },
                Err(e) => Err(format!("Failed to decode {} base64: {}", name, e)),
            }
        } else {
            Err(format!(
                "No {} image provided. Use base64_{} or path_{}",
                name,
                name.chars().next().unwrap_or('a'),
                name.chars().next().unwrap_or('a')
            ))
        }
    }

    /// Compare two screenshots and return similarity score
    #[tool(
        description = "Compare two screenshots and return similarity score. Returns a score between 0.0 (completely different) and 1.0 (identical)."
    )]
    async fn compare_screenshots(
        &self,
        Parameters(req): Parameters<CompareScreenshotsRequest>,
    ) -> String {
        let start = std::time::Instant::now();
        let algorithm = req.algorithm.as_deref().unwrap_or("hybrid");

        // Load first image (prefer file path over base64)
        let img_a = match Self::load_image_from_source(
            req.base64_a.as_deref(),
            req.path_a.as_deref(),
            "first",
        ) {
            Ok(img) => img,
            Err(e) => {
                return json!({
                    "error": "load_error",
                    "message": e
                })
                .to_string();
            }
        };

        // Load second image (prefer file path over base64)
        let img_b = match Self::load_image_from_source(
            req.base64_b.as_deref(),
            req.path_b.as_deref(),
            "second",
        ) {
            Ok(img) => img,
            Err(e) => {
                return json!({
                    "error": "load_error",
                    "message": e
                })
                .to_string();
            }
        };

        // Check dimensions match
        if img_a.dimensions() != img_b.dimensions() {
            return json!({
                "error": "dimension_mismatch",
                "message": format!(
                    "Image dimensions don't match: {:?} vs {:?}",
                    img_a.dimensions(),
                    img_b.dimensions()
                ),
                "dimensions_a": { "width": img_a.width(), "height": img_a.height() },
                "dimensions_b": { "width": img_b.width(), "height": img_b.height() }
            })
            .to_string();
        }

        // Compare images based on algorithm
        let result = match algorithm {
            "mssim" => {
                // MSSIM comparison using gray images
                let gray_a = image::DynamicImage::ImageRgba8(img_a.clone()).to_luma8();
                let gray_b = image::DynamicImage::ImageRgba8(img_b.clone()).to_luma8();
                image_compare::gray_similarity_structure(
                    &image_compare::Algorithm::MSSIMSimple,
                    &gray_a,
                    &gray_b,
                )
            }
            "rms" => {
                // RMS comparison using gray images
                let gray_a = image::DynamicImage::ImageRgba8(img_a.clone()).to_luma8();
                let gray_b = image::DynamicImage::ImageRgba8(img_b.clone()).to_luma8();
                image_compare::gray_similarity_structure(
                    &image_compare::Algorithm::RootMeanSquared,
                    &gray_a,
                    &gray_b,
                )
            }
            _ => image_compare::rgba_hybrid_compare(&img_a, &img_b),
        };

        let elapsed = start.elapsed();
        tracing::info!("compare_screenshots took {:?}", elapsed);

        match result {
            Ok(similarity) => json!({
                "score": similarity.score,
                "algorithm": algorithm,
                "dimensions": { "width": img_a.width(), "height": img_a.height() },
                "elapsed_ms": elapsed.as_millis()
            })
            .to_string(),
            Err(e) => json!({
                "error": "comparison_error",
                "message": format!("Failed to compare images: {}", e)
            })
            .to_string(),
        }
    }

    /// Generate a visual diff image highlighting differences between two screenshots
    #[tool(
        description = "Generate a visual diff image highlighting differences between two screenshots. Returns the diff image as base64-encoded PNG."
    )]
    async fn diff_screenshots(
        &self,
        Parameters(req): Parameters<DiffScreenshotsRequest>,
    ) -> Content {
        use base64::Engine;

        let start = std::time::Instant::now();
        let save_to_file = req.save_to_file.unwrap_or(false);

        // Load first image (prefer file path over base64)
        let img_a = match Self::load_image_from_source(
            req.base64_a.as_deref(),
            req.path_a.as_deref(),
            "first",
        ) {
            Ok(img) => img,
            Err(e) => {
                return Content::text(
                    json!({
                        "error": "load_error",
                        "message": e
                    })
                    .to_string(),
                );
            }
        };

        // Load second image (prefer file path over base64)
        let img_b = match Self::load_image_from_source(
            req.base64_b.as_deref(),
            req.path_b.as_deref(),
            "second",
        ) {
            Ok(img) => img,
            Err(e) => {
                return Content::text(
                    json!({
                        "error": "load_error",
                        "message": e
                    })
                    .to_string(),
                );
            }
        };

        // Check dimensions match
        if img_a.dimensions() != img_b.dimensions() {
            return Content::text(
                json!({
                    "error": "dimension_mismatch",
                    "message": format!(
                        "Image dimensions don't match: {:?} vs {:?}",
                        img_a.dimensions(),
                        img_b.dimensions()
                    )
                })
                .to_string(),
            );
        }

        // Compare and get diff image
        let result = image_compare::rgba_hybrid_compare(&img_a, &img_b);

        match result {
            Ok(comparison) => {
                // Convert the similarity image to a color map (DynamicImage)
                let diff_dynamic = comparison.image.to_color_map();
                let diff_rgba = diff_dynamic.to_rgba8();
                let (width, height) = diff_rgba.dimensions();

                // Create a colored diff for better visibility
                // In hybrid mode: 0.0 = no difference, 1.0 = maximum difference
                // The color map converts this to grayscale where darker = more similar
                let mut colored_diff = image::RgbaImage::new(width, height);

                for y in 0..height {
                    for x in 0..width {
                        let pixel = diff_rgba.get_pixel(x, y);
                        // In the color map, the gray value indicates similarity
                        // Lighter pixels = more difference
                        let diff_value = pixel[0]; // Use first channel (grayscale)

                        if diff_value > 10 {
                            // Highlight differences in red with intensity based on difference
                            let alpha = (diff_value as f32 * 0.8) as u8 + 50;
                            colored_diff.put_pixel(x, y, image::Rgba([255, 0, 0, alpha]));
                        } else {
                            // Keep similar areas semi-transparent with original image
                            let orig_pixel = img_a.get_pixel(x, y);
                            colored_diff.put_pixel(
                                x,
                                y,
                                image::Rgba([orig_pixel[0], orig_pixel[1], orig_pixel[2], 128]),
                            );
                        }
                    }
                }

                // Encode to PNG
                let mut buf = Vec::new();
                match colored_diff
                    .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                {
                    Ok(()) => {
                        let elapsed = start.elapsed();
                        tracing::info!("diff_screenshots took {:?}", elapsed);

                        if save_to_file {
                            // Save to temp file and return path
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis();
                            let file_path = format!("/tmp/egui-mcp-diff-{}.png", timestamp);
                            match std::fs::write(&file_path, &buf) {
                                Ok(()) => Content::text(
                                    json!({
                                        "file_path": file_path,
                                        "size_bytes": buf.len(),
                                        "elapsed_ms": elapsed.as_millis()
                                    })
                                    .to_string(),
                                ),
                                Err(e) => Content::text(
                                    json!({
                                        "error": "write_error",
                                        "message": format!("Failed to write diff file: {}", e)
                                    })
                                    .to_string(),
                                ),
                            }
                        } else {
                            let encoded = base64::engine::general_purpose::STANDARD.encode(&buf);
                            Content::image(encoded, "image/png")
                        }
                    }
                    Err(e) => Content::text(
                        json!({
                            "error": "encode_error",
                            "message": format!("Failed to encode diff image: {}", e)
                        })
                        .to_string(),
                    ),
                }
            }
            Err(e) => Content::text(
                json!({
                    "error": "comparison_error",
                    "message": format!("Failed to compare images: {}", e)
                })
                .to_string(),
            ),
        }
    }

    /// Highlight an element with a colored border
    #[tool(
        description = "Draw highlight overlay on element by ID. Requires AT-SPI to get element bounds."
    )]
    async fn highlight_element(
        &self,
        Parameters(req): Parameters<HighlightElementRequest>,
    ) -> String {
        let id: u64 = match req.id.parse() {
            Ok(id) => id,
            Err(_) => {
                return json!({
                    "error": "invalid_id",
                    "message": format!("Invalid ID format: {}", req.id)
                })
                .to_string();
            }
        };

        // Parse color from hex string
        let color = req.color.as_deref().unwrap_or("#ff0000ff");
        let color = parse_hex_color(color).unwrap_or([255, 0, 0, 200]); // Default: red with alpha

        let duration_ms = req.duration_ms.unwrap_or(3000);

        #[cfg(target_os = "linux")]
        {
            // Get element bounds via AT-SPI
            let bounds = atspi_client::get_bounds_blocking(&self.app_name, id);
            match bounds {
                Ok(Some(rect)) => {
                    // Send highlight request via IPC
                    match self
                        .ipc_client
                        .highlight_element(
                            rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                            color,
                            duration_ms,
                        )
                        .await
                    {
                        Ok(()) => json!({
                            "success": true,
                            "id": id,
                            "bounds": { "x": rect.x, "y": rect.y, "width": rect.width, "height": rect.height },
                            "duration_ms": duration_ms
                        })
                        .to_string(),
                        Err(e) => json!({
                            "error": "ipc_error",
                            "message": format!("Failed to send highlight request: {}", e)
                        })
                        .to_string(),
                    }
                }
                Ok(None) => json!({
                    "error": "no_bounds",
                    "message": format!("Element {} has no bounds", id)
                })
                .to_string(),
                Err(e) => json!({
                    "error": "atspi_error",
                    "message": format!("Failed to get element bounds: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (id, color, duration_ms);
            json!({
                "error": "not_available",
                "message": "highlight_element requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Clear all highlights
    #[tool(description = "Remove all highlights")]
    async fn clear_highlights(&self) -> String {
        match self.ipc_client.clear_highlights().await {
            Ok(()) => json!({
                "success": true,
                "message": "All highlights cleared"
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to clear highlights: {}", e)
            })
            .to_string(),
        }
    }

    // ========================================================================
    // Phase 8.3: Snapshot Diff
    // ========================================================================

    /// Save current UI tree state as a named snapshot
    #[tool(description = "Save current UI tree state as a named snapshot for later comparison")]
    async fn save_snapshot(&self, Parameters(req): Parameters<SaveSnapshotRequest>) -> String {
        #[cfg(target_os = "linux")]
        {
            // Get current UI tree
            match atspi_client::get_ui_tree_blocking(&self.app_name) {
                Ok(Some(tree)) => {
                    // Serialize to JSON
                    let json = serde_json::to_string(&tree).unwrap_or_default();
                    let node_count = tree.nodes.len();

                    // Store snapshot
                    if let Ok(mut snapshots) = self.snapshots.write() {
                        snapshots.insert(req.name.clone(), json);
                    }

                    json!({
                        "success": true,
                        "name": req.name,
                        "node_count": node_count
                    })
                    .to_string()
                }
                Ok(None) => json!({
                    "error": "no_tree",
                    "message": "No UI tree available"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "atspi_error",
                    "message": format!("Failed to get UI tree: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = req;
            json!({
                "error": "not_available",
                "message": "save_snapshot requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    /// Load a saved snapshot
    #[tool(description = "Load a saved UI tree snapshot")]
    async fn load_snapshot(&self, Parameters(req): Parameters<LoadSnapshotRequest>) -> String {
        if let Ok(snapshots) = self.snapshots.read() {
            if let Some(json) = snapshots.get(&req.name) {
                match serde_json::from_str::<egui_mcp_protocol::UiTree>(json) {
                    Ok(tree) => json!({
                        "success": true,
                        "name": req.name,
                        "node_count": tree.nodes.len(),
                        "tree": tree
                    })
                    .to_string(),
                    Err(e) => json!({
                        "error": "parse_error",
                        "message": format!("Failed to parse snapshot: {}", e)
                    })
                    .to_string(),
                }
            } else {
                json!({
                    "error": "not_found",
                    "message": format!("Snapshot '{}' not found", req.name)
                })
                .to_string()
            }
        } else {
            json!({
                "error": "lock_error",
                "message": "Failed to acquire snapshot lock"
            })
            .to_string()
        }
    }

    /// Compare two saved snapshots
    #[tool(description = "Compare two saved snapshots and return the differences")]
    async fn diff_snapshots(&self, Parameters(req): Parameters<DiffSnapshotsRequest>) -> String {
        let snapshots = match self.snapshots.read() {
            Ok(s) => s,
            Err(_) => {
                return json!({
                    "error": "lock_error",
                    "message": "Failed to acquire snapshot lock"
                })
                .to_string();
            }
        };

        let json_a = match snapshots.get(&req.name_a) {
            Some(j) => j,
            None => {
                return json!({
                    "error": "not_found",
                    "message": format!("Snapshot '{}' not found", req.name_a)
                })
                .to_string();
            }
        };

        let json_b = match snapshots.get(&req.name_b) {
            Some(j) => j,
            None => {
                return json!({
                    "error": "not_found",
                    "message": format!("Snapshot '{}' not found", req.name_b)
                })
                .to_string();
            }
        };

        let tree_a: egui_mcp_protocol::UiTree = match serde_json::from_str(json_a) {
            Ok(t) => t,
            Err(e) => {
                return json!({
                    "error": "parse_error",
                    "message": format!("Failed to parse snapshot '{}': {}", req.name_a, e)
                })
                .to_string();
            }
        };

        let tree_b: egui_mcp_protocol::UiTree = match serde_json::from_str(json_b) {
            Ok(t) => t,
            Err(e) => {
                return json!({
                    "error": "parse_error",
                    "message": format!("Failed to parse snapshot '{}': {}", req.name_b, e)
                })
                .to_string();
            }
        };

        let diff = compute_tree_diff(&tree_a, &tree_b);
        json!({
            "name_a": req.name_a,
            "name_b": req.name_b,
            "diff": diff
        })
        .to_string()
    }

    /// Compare current UI state with a saved snapshot
    #[tool(description = "Compare current UI tree state with a saved snapshot")]
    async fn diff_current(&self, Parameters(req): Parameters<DiffCurrentRequest>) -> String {
        #[cfg(target_os = "linux")]
        {
            // Get saved snapshot
            let saved_json = {
                let snapshots = match self.snapshots.read() {
                    Ok(s) => s,
                    Err(_) => {
                        return json!({
                            "error": "lock_error",
                            "message": "Failed to acquire snapshot lock"
                        })
                        .to_string();
                    }
                };

                match snapshots.get(&req.name) {
                    Some(j) => j.clone(),
                    None => {
                        return json!({
                            "error": "not_found",
                            "message": format!("Snapshot '{}' not found", req.name)
                        })
                        .to_string();
                    }
                }
            };

            let saved_tree: egui_mcp_protocol::UiTree = match serde_json::from_str(&saved_json) {
                Ok(t) => t,
                Err(e) => {
                    return json!({
                        "error": "parse_error",
                        "message": format!("Failed to parse saved snapshot: {}", e)
                    })
                    .to_string();
                }
            };

            // Get current tree
            match atspi_client::get_ui_tree_blocking(&self.app_name) {
                Ok(Some(current_tree)) => {
                    let diff = compute_tree_diff(&saved_tree, &current_tree);
                    json!({
                        "snapshot_name": req.name,
                        "diff": diff
                    })
                    .to_string()
                }
                Ok(None) => json!({
                    "error": "no_tree",
                    "message": "No current UI tree available"
                })
                .to_string(),
                Err(e) => json!({
                    "error": "atspi_error",
                    "message": format!("Failed to get current UI tree: {}", e)
                })
                .to_string(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = req;
            json!({
                "error": "not_available",
                "message": "diff_current requires AT-SPI on Linux."
            })
            .to_string()
        }
    }

    // =========================================================================
    // 8.5 Console/Log Access
    // =========================================================================

    /// Get recent log entries from the egui application
    #[tool(
        description = "Get recent log entries from the egui application. Note: Requires the egui app to be configured with McpLogLayer."
    )]
    async fn get_logs(&self, Parameters(req): Parameters<GetLogsRequest>) -> String {
        match self.ipc_client.get_logs(req.level, req.limit).await {
            Ok(entries) => json!({
                "count": entries.len(),
                "entries": entries
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to get logs: {}", e)
            })
            .to_string(),
        }
    }

    /// Clear all log entries in the egui application
    #[tool(description = "Clear the log buffer in the egui application")]
    async fn clear_logs(&self) -> String {
        match self.ipc_client.clear_logs().await {
            Ok(()) => json!({
                "success": true,
                "message": "Log buffer cleared"
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to clear logs: {}", e)
            })
            .to_string(),
        }
    }

    // =========================================================================
    // 8.4 Performance Metrics
    // =========================================================================

    /// Get current frame statistics from the egui application
    #[tool(
        description = "Get current frame statistics (FPS, frame time) from the egui application. Note: Requires the egui app to call record_frame()."
    )]
    async fn get_frame_stats(&self) -> String {
        match self.ipc_client.get_frame_stats().await {
            Ok(stats) => json!({
                "fps": stats.fps,
                "frame_time_ms": stats.frame_time_ms,
                "frame_time_min_ms": stats.frame_time_min_ms,
                "frame_time_max_ms": stats.frame_time_max_ms,
                "sample_count": stats.sample_count
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to get frame stats: {}", e)
            })
            .to_string(),
        }
    }

    /// Start recording performance data
    #[tool(
        description = "Start recording performance data for later analysis. Call get_perf_report to stop and get results."
    )]
    async fn start_perf_recording(
        &self,
        Parameters(req): Parameters<StartPerfRecordingRequest>,
    ) -> String {
        let duration = req.duration_ms.unwrap_or(0);
        match self.ipc_client.start_perf_recording(duration).await {
            Ok(()) => json!({
                "success": true,
                "message": if duration > 0 {
                    format!("Recording started for {}ms", duration)
                } else {
                    "Recording started (call get_perf_report to stop)".to_string()
                }
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to start recording: {}", e)
            })
            .to_string(),
        }
    }

    /// Get performance report (stops recording)
    #[tool(
        description = "Stop performance recording and get the report with statistics including percentiles."
    )]
    async fn get_perf_report(&self) -> String {
        match self.ipc_client.get_perf_report().await {
            Ok(Some(report)) => json!({
                "duration_ms": report.duration_ms,
                "total_frames": report.total_frames,
                "avg_fps": report.avg_fps,
                "avg_frame_time_ms": report.avg_frame_time_ms,
                "min_frame_time_ms": report.min_frame_time_ms,
                "max_frame_time_ms": report.max_frame_time_ms,
                "p95_frame_time_ms": report.p95_frame_time_ms,
                "p99_frame_time_ms": report.p99_frame_time_ms
            })
            .to_string(),
            Ok(None) => json!({
                "error": "no_data",
                "message": "No performance recording active or no frames recorded"
            })
            .to_string(),
            Err(e) => json!({
                "error": "ipc_error",
                "message": format!("Failed to get performance report: {}", e)
            })
            .to_string(),
        }
    }
}

/// Compute the difference between two UI trees
fn compute_tree_diff(
    tree_a: &egui_mcp_protocol::UiTree,
    tree_b: &egui_mcp_protocol::UiTree,
) -> serde_json::Value {
    use std::collections::HashMap;

    let map_a: HashMap<u64, &egui_mcp_protocol::NodeInfo> =
        tree_a.nodes.iter().map(|n| (n.id, n)).collect();
    let map_b: HashMap<u64, &egui_mcp_protocol::NodeInfo> =
        tree_b.nodes.iter().map(|n| (n.id, n)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();

    // Find added nodes (in B but not in A)
    for (id, node) in &map_b {
        if !map_a.contains_key(id) {
            added.push(json!({
                "id": id,
                "role": node.role,
                "label": node.label
            }));
        }
    }

    // Find removed nodes (in A but not in B)
    for (id, node) in &map_a {
        if !map_b.contains_key(id) {
            removed.push(json!({
                "id": id,
                "role": node.role,
                "label": node.label
            }));
        }
    }

    // Find modified nodes (in both but different)
    for (id, node_a) in &map_a {
        if let Some(node_b) = map_b.get(id) {
            let mut changes = Vec::new();

            if node_a.role != node_b.role {
                changes.push(json!({
                    "field": "role",
                    "old": node_a.role,
                    "new": node_b.role
                }));
            }
            if node_a.label != node_b.label {
                changes.push(json!({
                    "field": "label",
                    "old": node_a.label,
                    "new": node_b.label
                }));
            }
            if node_a.value != node_b.value {
                changes.push(json!({
                    "field": "value",
                    "old": node_a.value,
                    "new": node_b.value
                }));
            }
            if node_a.toggled != node_b.toggled {
                changes.push(json!({
                    "field": "toggled",
                    "old": node_a.toggled,
                    "new": node_b.toggled
                }));
            }
            if node_a.disabled != node_b.disabled {
                changes.push(json!({
                    "field": "disabled",
                    "old": node_a.disabled,
                    "new": node_b.disabled
                }));
            }
            if node_a.focused != node_b.focused {
                changes.push(json!({
                    "field": "focused",
                    "old": node_a.focused,
                    "new": node_b.focused
                }));
            }

            if !changes.is_empty() {
                modified.push(json!({
                    "id": id,
                    "role": node_a.role,
                    "label": node_a.label,
                    "changes": changes
                }));
            }
        }
    }

    json!({
        "added_count": added.len(),
        "removed_count": removed.len(),
        "modified_count": modified.len(),
        "added": added,
        "removed": removed,
        "modified": modified
    })
}

/// Parse a hex color string to RGBA array
fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim_start_matches('#');
    match s.len() {
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b, 200]) // Default alpha
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
        _ => None,
    }
}

impl EguiMcpServer {
    /// Save base64-encoded PNG data to a temp file and return Content with file path
    fn save_screenshot_to_file(&self, data: &str) -> Content {
        use base64::Engine;

        match base64::engine::general_purpose::STANDARD.decode(data) {
            Ok(png_bytes) => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let file_path = format!("/tmp/egui-mcp-screenshot-{}.png", timestamp);

                match std::fs::write(&file_path, png_bytes.as_slice()) {
                    Ok(()) => Content::text(
                        json!({
                            "file_path": file_path,
                            "size_bytes": png_bytes.len()
                        })
                        .to_string(),
                    ),
                    Err(e) => Content::text(
                        json!({
                            "error": "file_write_error",
                            "message": format!("Failed to write screenshot file: {}", e)
                        })
                        .to_string(),
                    ),
                }
            }
            Err(e) => Content::text(
                json!({
                    "error": "decode_error",
                    "message": format!("Failed to decode base64 data: {}", e)
                })
                .to_string(),
            ),
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
                 'click_element' to click an element by ID (AT-SPI), \
                 'set_text' to input text into a text field by ID (AT-SPI), \
                 'click_at' to click at specific coordinates (IPC), \
                 'double_click' to double click at specific coordinates (IPC), \
                 'keyboard_input' to send keyboard input (IPC), \
                 'scroll' to scroll at specific coordinates (IPC), \
                 'hover' to move mouse to specific coordinates (IPC), \
                 'drag' to drag from one point to another (IPC), \
                 'take_screenshot' to capture the current UI (IPC), \
                 'drag_element' to drag an element to target coordinates (AT-SPI + IPC), \
                 'get_bounds' to get element bounding box (AT-SPI Component), \
                 'focus_element' to focus an element (AT-SPI Component), \
                 'scroll_to_element' to scroll element into view (AT-SPI Component), \
                 'get_value' to get slider/progress value (AT-SPI Value), \
                 'set_value' to set slider value (AT-SPI Value), \
                 'select_item' to select item in list/combo (AT-SPI Selection), \
                 'deselect_item' to deselect item (AT-SPI Selection), \
                 'get_selected_count' to count selected items (AT-SPI Selection), \
                 'select_all' to select all items (AT-SPI Selection), \
                 'clear_selection' to clear all selections (AT-SPI Selection), \
                 'get_text' to get text content (AT-SPI Text), \
                 'get_text_selection' to get selected text range (AT-SPI Text), \
                 'set_text_selection' to select text range (AT-SPI Text), \
                 'get_caret_position' to get cursor position (AT-SPI Text), \
                 'set_caret_position' to set cursor position (AT-SPI Text), \
                 'is_visible' to check if element is visible (AT-SPI State), \
                 'is_enabled' to check if element is enabled (AT-SPI State), \
                 'is_focused' to check if element is focused (AT-SPI State), \
                 'is_checked' to check if element is checked/pressed (AT-SPI State), \
                 'screenshot_element' to capture a specific element (AT-SPI + IPC), \
                 'screenshot_region' to capture a specific region (IPC), \
                 'wait_for_element' to wait for element to appear/disappear (AT-SPI), \
                 'wait_for_state' to wait for element state change (AT-SPI), \
                 'compare_screenshots' to compare two screenshots and get similarity score, \
                 'diff_screenshots' to generate a visual diff image highlighting differences, \
                 'highlight_element' to draw a colored highlight on an element (AT-SPI + IPC), and \
                 'clear_highlights' to remove all highlights (IPC)."
                    .into(),
            ),
        }
    }
}

fn print_guide() {
    let version = env!("CARGO_PKG_VERSION");
    print!(
        r#"
================================================================================
                        egui-mcp-server Setup Guide
                              Version {version}
================================================================================

This guide explains how to set up egui-mcp for UI automation with MCP clients
like Claude Code.

--------------------------------------------------------------------------------
STEP 1: Add egui-mcp-client to your egui application
--------------------------------------------------------------------------------

Add the dependency to your Cargo.toml:

    [dependencies]
    egui-mcp-client = "{version}"

Initialize the client in your app:

    use egui_mcp_client::McpClient;

    struct MyApp {{
        mcp_client: McpClient,
        // ... your other fields
    }}

    impl MyApp {{
        fn new(cc: &eframe::CreationContext<'_>) -> Self {{
            Self {{
                mcp_client: McpClient::new(cc),
                // ...
            }}
        }}
    }}

    impl eframe::App for MyApp {{
        fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {{
            // Call this at the end of your update function
            self.mcp_client.update(ctx, frame);

            // ... your UI code
        }}
    }}

--------------------------------------------------------------------------------
STEP 2: Configure MCP client (e.g., Claude Code)
--------------------------------------------------------------------------------

Create or edit `.mcp.json` in your project root:

    {{
      "mcpServers": {{
        "egui-mcp": {{
          "command": "egui-mcp-server",
          "args": [],
          "env": {{
            "XDG_RUNTIME_DIR": "/mnt/wslg/runtime-dir",
            "EGUI_MCP_APP_NAME": "your-app-name"
          }}
        }}
      }}
    }}

Replace "your-app-name" with your egui application's window title.

For cargo-based development, use:

    {{
      "mcpServers": {{
        "egui-mcp": {{
          "command": "cargo",
          "args": ["run", "-p", "egui-mcp-server"],
          "env": {{
            "XDG_RUNTIME_DIR": "/mnt/wslg/runtime-dir",
            "EGUI_MCP_APP_NAME": "your-app-name"
          }}
        }}
      }}
    }}

--------------------------------------------------------------------------------
STEP 3: Run your application
--------------------------------------------------------------------------------

1. Start your egui application (with egui-mcp-client integrated)
2. The MCP server will automatically connect when Claude Code starts
3. Use natural language to interact with your UI:
   - "Click the Submit button"
   - "Type 'hello' in the text field"
   - "Take a screenshot"
   - "Find all buttons on the screen"

--------------------------------------------------------------------------------
ENVIRONMENT VARIABLES
--------------------------------------------------------------------------------

  EGUI_MCP_APP_NAME    (Required) Target application's window title
  XDG_RUNTIME_DIR      Runtime directory for IPC socket (WSL: /mnt/wslg/runtime-dir)
  RUST_LOG             Log level (e.g., "info", "debug")

--------------------------------------------------------------------------------
AVAILABLE MCP TOOLS
--------------------------------------------------------------------------------

UI Tree & Search:
  - get_ui_tree       Get the full accessibility tree
  - find_by_label     Search elements by label (substring match)
  - find_by_role      Search elements by role (Button, TextInput, etc.)
  - get_element       Get detailed info about a specific element

Interaction:
  - click_element     Click an element by ID
  - click_at          Click at specific coordinates
  - set_text          Set text in a text input
  - keyboard_input    Send keyboard input
  - scroll            Scroll at a position
  - hover             Move mouse to position
  - drag              Drag from one position to another

Screenshots:
  - take_screenshot   Capture the application window
  - compare_screenshots  Compare two screenshots for similarity
  - diff_screenshots     Generate visual diff between screenshots

For more information, visit: https://github.com/dijdzv/egui-mcp

================================================================================
"#
    );
}

async fn run_server() -> Result<()> {
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

    // Get application name from environment variable
    let app_name = std::env::var("EGUI_MCP_APP_NAME").map_err(|_| {
        anyhow::anyhow!(
            "EGUI_MCP_APP_NAME environment variable not set. \
             Please set it in .mcp.json env section. \
             Run 'egui-mcp-server guide' for setup instructions."
        )
    })?;

    tracing::info!("Target application: {}", app_name);

    // Create and run the server
    let server = EguiMcpServer::new(app_name);
    let service = server.serve(stdio()).await?;

    tracing::info!("Server started, waiting for connections...");
    service.waiting().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Guide => {
            print_guide();
            Ok(())
        }
        Commands::Serve => run_server().await,
    }
}
