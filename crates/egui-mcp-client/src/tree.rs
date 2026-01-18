//! UI tree building utilities

use accesskit::{Node, NodeId, Role, Toggled, TreeUpdate};
use egui_mcp_protocol::{NodeInfo, Rect, UiTree};
use std::collections::HashMap;

/// Builder for converting AccessKit trees to our UiTree format
pub struct UiTreeBuilder;

impl UiTreeBuilder {
    /// Build a UiTree from an AccessKit TreeUpdate
    pub fn from_accesskit(update: &TreeUpdate) -> UiTree {
        let mut nodes = Vec::new();
        let mut id_map: HashMap<NodeId, u64> = HashMap::new();

        // First pass: assign sequential IDs
        for (idx, (node_id, _)) in update.nodes.iter().enumerate() {
            id_map.insert(*node_id, idx as u64);
        }

        // Second pass: build NodeInfo for each node
        for (node_id, node) in &update.nodes {
            let id = id_map.get(node_id).copied().unwrap_or(0);
            let node_info = Self::convert_node(id, node, &id_map);
            nodes.push(node_info);
        }

        // Get root IDs
        let roots = if let Some(tree) = &update.tree {
            vec![id_map.get(&tree.root).copied().unwrap_or(0)]
        } else {
            Vec::new()
        };

        UiTree { roots, nodes }
    }

    fn convert_node(id: u64, node: &Node, id_map: &HashMap<NodeId, u64>) -> NodeInfo {
        let role = Self::role_to_string(node.role());

        // Get label from the node's label or name property
        let label = node.label().map(|s| s.to_string());

        // Get value from the node
        let value = node.value().map(|s| s.to_string());

        let bounds = node.bounds().map(|b| Rect {
            x: b.x0 as f32,
            y: b.y0 as f32,
            width: (b.x1 - b.x0) as f32,
            height: (b.y1 - b.y0) as f32,
        });

        let children: Vec<u64> = node
            .children()
            .iter()
            .filter_map(|child_id| id_map.get(child_id).copied())
            .collect();

        // Convert Toggled enum to Option<bool>
        let toggled = node.toggled().map(|t| match t {
            Toggled::True => true,
            Toggled::False => false,
            Toggled::Mixed => false, // Treat mixed as false
        });

        let disabled = node.is_disabled();

        // Check if node has focus by looking at the focused state
        let focused = false; // Will be updated from tree focus

        NodeInfo {
            id,
            role,
            label,
            value,
            bounds,
            children,
            toggled,
            disabled,
            focused,
        }
    }

    fn role_to_string(role: Role) -> String {
        match role {
            Role::Unknown => "Unknown",
            Role::Button => "Button",
            Role::CheckBox => "CheckBox",
            Role::ComboBox => "ComboBox",
            Role::Dialog => "Dialog",
            Role::GenericContainer => "GenericContainer",
            Role::Group => "Group",
            Role::Image => "Image",
            Role::Label => "Label",
            Role::Link => "Link",
            Role::List => "List",
            Role::ListItem => "ListItem",
            Role::Menu => "Menu",
            Role::MenuBar => "MenuBar",
            Role::MenuItem => "MenuItem",
            Role::ProgressIndicator => "ProgressIndicator",
            Role::RadioButton => "RadioButton",
            Role::ScrollBar => "ScrollBar",
            Role::ScrollView => "ScrollView",
            Role::Slider => "Slider",
            Role::SpinButton => "SpinButton",
            Role::Tab => "Tab",
            Role::TabList => "TabList",
            Role::TabPanel => "TabPanel",
            Role::TextInput => "TextInput",
            Role::Toolbar => "Toolbar",
            Role::Tooltip => "Tooltip",
            Role::Tree => "Tree",
            Role::TreeItem => "TreeItem",
            Role::Window => "Window",
            _ => "Other",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let update = TreeUpdate {
            nodes: vec![],
            tree: None,
            focus: accesskit::NodeId(0),
        };
        let tree = UiTreeBuilder::from_accesskit(&update);
        assert!(tree.nodes.is_empty());
        assert!(tree.roots.is_empty());
    }
}
