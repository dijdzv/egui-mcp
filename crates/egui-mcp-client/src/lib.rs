//! Library to embed in egui apps for MCP integration
//!
//! This crate provides client-side integration for egui applications
//! to support MCP automation features that require direct application access:
//! - Screenshots
//! - Coordinate-based input (clicks, drags)
//! - Keyboard input
//! - Scroll events
//!
//! Note: UI tree access and element-based interactions are handled via AT-SPI
//! on the server side and don't require this client library.
//!
//! ## Usage in raw_input_hook
//!
//! ```rust,ignore
//! impl eframe::App for MyApp {
//!     fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
//!         let inputs = self.runtime.block_on(self.mcp_client.take_pending_inputs());
//!         egui_mcp_client::inject_inputs(ctx, raw_input, inputs);
//!     }
//! }
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use egui_mcp_protocol::{MouseButton, Request, Response};

mod server;

pub use server::IpcServer;

// Re-export egui types for convenience
pub use egui;

/// Pending input event to be processed by the egui application
#[derive(Debug, Clone)]
pub enum PendingInput {
    /// Click at coordinates
    Click { x: f32, y: f32, button: MouseButton },
    /// Double click at coordinates
    DoubleClick { x: f32, y: f32, button: MouseButton },
    /// Move mouse to coordinates
    MoveMouse { x: f32, y: f32 },
    /// Keyboard input
    Keyboard { key: String },
    /// Scroll at coordinates
    Scroll {
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    },
    /// Drag operation
    Drag {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        button: MouseButton,
    },
}

/// Shared state for the MCP client
#[derive(Clone)]
pub struct McpClient {
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    socket_path: PathBuf,
    /// Screenshot data (PNG encoded)
    screenshot_data: Option<Vec<u8>>,
    /// Flag to request a screenshot
    screenshot_requested: bool,
    /// Pending input events to be processed by the egui app
    pending_inputs: Vec<PendingInput>,
}

impl McpClient {
    /// Create a new MCP client with default socket path
    pub fn new() -> Self {
        Self::with_socket_path(egui_mcp_protocol::default_socket_path())
    }

