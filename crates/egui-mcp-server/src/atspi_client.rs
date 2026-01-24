//! AT-SPI client for accessing accessibility information on Linux

use atspi_common::{CoordType, ObjectRef, ScrollType};
use atspi_connection::AccessibilityConnection;
use atspi_proxies::accessible::{AccessibleProxy, ObjectRefExt};
use atspi_proxies::proxy_ext::ProxyExt;
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

        for (idx, child_ref) in children.iter().enumerate() {
            let window_id = idx as u64 + 1;

            if let Some(path) =
                Box::pin(self.find_path_in_tree(child_ref, window_id, target_id)).await?
            {
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Recursively search for element path by ID
    async fn find_path_in_tree(
        &self,
        obj_ref: &ObjectRef,
        node_id: u64,
        target_id: u64,
    ) -> Result<Option<(String, String)>, BoxError> {
        if node_id == target_id {
            return Ok(Some((obj_ref.name.to_string(), obj_ref.path.to_string())));
        }

        let proxy = obj_ref
            .as_accessible_proxy(self.connection.connection())
            .await?;

        // Get children
        let children_refs: Vec<ObjectRef> = proxy.get_children().await.unwrap_or_default();
        let base_id = node_id * 100;

        for (idx, child_ref) in children_refs.iter().enumerate() {
            let child_id = base_id + idx as u64 + 1;

            // Depth limit
            if child_id >= 1_000_000_000 {
                continue;
            }

            if let Some(found) =
                Box::pin(self.find_path_in_tree(child_ref, child_id, target_id)).await?
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        let action_proxy = proxies.action()?;

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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        let editable_text_proxy = proxies.editable_text()?;

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

        for (idx, child_ref) in children.iter().enumerate() {
            let window_proxy: AccessibleProxy<'_> = child_ref
                .as_accessible_proxy(self.connection.connection())
                .await?;

            // Use a simple incrementing ID scheme
            let window_id = idx as u64 + 1;
            roots.push(window_id);

            self.traverse_tree(&window_proxy, window_id, &mut nodes)
                .await?;
        }

        if nodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(UiTree { nodes, roots }))
    }

    /// Recursively traverse the accessibility tree
    async fn traverse_tree(
        &self,
        proxy: &AccessibleProxy<'_>,
        node_id: u64,
        nodes: &mut Vec<NodeInfo>,
    ) -> Result<(), BoxError> {
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

        // Generate IDs for children
        let base_id = node_id * 100;
        for (idx, child_ref) in children_refs.iter().enumerate() {
            let child_id = base_id + idx as u64 + 1;
            child_ids.push(child_id);

            let child_proxy: AccessibleProxy<'_> = child_ref
                .as_accessible_proxy(self.connection.connection())
                .await?;

            // Recursive traversal with depth limit (to avoid infinite loops)
            if child_id < 1_000_000_000 {
                Box::pin(self.traverse_tree(&child_proxy, child_id, nodes)).await?;
            }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.component() {
            Ok(component_proxy) => {
                // Use Window coordinates (relative to the window)
                let (x, y, width, height) = component_proxy.get_extents(CoordType::Window).await?;
                Ok(Some(Rect {
                    x: x as f32,
                    y: y as f32,
                    width: width as f32,
                    height: height as f32,
                }))
            }
            Err(_) => Ok(None), // Component interface not available
        }
    }

    /// Focus an element using AT-SPI Component interface
    pub async fn focus_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.component() {
            Ok(component_proxy) => {
                let result = component_proxy.grab_focus().await?;
                Ok(result)
            }
            Err(e) => Err(format!("Component interface not available: {}", e).into()),
        }
    }

    /// Scroll element into view using AT-SPI Component interface
    pub async fn scroll_to_element(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.component() {
            Ok(component_proxy) => {
                // ScrollType::Anywhere - scroll to make element visible anywhere in view
                let result = component_proxy.scroll_to(ScrollType::Anywhere).await?;
                Ok(result)
            }
            Err(e) => Err(format!("Component interface not available: {}", e).into()),
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.value() {
            Ok(value_proxy) => {
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
            Err(_) => Ok(None), // Value interface not available
        }
    }

    /// Set value using AT-SPI Value interface
    pub async fn set_value(&self, app_name: &str, id: u64, value: f64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.value() {
            Ok(value_proxy) => {
                value_proxy.set_current_value(value).await?;
                Ok(true)
            }
            Err(e) => Err(format!("Value interface not available: {}", e).into()),
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.selection() {
            Ok(selection_proxy) => {
                let result = selection_proxy.select_child(index).await?;
                Ok(result)
            }
            Err(e) => Err(format!("Selection interface not available: {}", e).into()),
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.selection() {
            Ok(selection_proxy) => {
                let result = selection_proxy.deselect_child(index).await?;
                Ok(result)
            }
            Err(e) => Err(format!("Selection interface not available: {}", e).into()),
        }
    }

    /// Get count of selected items using AT-SPI Selection interface
    pub async fn get_selected_count(&self, app_name: &str, id: u64) -> Result<i32, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.selection() {
            Ok(selection_proxy) => {
                let count = selection_proxy.nselected_children().await?;
                Ok(count)
            }
            Err(e) => Err(format!("Selection interface not available: {}", e).into()),
        }
    }

    /// Select all items using AT-SPI Selection interface
    pub async fn select_all(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.selection() {
            Ok(selection_proxy) => {
                let result = selection_proxy.select_all().await?;
                Ok(result)
            }
            Err(e) => Err(format!("Selection interface not available: {}", e).into()),
        }
    }

    /// Clear all selections using AT-SPI Selection interface
    pub async fn clear_selection(&self, app_name: &str, id: u64) -> Result<bool, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.selection() {
            Ok(selection_proxy) => {
                let result = selection_proxy.clear_selection().await?;
                Ok(result)
            }
            Err(e) => Err(format!("Selection interface not available: {}", e).into()),
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.text() {
            Ok(text_proxy) => {
                let length = text_proxy.character_count().await?;
                let text = text_proxy.get_text(0, length).await?;
                let caret_offset = text_proxy.caret_offset().await?;
                Ok(Some(TextInfo {
                    text,
                    length,
                    caret_offset,
                }))
            }
            Err(_) => Ok(None), // Text interface not available
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.text() {
            Ok(text_proxy) => {
                let n_selections = text_proxy.get_nselections().await?;
                if n_selections > 0 {
                    let (start, end) = text_proxy.get_selection(0).await?;
                    Ok(Some(TextSelection { start, end }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None), // Text interface not available
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.text() {
            Ok(text_proxy) => {
                // Try to add a new selection or modify existing one
                let n_selections = text_proxy.get_nselections().await?;
                if n_selections > 0 {
                    let result = text_proxy.set_selection(0, start, end).await?;
                    Ok(result)
                } else {
                    let result = text_proxy.add_selection(start, end).await?;
                    Ok(result)
                }
            }
            Err(e) => Err(format!("Text interface not available: {}", e).into()),
        }
    }

    /// Get caret position using AT-SPI Text interface
    pub async fn get_caret_position(&self, app_name: &str, id: u64) -> Result<i32, BoxError> {
        let path_info = self.find_element_path_by_id(app_name, id).await?;
        let Some((destination, path)) = path_info else {
            return Err(format!("Element with id {} not found", id).into());
        };

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.text() {
            Ok(text_proxy) => {
                let offset = text_proxy.caret_offset().await?;
                Ok(offset)
            }
            Err(e) => Err(format!("Text interface not available: {}", e).into()),
        }
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

        let proxy: AccessibleProxy<'_> = AccessibleProxy::builder(self.connection.connection())
            .destination(destination.as_str())?
            .path(path.as_str())?
            .build()
            .await?;

        let mut proxies = proxy.proxies().await?;
        match proxies.text() {
            Ok(text_proxy) => {
                let result = text_proxy.set_caret_offset(offset).await?;
                Ok(result)
            }
            Err(e) => Err(format!("Text interface not available: {}", e).into()),
        }
    }
}
