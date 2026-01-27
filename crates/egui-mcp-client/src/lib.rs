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
use tokio::sync::{RwLock, oneshot};

pub use egui_mcp_protocol::{FrameStats, LogEntry, MouseButton, PerfReport, Request, Response};

mod log_layer;
mod server;

pub use log_layer::{DEFAULT_MAX_MESSAGE_LENGTH, LogBuffer, McpLogLayer, level_to_priority};
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

/// A visual highlight to be drawn over an element
#[derive(Debug, Clone)]
pub struct Highlight {
    /// Bounding rectangle
    pub rect: egui::Rect,
    /// Highlight color (with alpha)
    pub color: egui::Color32,
    /// When the highlight should expire (None = never expires)
    pub expires_at: Option<std::time::Instant>,
}

/// Shared state for the MCP client
#[derive(Clone)]
pub struct McpClient {
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    socket_path: PathBuf,
    /// Pending screenshot request sender (event-driven)
    screenshot_sender: Option<oneshot::Sender<Vec<u8>>>,
    /// Pending input events to be processed by the egui app
    pending_inputs: Vec<PendingInput>,
    /// Active highlights to be drawn
    highlights: Vec<Highlight>,
    /// Optional log buffer (shared with McpLogLayer)
    log_buffer: Option<LogBuffer>,
    /// Frame times for performance monitoring (rolling window)
    frame_times: std::collections::VecDeque<std::time::Duration>,
    /// Maximum number of frame times to keep
    max_frame_samples: usize,
    /// Performance recording state
    perf_recording: Option<PerfRecording>,
    /// Last frame instant for automatic timing
    last_frame_instant: Option<std::time::Instant>,
}

