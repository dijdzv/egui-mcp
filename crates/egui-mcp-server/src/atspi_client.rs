//! AT-SPI client for accessing accessibility information on Linux

use atspi_common::{CoordType, ObjectRef, ScrollType};
use atspi_connection::AccessibilityConnection;
use atspi_proxies::accessible::{AccessibleProxy, ObjectRefExt};
use egui_mcp_protocol::{NodeInfo, Rect, UiTree};
use std::thread;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Get UI tree for a specific application using AT-SPI
/// This function runs in a separate thread with async-std runtime
pub fn get_ui_tree_blocking(app_name: &str) -> Result<Option<UiTree>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_ui_tree_by_app_name(&app_name).await
        })
    });
    handle.join().unwrap()
}

/// Find UI elements by label using AT-SPI
pub fn find_by_label_blocking(
    app_name: &str,
    pattern: &str,
    exact: bool,
) -> Result<Vec<NodeInfo>, BoxError> {
    let app_name = app_name.to_string();
    let pattern = pattern.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.find_by_label(&app_name, &pattern, exact).await
        })
    });
    handle.join().unwrap()
}

/// Find UI elements by role using AT-SPI
pub fn find_by_role_blocking(app_name: &str, role: &str) -> Result<Vec<NodeInfo>, BoxError> {
    let app_name = app_name.to_string();
    let role = role.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.find_by_role(&app_name, &role).await
        })
    });
    handle.join().unwrap()
}

/// Get a specific element by ID using AT-SPI
pub fn get_element_blocking(app_name: &str, id: u64) -> Result<Option<NodeInfo>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            let tree = client.get_ui_tree_by_app_name(&app_name).await?;
            if let Some(tree) = tree {
                Ok(tree.nodes.into_iter().find(|n| n.id == id))
            } else {
                Ok(None)
            }
        })
    });
    handle.join().unwrap()
}

/// Click an element by ID using AT-SPI Action interface
pub fn click_element_blocking(app_name: &str, id: u64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.click_element(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Set text content of an element by ID using AT-SPI EditableText interface
pub fn set_text_blocking(app_name: &str, id: u64, text: &str) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let text = text.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.set_text(&app_name, id, &text).await
        })
    });
    handle.join().unwrap()
}

// ============================================================================
// Priority 2: Element Information (AT-SPI Component)
// ============================================================================

/// Get element bounds (bounding box) using AT-SPI Component interface
pub fn get_bounds_blocking(app_name: &str, id: u64) -> Result<Option<Rect>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_bounds(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Focus an element by ID using AT-SPI Component interface
pub fn focus_element_blocking(app_name: &str, id: u64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.focus_element(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Scroll element into view using AT-SPI Component interface
pub fn scroll_to_element_blocking(app_name: &str, id: u64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.scroll_to_element(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

// ============================================================================
// Priority 3: Value Operations (AT-SPI Value)
// ============================================================================

/// Value information returned from AT-SPI
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValueInfo {
    pub current: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub increment: f64,
}

/// Get value of an element using AT-SPI Value interface
pub fn get_value_blocking(app_name: &str, id: u64) -> Result<Option<ValueInfo>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_value(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Set value of an element using AT-SPI Value interface
pub fn set_value_blocking(app_name: &str, id: u64, value: f64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.set_value(&app_name, id, value).await
        })
    });
    handle.join().unwrap()
}

// ============================================================================
// Priority 4: Selection Operations (AT-SPI Selection)
// ============================================================================

/// Select an item by index using AT-SPI Selection interface
pub fn select_item_blocking(app_name: &str, id: u64, index: i32) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.select_item(&app_name, id, index).await
        })
    });
    handle.join().unwrap()
}

/// Deselect an item by index using AT-SPI Selection interface
pub fn deselect_item_blocking(app_name: &str, id: u64, index: i32) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.deselect_item(&app_name, id, index).await
        })
    });
    handle.join().unwrap()
}

