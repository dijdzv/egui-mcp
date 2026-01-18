//! IPC server for handling MCP requests

use crate::McpClient;
use egui_mcp_protocol::{ProtocolError, Request, Response, read_request, write_response};
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
        }
    }
}