/// State for an active performance recording session
struct PerfRecording {
    /// When the recording started
    start_time: std::time::Instant,
    /// Recorded frame times
    frame_times: Vec<std::time::Duration>,
    /// Optional auto-stop after duration
    duration_ms: u64,
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
                screenshot_sender: None,
                pending_inputs: Vec::new(),
                highlights: Vec::new(),
                log_buffer: None,
                frame_times: std::collections::VecDeque::with_capacity(120),
                max_frame_samples: 120, // ~2 seconds at 60fps
                perf_recording: None,
                last_frame_instant: None,
            })),
        }
    }

    /// Set the log buffer (from McpLogLayer::new())
    pub async fn with_log_buffer(self, buffer: LogBuffer) -> Self {
        self.state.write().await.log_buffer = Some(buffer);
        self
    }

    /// Set the log buffer synchronously (for initialization)
    pub fn with_log_buffer_sync(self, buffer: LogBuffer) -> Self {
        // Use try_write to avoid blocking
        if let Ok(mut state) = self.state.try_write() {
            state.log_buffer = Some(buffer);
        }
        self
    }

    /// Get the socket path
    pub async fn socket_path(&self) -> PathBuf {
        self.state.read().await.socket_path.clone()
    }

    // Screenshot methods (event-driven)

    /// Request a screenshot and return a receiver to await the result.
    /// This is more efficient than polling as it uses a oneshot channel.
    pub async fn request_screenshot(&self) -> oneshot::Receiver<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        self.state.write().await.screenshot_sender = Some(tx);
        rx
    }

    /// Check if screenshot is requested and return the sender if available.
    /// Called by the UI to check if it should capture a screenshot.
    pub async fn take_screenshot_request(&self) -> bool {
        self.state.read().await.screenshot_sender.is_some()
    }

    /// Set screenshot data (PNG encoded) - sends through the oneshot channel.
    /// Called by the UI after capturing a screenshot.
    pub async fn set_screenshot(&self, data: Vec<u8>) {
        let sender = self.state.write().await.screenshot_sender.take();
        if let Some(tx) = sender {
            // Ignore error if receiver was dropped (e.g., timeout)
            let _ = tx.send(data);
        }
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

    // Highlight methods

    /// Add a highlight to be drawn
    pub async fn add_highlight(&self, highlight: Highlight) {
        self.state.write().await.highlights.push(highlight);
    }

    /// Clear all highlights
    pub async fn clear_highlights(&self) {
        self.state.write().await.highlights.clear();
    }

    /// Get active highlights (removes expired ones)
    pub async fn get_highlights(&self) -> Vec<Highlight> {
        let mut state = self.state.write().await;
        let now = std::time::Instant::now();
        // Remove expired highlights
        state
            .highlights
            .retain(|h| h.expires_at.is_none() || h.expires_at.unwrap() > now);
        state.highlights.clone()
    }

    // Log methods

    /// Get log entries, optionally filtered by level and limited in count
    pub async fn get_logs(&self, min_level: Option<&str>, limit: Option<usize>) -> Vec<LogEntry> {
        let state = self.state.read().await;
        if let Some(ref buffer) = state.log_buffer {
            let buf = buffer.lock();
            let min_priority = min_level.map(level_to_priority).unwrap_or(0);

            let filtered: Vec<LogEntry> = buf
                .iter()
                .filter(|entry| level_to_priority(&entry.level) >= min_priority)
                .cloned()
                .collect();

            match limit {
                Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
                None => filtered,
            }
        } else {
            Vec::new()
        }
    }

    /// Clear all log entries
    pub async fn clear_logs(&self) {
        let state = self.state.read().await;
        if let Some(ref buffer) = state.log_buffer {
            buffer.lock().clear();
        }
    }

    // Performance monitoring methods

    /// Record a frame for performance monitoring (auto-timing version)
    /// Call this once at the end of each frame (in eframe::App::update).
    /// The frame time is automatically calculated from the previous call.
    pub async fn record_frame_auto(&self) {
        let mut state = self.state.write().await;
        let now = std::time::Instant::now();

        if let Some(last) = state.last_frame_instant {
            let frame_time = now.duration_since(last);
            let max_samples = state.max_frame_samples;

            // Add to rolling window
            state.frame_times.push_back(frame_time);
            while state.frame_times.len() > max_samples {
                state.frame_times.pop_front();
            }

            // Add to recording if active
            if let Some(ref mut recording) = state.perf_recording {
                recording.frame_times.push(frame_time);
            }
        }

        state.last_frame_instant = Some(now);
    }

    /// Record a frame time for performance monitoring (manual timing version)
    /// Call this at the end of each frame (in eframe::App::update)
    pub async fn record_frame(&self, frame_time: std::time::Duration) {
        let mut state = self.state.write().await;
        let max_samples = state.max_frame_samples;

        // Add to rolling window
        state.frame_times.push_back(frame_time);
        while state.frame_times.len() > max_samples {
            state.frame_times.pop_front();
        }

        // Add to recording if active
        if let Some(ref mut recording) = state.perf_recording {
            recording.frame_times.push(frame_time);

            // Check if recording should auto-stop
            if recording.duration_ms > 0 {
                let elapsed = recording.start_time.elapsed().as_millis() as u64;
                if elapsed >= recording.duration_ms {
                    // Recording will be stopped when get_perf_report is called
                }
            }
        }
    }

    /// Get current frame statistics
    pub async fn get_frame_stats(&self) -> FrameStats {
        let state = self.state.read().await;

        if state.frame_times.is_empty() {
            return FrameStats {
                fps: 0.0,
                frame_time_ms: 0.0,
                frame_time_min_ms: 0.0,
                frame_time_max_ms: 0.0,
                sample_count: 0,
            };
        }

        let times: Vec<f32> = state
            .frame_times
            .iter()
            .map(|d| d.as_secs_f32() * 1000.0)
            .collect();

        let sum: f32 = times.iter().sum();
        let avg = sum / times.len() as f32;
        let min = times.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = times.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        FrameStats {
            fps: if avg > 0.0 { 1000.0 / avg } else { 0.0 },
            frame_time_ms: avg,
            frame_time_min_ms: min,
            frame_time_max_ms: max,
            sample_count: times.len(),
        }
    }

    /// Start recording performance data
    pub async fn start_perf_recording(&self, duration_ms: u64) {
        let mut state = self.state.write().await;
        state.perf_recording = Some(PerfRecording {
            start_time: std::time::Instant::now(),
            frame_times: Vec::new(),
            duration_ms,
        });
    }

    /// Stop recording and get the performance report
    pub async fn get_perf_report(&self) -> Option<PerfReport> {
        let mut state = self.state.write().await;
        let recording = state.perf_recording.take()?;

        if recording.frame_times.is_empty() {
            return None;
        }

        let duration_ms = recording.start_time.elapsed().as_millis() as u64;
        let total_frames = recording.frame_times.len();

        let mut times_ms: Vec<f32> = recording
            .frame_times
            .iter()
            .map(|d| d.as_secs_f32() * 1000.0)
            .collect();

        let sum: f32 = times_ms.iter().sum();
        let avg_frame_time = sum / total_frames as f32;
        let avg_fps = if avg_frame_time > 0.0 {
            1000.0 / avg_frame_time
        } else {
            0.0
        };
        let min_frame_time = times_ms.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_frame_time = times_ms.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        // Calculate percentiles
        times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_idx = (total_frames as f32 * 0.95) as usize;
        let p99_idx = (total_frames as f32 * 0.99) as usize;
        let p95_frame_time = times_ms
            .get(p95_idx.min(total_frames - 1))
            .copied()
            .unwrap_or(0.0);
        let p99_frame_time = times_ms
            .get(p99_idx.min(total_frames - 1))
            .copied()
            .unwrap_or(0.0);

        Some(PerfReport {
            duration_ms,
            total_frames,
            avg_fps,
            avg_frame_time_ms: avg_frame_time,
            min_frame_time_ms: min_frame_time,
            max_frame_time_ms: max_frame_time,
            p95_frame_time_ms: p95_frame_time,
            p99_frame_time_ms: p99_frame_time,
        })
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

// ============================================================================
// Highlight Drawing Helper
// ============================================================================

/// Draw active highlights on the egui context.
///
/// Call this function at the end of your `eframe::App::update` implementation
/// to draw element highlights over the UI.
///
/// # Example
///
/// ```rust,ignore
/// impl eframe::App for MyApp {
///     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
///         // ... your UI code ...
///
///         // Draw highlights at the end
///         let highlights = self.runtime.block_on(self.mcp_client.get_highlights());
///         egui_mcp_client::draw_highlights(ctx, &highlights);
///     }
/// }
/// ```
pub fn draw_highlights(ctx: &egui::Context, highlights: &[Highlight]) {
    if highlights.is_empty() {
        return;
    }

    // Request repaint to ensure highlights are updated (for expiration)
    ctx.request_repaint();

    // Use the debug painter to draw on top of everything
    let painter = ctx.debug_painter();

    for highlight in highlights {
        // Draw a colored rectangle border
        painter.rect_stroke(
            highlight.rect,
            0.0, // No rounding
            egui::Stroke::new(3.0, highlight.color),
            egui::StrokeKind::Outside,
        );

        // Draw a semi-transparent fill
        let fill_color = egui::Color32::from_rgba_unmultiplied(
            highlight.color.r(),
            highlight.color.g(),
            highlight.color.b(),
            highlight.color.a() / 4, // 25% opacity for fill
        );
        painter.rect_filled(highlight.rect, 0.0, fill_color);
    }
}