/// Get count of selected items using AT-SPI Selection interface
pub fn get_selected_count_blocking(app_name: &str, id: u64) -> Result<i32, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_selected_count(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Select all items using AT-SPI Selection interface
pub fn select_all_blocking(app_name: &str, id: u64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.select_all(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Clear all selections using AT-SPI Selection interface
pub fn clear_selection_blocking(app_name: &str, id: u64) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.clear_selection(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

// ============================================================================
// Priority 5: Text Operations (AT-SPI Text)
// ============================================================================

/// Text information returned from AT-SPI
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextInfo {
    pub text: String,
    pub length: i32,
    pub caret_offset: i32,
}

/// Text selection information
#[derive(Debug, Clone, serde::Serialize)]
pub struct TextSelection {
    pub start: i32,
    pub end: i32,
}

/// Get text content using AT-SPI Text interface
pub fn get_text_blocking(app_name: &str, id: u64) -> Result<Option<TextInfo>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_text(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Get text selection range using AT-SPI Text interface
pub fn get_text_selection_blocking(
    app_name: &str,
    id: u64,
) -> Result<Option<TextSelection>, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_text_selection(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Set text selection range using AT-SPI Text interface
pub fn set_text_selection_blocking(
    app_name: &str,
    id: u64,
    start: i32,
    end: i32,
) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.set_text_selection(&app_name, id, start, end).await
        })
    });
    handle.join().unwrap()
}

/// Get caret (cursor) position using AT-SPI Text interface
pub fn get_caret_position_blocking(app_name: &str, id: u64) -> Result<i32, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.get_caret_position(&app_name, id).await
        })
    });
    handle.join().unwrap()
}

/// Set caret (cursor) position using AT-SPI Text interface
pub fn set_caret_position_blocking(app_name: &str, id: u64, offset: i32) -> Result<bool, BoxError> {
    let app_name = app_name.to_string();
    let handle = thread::spawn(move || {
        async_std::task::block_on(async {
            let client = AtspiClient::new().await?;
            client.set_caret_position(&app_name, id, offset).await
        })
    });
    handle.join().unwrap()
}

/// AT-SPI client for communicating with accessible applications
pub struct AtspiClient {
    connection: AccessibilityConnection,
}

/// Extract the actual AT-SPI node ID from an ObjectRef path
/// The path format is like "/org/a11y/atspi/accessible/0/4467407273966801439"
/// We want to extract "4467407273966801439" as a u64
fn extract_atspi_node_id(path: &str) -> Option<u64> {
    path.rsplit('/').next().and_then(|s| s.parse().ok())
}

impl AtspiClient {
    /// Create a new AT-SPI client
    pub async fn new() -> Result<Self, BoxError> {
        let connection = AccessibilityConnection::new().await?;
        Ok(Self { connection })
    }

    /// Get the UI tree for a specific application by name
    pub async fn get_ui_tree_by_app_name(
        &self,
        app_name: &str,
    ) -> Result<Option<UiTree>, BoxError> {
        let app_ref = self.find_app_ref_by_name(app_name).await?;
        let Some(app_ref) = app_ref else {
            return Ok(None);
        };
        let app_proxy = app_ref
            .as_accessible_proxy(self.connection.connection())
            .await?;
        self.build_ui_tree_from_proxy(&app_proxy).await
    }

    /// Find an application ObjectRef by name
    async fn find_app_ref_by_name(&self, app_name: &str) -> Result<Option<ObjectRef>, BoxError> {
        let registry_proxy: AccessibleProxy<'_> =
            AccessibleProxy::builder(self.connection.connection())
                .destination("org.a11y.atspi.Registry")?
                .path("/org/a11y/atspi/accessible/root")?
                .build()
                .await?;

        let apps: Vec<ObjectRef> = registry_proxy.get_children().await?;

        for app_ref in apps {
            let app_proxy: AccessibleProxy<'_> = app_ref
                .as_accessible_proxy(self.connection.connection())
                .await?;
            let name: String = app_proxy.name().await.unwrap_or_default();

            if name.contains(app_name) {
                tracing::info!("Found application: {}", name);
                return Ok(Some(app_ref));
            }
        }

        Ok(None)
    }

