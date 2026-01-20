//! IPC server for handling MCP requests

use crate::McpClient;
use base64::Engine;
use egui_mcp_protocol::{NodeInfo, ProtocolError, Request, Response, read_request, write_response};
use tokio::net::{UnixListener, UnixStream};

/// IPC server that listens for MCP requests
pub struct IpcServer;

impl IpcServer {
    /// Run the IPC server
    pub async fn run(client: McpClient) -> Result<(), ProtocolError> {
        let socket_path = client.socket_path().await;

        // Remove existing socket file if it exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        // Create parent directory if needed
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&socket_path)?;
        tracing::info!("IPC server listening on {:?}", socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let client = client.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, client).await {
                            match e {
                                ProtocolError::ConnectionClosed => {
                                    tracing::debug!("Client disconnected");
                                }
                                _ => {
                                    tracing::error!("Connection error: {}", e);
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Handle a single connection
    async fn handle_connection(stream: UnixStream, client: McpClient) -> Result<(), ProtocolError> {
        let (mut reader, mut writer) = stream.into_split();

        loop {
            let request = read_request(&mut reader).await?;
            tracing::debug!("Received request: {:?}", request);

            let response = Self::handle_request(&request, &client).await;
            tracing::debug!("Sending response: {:?}", response);

            write_response(&mut writer, &response).await?;
        }
    }

    /// Handle a single request
    async fn handle_request(request: &Request, client: &McpClient) -> Response {
        match request {
            Request::Ping => Response::Pong,
            Request::GetUiTree => {
                let tree = client.get_ui_tree().await;
                Response::UiTree(tree)
            }
            Request::FindByLabel { pattern, exact } => {
                let tree = client.get_ui_tree().await;
                let elements = Self::find_by_label(&tree.nodes, pattern, *exact);
                Response::Elements(elements)
            }
            Request::FindByRole { role } => {
                let tree = client.get_ui_tree().await;
                let elements = Self::find_by_role(&tree.nodes, role);
                Response::Elements(elements)
            }
            Request::GetElement { id } => {
                let tree = client.get_ui_tree().await;
                let element = tree.nodes.into_iter().find(|n| n.id == *id);
                Response::Element(element)
            }
            Request::TakeScreenshot => {
                // Request a screenshot from the UI
                client.request_screenshot().await;

                // Wait for the screenshot to be captured (with timeout)
                let mut attempts = 0;
                const MAX_ATTEMPTS: u32 = 50; // 50 * 100ms = 5 seconds max
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if let Some(data) = client.get_screenshot().await {
                        // Clear the screenshot data after reading
                        client.clear_screenshot().await;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
                        return Response::Screenshot {
                            data: encoded,
                            format: "png".to_string(),
                        };
                    }
                    attempts += 1;
                    if attempts >= MAX_ATTEMPTS {
                        return Response::Error {
                            message: "Screenshot timeout: the egui app did not provide a screenshot within 5 seconds".to_string(),
                        };
                    }
                }
            }
        }
    }

    /// Find nodes by label (exact or substring match)
    fn find_by_label(nodes: &[NodeInfo], pattern: &str, exact: bool) -> Vec<NodeInfo> {
        nodes
            .iter()
            .filter(|node| {
                if let Some(ref label) = node.label {
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
            .collect()
    }

    /// Find nodes by role
    fn find_by_role(nodes: &[NodeInfo], role: &str) -> Vec<NodeInfo> {
        nodes
            .iter()
            .filter(|node| node.role.eq_ignore_ascii_case(role))
            .cloned()
            .collect()
    }
}
