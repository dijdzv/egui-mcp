//! IPC server for handling MCP requests
//!
//! This server handles requests that require direct application access:
//! - Screenshots
//! - Coordinate-based input
//! - Keyboard input
//! - Scroll events

use crate::{McpClient, PendingInput};
use base64::Engine;
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

            Request::ClickAt { x, y, button } => {
                client
                    .queue_input(PendingInput::Click {
                        x: *x,
                        y: *y,
                        button: *button,
                    })
                    .await;
                Response::Success
            }

            Request::MoveMouse { x, y } => {
                client
                    .queue_input(PendingInput::MoveMouse { x: *x, y: *y })
                    .await;
                Response::Success
            }

            Request::KeyboardInput { key } => {
                client
                    .queue_input(PendingInput::Keyboard { key: key.clone() })
                    .await;
                Response::Success
            }

            Request::Scroll {
                x,
                y,
                delta_x,
                delta_y,
            } => {
                client
                    .queue_input(PendingInput::Scroll {
                        x: *x,
                        y: *y,
                        delta_x: *delta_x,
                        delta_y: *delta_y,
                    })
                    .await;
                Response::Success
            }

            Request::Drag {
                start_x,
                start_y,
                end_x,
                end_y,
                button,
            } => {
                client
                    .queue_input(PendingInput::Drag {
                        start_x: *start_x,
                        start_y: *start_y,
                        end_x: *end_x,
                        end_y: *end_y,
                        button: *button,
                    })
                    .await;
                Response::Success
            }
        }
    }
}