    /// Find element info (destination and path) by ID within an application
    /// The ID is the actual AT-SPI node ID extracted from the object path
    async fn find_element_path_by_id(
        &self,
        app_name: &str,
        target_id: u64,
    ) -> Result<Option<(String, String)>, BoxError> {
        let app_ref = self.find_app_ref_by_name(app_name).await?;
        let Some(app_ref) = app_ref else {
            return Ok(None);
        };

        let app_proxy = app_ref
            .as_accessible_proxy(self.connection.connection())
            .await?;

        // Get the root's children (typically the window)
        let children: Vec<ObjectRef> = app_proxy.get_children().await?;

        for child_ref in children.iter() {
            if let Some(path) =
                Box::pin(self.find_path_in_tree_by_atspi_id(child_ref, target_id)).await?
            {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Recursively search for element path by actual AT-SPI node ID
    async fn find_path_in_tree_by_atspi_id(
        &self,
        obj_ref: &ObjectRef,
        target_id: u64,
    ) -> Result<Option<(String, String)>, BoxError> {
        // Check if this node's path ends with the target ID
        let path_str = obj_ref.path.to_string();
        if extract_atspi_node_id(&path_str) == Some(target_id) {
            return Ok(Some((obj_ref.name.to_string(), path_str)));
        }

        let proxy = obj_ref
            .as_accessible_proxy(self.connection.connection())
            .await?;

        // Get children and search recursively
        let children_refs: Vec<ObjectRef> = proxy.get_children().await.unwrap_or_default();

        for child_ref in children_refs.iter() {
            if let Some(found) =
                Box::pin(self.find_path_in_tree_by_atspi_id(child_ref, target_id)).await?
            {
                return Ok(Some(found));
            }
        }

        Ok(None)
    }

    /// Click an element using AT-SPI Action interface
    pub async fn click_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ActionProxy directly (Proxies::action() has issues with interface conversion)
        use atspi_proxies::action::ActionProxy;
        let action_proxy = ActionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        // Action index 0 is typically the default action (click for buttons)
        let result = action_proxy.do_action(0).await?;
        Ok(result)
    }

    /// Set text content using AT-SPI EditableText interface
    pub async fn set_text(&self, app_name: &str, id: u64, text: &str) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build EditableTextProxy directly
        use atspi_proxies::editable_text::EditableTextProxy;
        let editable_text_proxy = EditableTextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = editable_text_proxy.set_text_contents(text).await?;
        Ok(result)
    }

    /// Build a UiTree from an AccessibleProxy
    async fn build_ui_tree_from_proxy(
        &self,
        root_proxy: &AccessibleProxy<'_>,
    ) -> Result<Option<UiTree>, BoxError> {
        let mut nodes: Vec<NodeInfo> = Vec::new();
        let mut roots: Vec<u64> = Vec::new();

        // Get the root's children (typically the window)
        let children: Vec<ObjectRef> = root_proxy.get_children().await?;

        for child_ref in children.iter() {
            let window_proxy: AccessibleProxy<'_> = child_ref
                .as_accessible_proxy(self.connection.connection())
                .await?;

            // Use the actual AT-SPI node ID from the object path
            let window_id = extract_atspi_node_id(&child_ref.path.to_string()).unwrap_or(1);
            roots.push(window_id);

            self.traverse_tree_with_atspi_ids(
                &window_proxy,
                &child_ref.path.to_string(),
                &mut nodes,
            )
            .await?;
        }

        if nodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(UiTree { nodes, roots }))
    }

    /// Recursively traverse the accessibility tree using actual AT-SPI node IDs
    async fn traverse_tree_with_atspi_ids(
        &self,
        proxy: &AccessibleProxy<'_>,
        path: &str,
        nodes: &mut Vec<NodeInfo>,
    ) -> Result<(), BoxError> {
        // Extract the actual AT-SPI node ID from the path
        let node_id = extract_atspi_node_id(path).unwrap_or(0);

        // Get node information
        let name: String = proxy.name().await.unwrap_or_default();
        let description: String = proxy.description().await.unwrap_or_default();
        let role_enum = proxy.get_role().await.ok();
        let role = role_enum
            .map(|r| format!("{:?}", r))
            .unwrap_or_else(|| "Unknown".to_string());

        // Get state information
        let state_set = proxy.get_state().await.ok();
        let (focused, disabled, toggled) = if let Some(state) = state_set {
            (
                state.contains(atspi_common::State::Focused),
                !state.contains(atspi_common::State::Enabled),
                if state.contains(atspi_common::State::Checked)
                    || state.contains(atspi_common::State::Pressed)
                {
                    Some(true)
                } else if state.contains(atspi_common::State::Checkable) {
                    Some(false)
                } else {
                    None
                },
            )
        } else {
            (false, false, None)
        };

        // Get children
        let children_refs: Vec<ObjectRef> = proxy.get_children().await.unwrap_or_default();
        let mut child_ids: Vec<u64> = Vec::new();

        // Process children using their actual AT-SPI IDs
        for child_ref in children_refs.iter() {
            let child_path = child_ref.path.to_string();
            let child_id = extract_atspi_node_id(&child_path).unwrap_or(0);
            child_ids.push(child_id);

            let child_proxy: AccessibleProxy<'_> = child_ref
                .as_accessible_proxy(self.connection.connection())
                .await?;

            // Recursive traversal
            Box::pin(self.traverse_tree_with_atspi_ids(&child_proxy, &child_path, nodes)).await?;
        }

        // Determine label based on role and name
        let label = if !name.is_empty() {
            Some(name)
        } else if !description.is_empty() {
            Some(description)
        } else {
            None
        };

        let node_info = NodeInfo {
            id: node_id,
            role,
            label,
            value: None, // AT-SPI can provide value through Value interface
            children: child_ids,
            bounds: None, // AT-SPI can provide bounds through Component interface
            toggled,
            disabled,
            focused,
        };

        nodes.push(node_info);
        Ok(())
    }

