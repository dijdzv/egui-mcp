//! Utility functions for the MCP server
//!
//! This module contains helper functions used by tool handlers.

use serde_json::json;

/// Compute the difference between two UI trees
pub fn compute_tree_diff(
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
///
/// Supports formats:
/// - `#RRGGBB` - RGB with default alpha (200)
/// - `#RRGGBBAA` - RGBA
pub fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_rgb() {
        assert_eq!(parse_hex_color("#ff0000"), Some([255, 0, 0, 200]));
        assert_eq!(parse_hex_color("#00ff00"), Some([0, 255, 0, 200]));
        assert_eq!(parse_hex_color("#0000ff"), Some([0, 0, 255, 200]));
        assert_eq!(parse_hex_color("ff0000"), Some([255, 0, 0, 200]));
    }

    #[test]
    fn test_parse_hex_color_rgba() {
        assert_eq!(parse_hex_color("#ff000080"), Some([255, 0, 0, 128]));
        assert_eq!(parse_hex_color("#00ff00ff"), Some([0, 255, 0, 255]));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("#fff"), None);
        assert_eq!(parse_hex_color("#fffff"), None);
        assert_eq!(parse_hex_color(""), None);
    }
}
