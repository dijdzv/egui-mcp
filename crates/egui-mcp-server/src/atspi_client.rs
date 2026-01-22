//! AT-SPI client for accessing accessibility information on Linux

use atspi_common::ObjectRef;
use atspi_connection::AccessibilityConnection;
use atspi_proxies::accessible::{AccessibleProxy, ObjectRefExt};
use egui_mcp_protocol::{NodeInfo, UiTree};
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
        // Get the registry's accessible children (applications)
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
                return self.build_ui_tree_from_proxy(&app_proxy).await;
            }
        }

        Ok(None)
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
}
