//! UI Automation client for accessing accessibility information on Windows
//!
//! This module provides async functions to interact with accessible applications
//! via the Windows UI Automation API.

use egui_mcp_protocol::{NodeInfo, Rect, UiTree};
use uiautomation::UIAutomation;

/// Boxed error type for UI Automation operations
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Value information returned from UI Automation Value pattern
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValueInfo {
    pub current: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub increment: f64,
}

/// Text information returned from UI Automation Text pattern
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextInfo {
    pub text: String,
    pub length: i32,
    pub caret_offset: i32,
}

/// Text selection range
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextSelection {
    pub start: i32,
    pub end: i32,
}

/// UI Automation client for communicating with accessible applications
pub struct UiaClient {
    automation: UIAutomation,
}

impl UiaClient {
    /// Create a new UI Automation client
    pub fn new() -> Result<Self, BoxError> {
        let automation = UIAutomation::new()
            .map_err(|e| format!("Failed to initialize UI Automation: {}", e))?;
        Ok(Self { automation })
    }

    /// Get the UI tree for a specific application by name
    pub fn get_ui_tree_by_app_name(&self, app_name: &str) -> Result<Option<UiTree>, BoxError> {
        let root = self
            .automation
            .get_root_element()
            .map_err(|e| format!("Failed to get root element: {}", e))?;

        // Find the application window by name
        let matcher = self
            .automation
            .create_matcher()
            .name(app_name)
            .timeout(1000);
        let app_window = match root.find_first(matcher) {
            Ok(window) => window,
            Err(_) => return Ok(None),
        };

        self.build_ui_tree_from_element(&app_window)
    }