    /// Create a new MCP client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClientState {
                socket_path,
                screenshot_data: None,
                screenshot_requested: false,
                pending_inputs: Vec::new(),
            })),
        }
    }

    /// Get the socket path
    pub async fn socket_path(&self) -> PathBuf {
        self.state.read().await.socket_path.clone()
    }

    // Screenshot methods

    /// Set screenshot data (PNG encoded)
    pub async fn set_screenshot(&self, data: Vec<u8>) {
        self.state.write().await.screenshot_data = Some(data);
    }

    /// Get screenshot data (PNG encoded)
    pub async fn get_screenshot(&self) -> Option<Vec<u8>> {
        self.state.read().await.screenshot_data.clone()
    }

    /// Clear screenshot data
    pub async fn clear_screenshot(&self) {
        self.state.write().await.screenshot_data = None;
    }

    /// Request a screenshot (sets flag for the UI to capture)
    pub async fn request_screenshot(&self) {
        self.state.write().await.screenshot_requested = true;
    }

    /// Check if screenshot is requested and clear the flag
    pub async fn take_screenshot_request(&self) -> bool {
        let mut state = self.state.write().await;
        let requested = state.screenshot_requested;
        state.screenshot_requested = false;
        requested
    }

    // Input methods

    /// Queue an input event to be processed by the egui app
    pub async fn queue_input(&self, input: PendingInput) {
        self.state.write().await.pending_inputs.push(input);
    }

    /// Take all pending input events (clears the queue)
    pub async fn take_pending_inputs(&self) -> Vec<PendingInput> {
        std::mem::take(&mut self.state.write().await.pending_inputs)
    }

    /// Start the IPC server in a background task
    pub fn start_server(&self) -> tokio::task::JoinHandle<()> {
        let client = self.clone();
        tokio::spawn(async move {
            if let Err(e) = IpcServer::run(client).await {
                tracing::error!("IPC server error: {}", e);
            }
        })
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Input Injection Helpers
// ============================================================================

/// Convert MCP MouseButton to egui PointerButton
fn convert_mouse_button(button: &MouseButton) -> egui::PointerButton {
    match button {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
    }
}

/// Parse a key string into egui Key for special keys
fn parse_special_key(key: &str) -> Option<egui::Key> {
    match key.to_lowercase().as_str() {
        "enter" | "return" => Some(egui::Key::Enter),
        "tab" => Some(egui::Key::Tab),
        "backspace" => Some(egui::Key::Backspace),
        "delete" => Some(egui::Key::Delete),
        "escape" | "esc" => Some(egui::Key::Escape),
        "space" => Some(egui::Key::Space),
        "arrowup" | "up" => Some(egui::Key::ArrowUp),
        "arrowdown" | "down" => Some(egui::Key::ArrowDown),
        "arrowleft" | "left" => Some(egui::Key::ArrowLeft),
        "arrowright" | "right" => Some(egui::Key::ArrowRight),
        "home" => Some(egui::Key::Home),
        "end" => Some(egui::Key::End),
        "pageup" => Some(egui::Key::PageUp),
        "pagedown" => Some(egui::Key::PageDown),
        "insert" => Some(egui::Key::Insert),
        "f1" => Some(egui::Key::F1),
        "f2" => Some(egui::Key::F2),
        "f3" => Some(egui::Key::F3),
        "f4" => Some(egui::Key::F4),
        "f5" => Some(egui::Key::F5),
        "f6" => Some(egui::Key::F6),
        "f7" => Some(egui::Key::F7),
        "f8" => Some(egui::Key::F8),
        "f9" => Some(egui::Key::F9),
        "f10" => Some(egui::Key::F10),
        "f11" => Some(egui::Key::F11),
        "f12" => Some(egui::Key::F12),
        _ => None,
    }
}

/// Inject pending MCP inputs into egui's RawInput.
///
/// Call this function in your `eframe::App::raw_input_hook` implementation
/// to convert MCP inputs into egui events.
///
/// # Example
///
/// ```rust,ignore
/// impl eframe::App for MyApp {
///     fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
///         let inputs = self.runtime.block_on(self.mcp_client.take_pending_inputs());
///         egui_mcp_client::inject_inputs(ctx, raw_input, inputs);
///     }
/// }
/// ```
pub fn inject_inputs(
    ctx: &egui::Context,
    raw_input: &mut egui::RawInput,
    inputs: Vec<PendingInput>,
) {
    if inputs.is_empty() {
        return;
    }

    // Request repaint to ensure UI updates even in background
    ctx.request_repaint();

    for input in inputs {
        match input {
            PendingInput::MoveMouse { x, y } => {
                tracing::debug!("Injecting mouse move to ({}, {})", x, y);
                raw_input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
            }
            PendingInput::Click { x, y, button } => {
                tracing::debug!("Injecting click at ({}, {})", x, y);
                let egui_button = convert_mouse_button(&button);
                let pos = egui::pos2(x, y);

                raw_input.events.push(egui::Event::PointerMoved(pos));
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            PendingInput::DoubleClick { x, y, button } => {
                tracing::debug!("Injecting double click at ({}, {})", x, y);
                let egui_button = convert_mouse_button(&button);
                let pos = egui::pos2(x, y);

                raw_input.events.push(egui::Event::PointerMoved(pos));
                // First click
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
                // Second click
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                raw_input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui_button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            PendingInput::Drag {
                start_x,
                start_y,
                end_x,
                end_y,
                button,
            } => {
                tracing::debug!(
                    "Injecting drag from ({}, {}) to ({}, {})",
                    start_x,
                    start_y,
                    end_x,
                    end_y
                );
                let egui_button = convert_mouse_button(&button);
                let start_pos = egui::pos2(start_x, start_y);
                let end_pos = egui::pos2(end_x, end_y);

                raw_input.events.push(egui::Event::PointerMoved(start_pos));
                raw_input.events.push(egui::Event::PointerButton {
                    pos: start_pos,
                    button: egui_button,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                raw_input.events.push(egui::Event::PointerMoved(end_pos));
                raw_input.events.push(egui::Event::PointerButton {
                    pos: end_pos,
                    button: egui_button,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            PendingInput::Keyboard { key } => {
                tracing::debug!("Injecting keyboard input: {}", key);
                if let Some(egui_key) = parse_special_key(&key) {
                    // Special key (Enter, Tab, Backspace, etc.)
                    raw_input.events.push(egui::Event::Key {
                        key: egui_key,
                        physical_key: Some(egui_key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                    raw_input.events.push(egui::Event::Key {
                        key: egui_key,
                        physical_key: Some(egui_key),
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                } else {
                    // Regular text input
                    raw_input.events.push(egui::Event::Text(key));
                }
            }
            PendingInput::Scroll {
                x,
                y,
                delta_x,
                delta_y,
            } => {
                tracing::debug!(
                    "Injecting scroll at ({}, {}) delta ({}, {})",
                    x,
                    y,
                    delta_x,
                    delta_y
                );
                raw_input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
                raw_input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(delta_x, delta_y),
                    modifiers: egui::Modifiers::NONE,
                });
            }
        }
    }
}
