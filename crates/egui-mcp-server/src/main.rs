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
            match atspi_client::get_bounds_blocking("demo", id) {
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
            match atspi_client::get_bounds_blocking("demo", id) {
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
            match atspi_client::focus_element_blocking("demo", id) {
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
            match atspi_client::scroll_to_element_blocking("demo", id) {
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
            match atspi_client::get_value_blocking("demo", id) {
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
            match atspi_client::set_value_blocking("demo", id, value) {
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
            match atspi_client::select_item_blocking("demo", id, index) {
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
            match atspi_client::deselect_item_blocking("demo", id, index) {
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
            match atspi_client::get_selected_count_blocking("demo", id) {
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
            match atspi_client::select_all_blocking("demo", id) {
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
            match atspi_client::clear_selection_blocking("demo", id) {
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
            match atspi_client::get_text_blocking("demo", id) {
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
            match atspi_client::get_text_selection_blocking("demo", id) {
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
            match atspi_client::set_text_selection_blocking("demo", id, start, end) {
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
            match atspi_client::get_caret_position_blocking("demo", id) {
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
            match atspi_client::set_caret_position_blocking("demo", id, offset) {
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
                 'get_caret_position' to get cursor position (AT-SPI Text), and \
                 'set_caret_position' to set cursor position (AT-SPI Text)."
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