    /// Find UI elements by label (exact or substring match)
    pub async fn find_by_label(
        &self,
        app_name: &str,
        pattern: &str,
        exact: bool,
    ) -> Result<Vec<NodeInfo>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name).await?;
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
    pub async fn find_by_role(
        &self,
        app_name: &str,
        role: &str,
    ) -> Result<Vec<NodeInfo>, BoxError> {
        let tree = self.get_ui_tree_by_app_name(app_name).await?;
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

    // ========================================================================
    // Priority 2: Element Information (AT-SPI Component)
    // ========================================================================

    /// Get element bounds using AT-SPI Component interface
    pub async fn get_bounds(&self, app_name: &str, id: u64) -> Result<Option<Rect>, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ComponentProxy directly
        use atspi_proxies::component::ComponentProxy;
        let component_proxy = ComponentProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        // Use Window coordinates (relative to the window)
        let (x, y, width, height) = component_proxy.get_extents(CoordType::Window).await?;
        Ok(Some(Rect {
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: height as f32,
        }))
    }

    /// Focus an element using AT-SPI Component interface
    pub async fn focus_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ComponentProxy directly
        use atspi_proxies::component::ComponentProxy;
        let component_proxy = ComponentProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = component_proxy.grab_focus().await?;
        Ok(result)
    }

    /// Scroll element into view using AT-SPI Component interface
    pub async fn scroll_to_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ComponentProxy directly
        use atspi_proxies::component::ComponentProxy;
        let component_proxy = ComponentProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        // ScrollType::Anywhere - scroll to make element visible anywhere in view
        let result = component_proxy.scroll_to(ScrollType::Anywhere).await?;
        Ok(result)
    }

    // ========================================================================
    // Priority 3: Value Operations (AT-SPI Value)
    // ========================================================================

    /// Get value information using AT-SPI Value interface
    pub async fn get_value(&self, app_name: &str, id: u64) -> Result<Option<ValueInfo>, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ValueProxy directly (Proxies::value() has issues with interface conversion)
        use atspi_proxies::value::ValueProxy;
        let value_proxy = ValueProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let current = value_proxy.current_value().await?;
        let minimum = value_proxy.minimum_value().await?;
        let maximum = value_proxy.maximum_value().await?;
        let increment = value_proxy.minimum_increment().await?;

        Ok(Some(ValueInfo {
            current,
            minimum,
            maximum,
            increment,
        }))
    }

    /// Set value using AT-SPI Value interface
    pub async fn set_value(&self, app_name: &str, id: u64, value: f64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build ValueProxy directly (Proxies::value() has issues with interface conversion)
        use atspi_proxies::value::ValueProxy;
        let value_proxy = ValueProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        value_proxy.set_current_value(value).await?;
        Ok(true)
    }

    // ========================================================================
    // Priority 4: Selection Operations (AT-SPI Selection)
    // ========================================================================

    /// Select an item by index using AT-SPI Selection interface
    pub async fn select_item(&self, app_name: &str, id: u64, index: i32) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build SelectionProxy directly (Proxies::selection() has issues with interface conversion)
        use atspi_proxies::selection::SelectionProxy;
        let selection_proxy = SelectionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = selection_proxy.select_child(index).await?;
        Ok(result)
    }

    /// Deselect an item by index using AT-SPI Selection interface
    pub async fn deselect_item(
        &self,
        app_name: &str,
        id: u64,
        index: i32,
    ) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build SelectionProxy directly (Proxies::selection() has issues with interface conversion)
        use atspi_proxies::selection::SelectionProxy;
        let selection_proxy = SelectionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = selection_proxy.deselect_child(index).await?;
        Ok(result)
    }

    /// Get count of selected items using AT-SPI Selection interface
    ///
    /// For ComboBox, this checks if there's a selected value (name property).
    /// For other containers, it uses the Selection interface's nselected_children().
    pub async fn get_selected_count(&self, app_name: &str, id: u64) -> Result<i32, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // First, check the role to handle ComboBox specially
        let accessible_proxy = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let role = accessible_proxy.get_role().await.ok();

        // ComboBox: check if there's a selected value (stored in name property)
        if let Some(role) = role
            && role == atspi_common::Role::ComboBox
        {
            let name: String = accessible_proxy.name().await.unwrap_or_default();
            // If name is not empty, something is selected
            return Ok(if name.is_empty() { 0 } else { 1 });
        }

        // For other containers, use the Selection interface
        use atspi_proxies::selection::SelectionProxy;
        let selection_proxy = SelectionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let count = selection_proxy.nselected_children().await?;
        Ok(count)
    }

    /// Select all items using AT-SPI Selection interface
    pub async fn select_all(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build SelectionProxy directly (Proxies::selection() has issues with interface conversion)
        use atspi_proxies::selection::SelectionProxy;
        let selection_proxy = SelectionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = selection_proxy.select_all().await?;
        Ok(result)
    }

    /// Clear all selections using AT-SPI Selection interface
    pub async fn clear_selection(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build SelectionProxy directly (Proxies::selection() has issues with interface conversion)
        use atspi_proxies::selection::SelectionProxy;
        let selection_proxy = SelectionProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = selection_proxy.clear_selection().await?;
        Ok(result)
    }

    // ========================================================================
    // Priority 5: Text Operations (AT-SPI Text)
    // ========================================================================

    /// Get text content using AT-SPI Text interface
    pub async fn get_text(&self, app_name: &str, id: u64) -> Result<Option<TextInfo>, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build TextProxy directly (Proxies::text() has issues with interface conversion)
        use atspi_proxies::text::TextProxy;
        let text_proxy = match TextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await
        {
            Ok(proxy) => proxy,
            Err(_) => return Ok(None), // Text interface not available
        };

        let length = text_proxy.character_count().await?;
        let text = text_proxy.get_text(0, length).await?;
        let caret_offset = text_proxy.caret_offset().await?;
        Ok(Some(TextInfo {
            text,
            length,
            caret_offset,
        }))
    }

    /// Get text selection using AT-SPI Text interface
    pub async fn get_text_selection(
        &self,
        app_name: &str,
        id: u64,
    ) -> Result<Option<TextSelection>, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build TextProxy directly (Proxies::text() has issues with interface conversion)
        use atspi_proxies::text::TextProxy;
        let text_proxy = match TextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await
        {
            Ok(proxy) => proxy,
            Err(_) => return Ok(None), // Text interface not available
        };

        // Note: atspi-proxies has a bug where it calls "GetNselections" instead of "GetNSelections"
        // (case mismatch). We use call_method directly with the correct method name.
        let n_selections: i32 = text_proxy
            .inner()
            .call_method("GetNSelections", &())
            .await?
            .body()
            .deserialize()?;
        if n_selections > 0 {
            let (start, end) = text_proxy.get_selection(0).await?;
            Ok(Some(TextSelection { start, end }))
        } else {
            Ok(None)
        }
    }

    /// Set text selection using AT-SPI Text interface
    pub async fn set_text_selection(
        &self,
        app_name: &str,
        id: u64,
        start: i32,
        end: i32,
    ) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build TextProxy directly (Proxies::text() has issues with interface conversion)
        use atspi_proxies::text::TextProxy;
        let text_proxy = TextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        // Try to add a new selection or modify existing one
        // Note: atspi-proxies has a bug where it calls "GetNselections" instead of "GetNSelections"
        let n_selections: i32 = text_proxy
            .inner()
            .call_method("GetNSelections", &())
            .await?
            .body()
            .deserialize()?;
        if n_selections > 0 {
            let result = text_proxy.set_selection(0, start, end).await?;
            Ok(result)
        } else {
            let result = text_proxy.add_selection(start, end).await?;
            Ok(result)
        }
    }

    /// Get caret position using AT-SPI Text interface
    pub async fn get_caret_position(&self, app_name: &str, id: u64) -> Result<i32, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build TextProxy directly (Proxies::text() has issues with interface conversion)
        use atspi_proxies::text::TextProxy;
        let text_proxy = TextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let offset = text_proxy.caret_offset().await?;
        Ok(offset)
    }

    /// Set caret position using AT-SPI Text interface
    pub async fn set_caret_position(
        &self,
        app_name: &str,
        id: u64,
        offset: i32,
    ) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        // Build TextProxy directly (Proxies::text() has issues with interface conversion)
        use atspi_proxies::text::TextProxy;
        let text_proxy = TextProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let result = text_proxy.set_caret_offset(offset).await?;
        Ok(result)
    }
}
