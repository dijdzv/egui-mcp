//! UI tree tool implementations (get_ui_tree, find_by_label, find_by_role, get_element)

use super::{ToolResult, error_response, parse_element_id};
use serde_json::json;

#[cfg(target_os = "linux")]
use crate::atspi_client::AtspiClient;

/// Get the UI tree from the connected egui application via AT-SPI
pub async fn get_ui_tree(app_name: &str) -> ToolResult {
    #[cfg(target_os = "linux")]
    {
        let client = match AtspiClient::new().await {
            Ok(c) => c,
            Err(e) => return super::atspi_connection_error(e),
        };

        match client.get_ui_tree_by_app_name(app_name).await {
            Ok(Some(tree)) => {
                return serde_json::to_string_pretty(&tree).unwrap_or_else(|e| {
                    error_response(
                        "serialization_error",
                        format!("Failed to serialize UI tree: {}", e),
                    )
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

    #[cfg(not(target_os = "linux"))]
    let _ = app_name;

    error_response(
        "not_available",
        "UI tree access requires AT-SPI on Linux. Make sure the egui app is running.",
    )
}

/// Find UI elements by their label text (substring match)
pub async fn find_by_label(app_name: &str, pattern: &str, exact: bool) -> ToolResult {
    #[cfg(target_os = "linux")]
    {
        let client = match AtspiClient::new().await {
            Ok(c) => c,
            Err(e) => return super::atspi_connection_error(e),
        };

        match client.find_by_label(app_name, pattern, exact).await {
            Ok(elements) => {
                return serde_json::to_string_pretty(&json!({
                    "count": elements.len(),
                    "elements": elements
                }))
                .unwrap_or_else(|e| {
                    error_response(
                        "serialization_error",
                        format!("Failed to serialize elements: {}", e),
                    )
                });
            }
            Err(e) => {
                tracing::warn!("AT-SPI find_by_label failed: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (app_name, pattern, exact);

    error_response("not_available", "Element search requires AT-SPI on Linux.")
}

/// Find UI elements by their role
pub async fn find_by_role(app_name: &str, role: &str) -> ToolResult {
    #[cfg(target_os = "linux")]
    {
        let client = match AtspiClient::new().await {
            Ok(c) => c,
            Err(e) => return super::atspi_connection_error(e),
        };

        match client.find_by_role(app_name, role).await {
            Ok(elements) => {
                return serde_json::to_string_pretty(&json!({
                    "count": elements.len(),
                    "elements": elements
                }))
                .unwrap_or_else(|e| {
                    error_response(
                        "serialization_error",
                        format!("Failed to serialize elements: {}", e),
                    )
                });
            }
            Err(e) => {
                tracing::warn!("AT-SPI find_by_role failed: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (app_name, role);

    error_response("not_available", "Element search requires AT-SPI on Linux.")
}

/// Get detailed information about a specific UI element by its ID
pub async fn get_element(app_name: &str, id_str: &str) -> ToolResult {
    let id = match parse_element_id(id_str) {
        Ok(id) => id,
        Err(e) => return e,
    };

    #[cfg(target_os = "linux")]
    {
        let client = match AtspiClient::new().await {
            Ok(c) => c,
            Err(e) => return super::atspi_connection_error(e),
        };

        match client.get_element(app_name, id).await {
            Ok(Some(element)) => {
                return serde_json::to_string_pretty(&element).unwrap_or_else(|e| {
                    error_response(
                        "serialization_error",
                        format!("Failed to serialize element: {}", e),
                    )
                });
            }
            Ok(None) => {
                return error_response("not_found", format!("Element with ID {} not found", id));
            }
            Err(e) => {
                tracing::warn!("AT-SPI get_element failed: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (app_name, id);

    error_response(
        "not_available",
        "Element retrieval requires AT-SPI on Linux.",
    )
}