    /// Build a UiTree from a UI Automation element
    fn build_ui_tree_from_element(
        &self,
        root: &uiautomation::UIElement,
    ) -> Result<Option<UiTree>, BoxError> {
        let mut nodes: Vec<NodeInfo> = Vec::new();
        let mut roots: Vec<u64> = Vec::new();

        // Use the runtime ID as the node ID
        let root_id = self.get_element_id(root)?;
        roots.push(root_id);

        self.traverse_tree(root, &mut nodes)?;

        if nodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(UiTree { nodes, roots }))
    }

    /// Get a unique ID for an element
    fn get_element_id(&self, element: &uiautomation::UIElement) -> Result<u64, BoxError> {
        // Use the runtime ID hash as a unique identifier
        let runtime_id = element
            .get_runtime_id()
            .map_err(|e| format!("Failed to get runtime ID: {}", e))?;
        // Convert runtime ID to a single u64 hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        runtime_id.hash(&mut hasher);
        Ok(hasher.finish())
    }

    /// Recursively traverse the UI tree
    fn traverse_tree(
        &self,
        element: &uiautomation::UIElement,
        nodes: &mut Vec<NodeInfo>,
    ) -> Result<(), BoxError> {
        let node_id = self.get_element_id(element)?;

        // Get element properties
        let name = element.get_name().unwrap_or_default();
        let control_type = element
            .get_control_type()
            .map(|ct| format!("{:?}", ct))
            .unwrap_or_else(|_| "Unknown".to_string());

        // Get bounds
        let bounds = element.get_bounding_rectangle().ok().map(|rect| Rect {
            x: rect.get_left() as f32,
            y: rect.get_top() as f32,
            width: rect.get_width() as f32,
            height: rect.get_height() as f32,
        });

        // Get enabled state
        let disabled = !element.get_is_enabled().unwrap_or(true);

        // Get focused state
        let focused = element.get_has_keyboard_focus().unwrap_or(false);

        // Get toggle state if available
        let toggled = element
            .get_toggle_state()
            .ok()
            .map(|state| matches!(state, uiautomation::types::ToggleState::On));

        // Get children
        let children_elements = element.find_all(
            self.automation
                .create_matcher()
                .from(uiautomation::types::TreeScope::Children),
        );
        let mut child_ids: Vec<u64> = Vec::new();

        if let Ok(children) = children_elements {
            for child in children.iter() {
                let child_id = self.get_element_id(&child)?;
                child_ids.push(child_id);
                Box::pin(async { self.traverse_tree(&child, nodes) }).await??;
            }
        }

        let label = if !name.is_empty() { Some(name) } else { None };

        let node_info = NodeInfo {
            id: node_id,
            role: control_type,
            label,
            value: None,
            children: child_ids,
            bounds,
            toggled,
            disabled,
            focused,
        };

        nodes.push(node_info);
        Ok(())
    }

    /// Click an element using UI Automation Invoke pattern
    pub fn click_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Err(format!("Element with id {} not found in '{}'", id, app_name).into());
        };

        // Try Invoke pattern first
        if let Ok(invoke) = element.get_invoke_pattern() {
            invoke
                .invoke()
                .map_err(|e| format!("Failed to invoke element: {}", e))?;
            return Ok(true);
        }

        // Fall back to click via bounds
        if let Ok(rect) = element.get_bounding_rectangle() {
            let center_x = rect.get_left() + rect.get_width() / 2;
            let center_y = rect.get_top() + rect.get_height() / 2;
            // Note: Direct mouse click would require additional implementation
            tracing::warn!("Click via bounds not implemented, use IPC for coordinates");
        }

        Ok(false)
    }

    /// Set text content using UI Automation Value pattern
    pub fn set_text(&self, app_name: &str, id: u64, text: &str) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Err(format!("Element with id {} not found in '{}'", id, app_name).into());
        };

        if let Ok(value_pattern) = element.get_value_pattern() {
            value_pattern
                .set_value(text)
                .map_err(|e| format!("Failed to set value: {}", e))?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Find an element by ID within an application
    fn find_element_by_id(
        &self,
        app_name: &str,
        target_id: u64,
    ) -> Result<Option<uiautomation::UIElement>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name)?;
        let Some(_tree) = tree else {
            return Ok(None);
        };

        // This is a simplified implementation
        // In practice, we would need to maintain a mapping of IDs to elements
        // or search the tree to find the element
        let root = self.automation.get_root_element()?;
        let matcher = self
            .automation
            .create_matcher()
            .name(app_name)
            .timeout(1000);

        if let Ok(app_window) = root.find_first(matcher) {
            return self.find_element_in_tree(&app_window, target_id);
        }

        Ok(None)
    }

    /// Recursively search for an element by ID
    fn find_element_in_tree(
        &self,
        element: &uiautomation::UIElement,
        target_id: u64,
    ) -> Result<Option<uiautomation::UIElement>, BoxError> {
        let element_id = self.get_element_id(element)?;
        if element_id == target_id {
            return Ok(Some(element.clone()));
        }

        let children = element.find_all(
            self.automation
                .create_matcher()
                .from(uiautomation::types::TreeScope::Children),
        );

        if let Ok(children) = children {
            for child in children.iter() {
                if let Ok(Some(found)) = self.find_element_in_tree(&child, target_id) {
                    return Ok(Some(found));
                }
            }
        }

        Ok(None)
    }

    /// Find UI elements by label (exact or substring match)
    pub fn find_by_label(
        &self,
        app_name: &str,
        pattern: &str,
        exact: bool,
    ) -> Result<Vec<NodeInfo>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name)?;
        let Some(tree) = tree else {
            return Ok(vec![]);
        };

        let results: Vec<NodeInfo> = tree
            .nodes
            .iter()
            .filter(|node| {
                if let Some(label) = &node.label {
                    if exact {
                        label == pattern
                    } else {
                        label.contains(pattern)
                    }
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Find UI elements by role
    pub fn find_by_role(&self, app_name: &str, role: &str) -> Result<Vec<NodeInfo>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name)?;
        let Some(tree) = tree else {
            return Ok(vec![]);
        };

        let results: Vec<NodeInfo> = tree
            .nodes
            .iter()
            .filter(|node| node.role.to_lowercase().contains(&role.to_lowercase()))
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get element by ID
    pub fn get_element(&self, app_name: &str, id: u64) -> Result<Option<NodeInfo>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name)?;
        let Some(tree) = tree else {
            return Ok(None);
        };

        Ok(tree.nodes.into_iter().find(|node| node.id == id))
    }

    /// Get element bounds
    pub fn get_bounds(&self, app_name: &str, id: u64) -> Result<Option<Rect>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        let rect = element.get_bounding_rectangle().ok();
        Ok(rect.map(|r| Rect {
            x: r.get_left() as f32,
            y: r.get_top() as f32,
            width: r.get_width() as f32,
            height: r.get_height() as f32,
        }))
    }

    /// Focus an element
    pub fn focus_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        element
            .set_focus()
            .map_err(|e| format!("Failed to set focus: {}", e))?;
        Ok(true)
    }

    /// Get value information
    pub fn get_value(&self, app_name: &str, id: u64) -> Result<Option<ValueInfo>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        if let Ok(range_value) = element.get_range_value_pattern() {
            let current = range_value.get_value().unwrap_or(0.0);
            let minimum = range_value.get_minimum().unwrap_or(0.0);
            let maximum = range_value.get_maximum().unwrap_or(100.0);
            let increment = range_value.get_small_change().unwrap_or(1.0);

            return Ok(Some(ValueInfo {
                current,
                minimum,
                maximum,
                increment,
            }));
        }

        Ok(None)
    }

    /// Set value
    pub fn set_value(&self, app_name: &str, id: u64, value: f64) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        if let Ok(range_value) = element.get_range_value_pattern() {
            range_value
                .set_value(value)
                .map_err(|e| format!("Failed to set value: {}", e))?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Check if element is visible
    pub fn is_visible(&self, app_name: &str, id: u64) -> Result<Option<bool>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        let is_offscreen = element.get_is_offscreen().unwrap_or(true);
        Ok(Some(!is_offscreen))
    }

    /// Check if element is enabled
    pub fn is_enabled(&self, app_name: &str, id: u64) -> Result<Option<bool>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        Ok(element.get_is_enabled().ok())
    }

    /// Check if element is focused
    pub fn is_focused(&self, app_name: &str, id: u64) -> Result<Option<bool>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        Ok(element.get_has_keyboard_focus().ok())
    }

    /// Check if element is checked/toggled
    pub fn is_checked(&self, app_name: &str, id: u64) -> Result<Option<bool>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        if let Ok(toggle_state) = element.get_toggle_state() {
            return Ok(Some(matches!(
                toggle_state,
                uiautomation::types::ToggleState::On
            )));
        }

        Ok(None)
    }

    /// Get text content
    pub fn get_text(&self, app_name: &str, id: u64) -> Result<Option<TextInfo>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        if let Ok(text_pattern) = element.get_text_pattern() {
            if let Ok(document_range) = text_pattern.get_document_range() {
                let text = document_range.get_text(-1).unwrap_or_default();
                let length = text.len() as i32;
                return Ok(Some(TextInfo {
                    text,
                    length,
                    caret_offset: -1, // Caret position not easily available in UIA
                }));
            }
        }

        // Fall back to Value pattern
        if let Ok(value_pattern) = element.get_value_pattern() {
            let text = value_pattern.get_value().unwrap_or_default();
            let length = text.len() as i32;
            return Ok(Some(TextInfo {
                text,
                length,
                caret_offset: -1,
            }));
        }

        Ok(None)
    }

    /// Scroll element into view
    pub fn scroll_to_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        if let Ok(scroll_item) = element.get_scroll_item_pattern() {
            scroll_item
                .scroll_into_view()
                .map_err(|e| format!("Failed to scroll into view: {}", e))?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Select an item in a selection container
    pub fn select_item(&self, app_name: &str, id: u64, index: i32) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        // Find child at index and select it
        let children = element.find_all(
            self.automation
                .create_matcher()
                .from(uiautomation::types::TreeScope::Children),
        )?;

        if let Some(child) = children.iter().nth(index as usize) {
            if let Ok(selection_item) = child.get_selection_item_pattern() {
                selection_item
                    .select()
                    .map_err(|e| format!("Failed to select item: {}", e))?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get text selection
    pub fn get_text_selection(
        &self,
        app_name: &str,
        id: u64,
    ) -> Result<Option<TextSelection>, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(None);
        };

        if let Ok(text_pattern) = element.get_text_pattern() {
            if let Ok(selections) = text_pattern.get_selection() {
                if let Some(selection) = selections.iter().next() {
                    // Getting exact offsets requires more complex handling
                    // This is a simplified implementation
                    let text = selection.get_text(-1).unwrap_or_default();
                    if !text.is_empty() {
                        return Ok(Some(TextSelection {
                            start: 0,
                            end: text.len() as i32,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Set text selection
    pub fn set_text_selection(
        &self,
        app_name: &str,
        id: u64,
        start: i32,
        end: i32,
    ) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        if let Ok(text_pattern) = element.get_text_pattern() {
            if let Ok(document_range) = text_pattern.get_document_range() {
                // Move to start position and extend to end
                // This is a simplified implementation
                if let Ok(range) = document_range.clone_range() {
                    range
                        .select()
                        .map_err(|e| format!("Failed to select text range: {}", e))?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get caret position
    pub fn get_caret_position(&self, app_name: &str, id: u64) -> Result<i32, BoxError> {
        // Caret position is not directly available in UI Automation
        // This would require tracking the text cursor position separately
        let _ = (app_name, id);
        Ok(-1)
    }

    /// Set caret position
    pub fn set_caret_position(
        &self,
        app_name: &str,
        id: u64,
        offset: i32,
    ) -> Result<bool, BoxError> {
        let element = self.find_element_by_id(app_name, id)?;
        let Some(element) = element else {
            return Ok(false);
        };

        if let Ok(text_pattern) = element.get_text_pattern() {
            if let Ok(document_range) = text_pattern.get_document_range() {
                if let Ok(range) = document_range.clone_range() {
                    // Move range to offset position
                    // This is a simplified implementation
                    let _ = offset;
                    range.select()?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
