//! Demo egui application for testing egui-mcp
//!
//! This app demonstrates integration with egui-mcp-client for:
//! - Screenshots
//! - Coordinate-based input (click, hover, drag)
//! - Keyboard input
//! - Scroll events
//!
//! Note: UI tree access is handled via AT-SPI on the server side
//! and doesn't require any special code in the egui application.

use eframe::egui;
use egui_mcp_client::{McpClient, MouseButton, PendingInput};
use image::ImageEncoder;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    // Create tokio runtime for async operations
    let runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));

    // Create MCP client
    let mcp_client = McpClient::new();

    // Start IPC server in background
    let client_clone = mcp_client.clone();
    runtime.spawn(async move {
        if let Err(e) = egui_mcp_client::IpcServer::run(client_clone).await {
            tracing::error!("IPC server error: {}", e);
        }
    });

    tracing::info!("Starting demo app with MCP client...");
    tracing::info!(
        "Socket path: {:?}",
        runtime.block_on(mcp_client.socket_path())
    );

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-mcp Demo App",
        options,
        Box::new(move |cc| Ok(Box::new(DemoApp::new(cc, mcp_client, runtime)))),
    )
}

struct DemoApp {
    name: String,
    counter: i32,
    checkbox_value: bool,
    mcp_client: McpClient,
    runtime: Arc<Runtime>,
    // Input state for visualization
    last_mouse_pos: Option<(f32, f32)>,
    last_click: Option<(f32, f32, String)>,
    last_double_click: Option<(f32, f32, String)>,
    last_drag: Option<((f32, f32), (f32, f32))>,
    last_key: Option<String>,
    last_scroll: Option<(f32, f32, f32, f32)>,
}

impl DemoApp {
    fn new(cc: &eframe::CreationContext<'_>, mcp_client: McpClient, runtime: Arc<Runtime>) -> Self {
        // Enable AccessKit at initialization for AT-SPI integration
        cc.egui_ctx.enable_accesskit();

        Self {
            name: String::new(),
            counter: 0,
            checkbox_value: false,
            mcp_client,
            runtime,
            last_mouse_pos: None,
            last_click: None,
            last_double_click: None,
            last_drag: None,
            last_key: None,
            last_scroll: None,
        }
    }

    /// Process pending MCP inputs and update state
    fn process_pending_inputs(&mut self) {
        let inputs = self.runtime.block_on(self.mcp_client.take_pending_inputs());
        for input in inputs {
            match input {
                PendingInput::MoveMouse { x, y } => {
                    self.last_mouse_pos = Some((x, y));
                    tracing::info!("Mouse moved to ({}, {})", x, y);
                }
                PendingInput::Click { x, y, button } => {
                    let button_name = match button {
                        MouseButton::Left => "left",
                        MouseButton::Right => "right",
                        MouseButton::Middle => "middle",
                    };
                    self.last_click = Some((x, y, button_name.to_string()));
                    tracing::info!("Click at ({}, {}) with {} button", x, y, button_name);
                }
                PendingInput::DoubleClick { x, y, button } => {
                    let button_name = match button {
                        MouseButton::Left => "left",
                        MouseButton::Right => "right",
                        MouseButton::Middle => "middle",
                    };
                    self.last_double_click = Some((x, y, button_name.to_string()));
                    tracing::info!("Double click at ({}, {}) with {} button", x, y, button_name);
                }
                PendingInput::Drag {
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                    button: _,
                } => {
                    self.last_drag = Some(((start_x, start_y), (end_x, end_y)));
                    tracing::info!(
                        "Drag from ({}, {}) to ({}, {})",
                        start_x,
                        start_y,
                        end_x,
                        end_y
                    );
                }
                PendingInput::Keyboard { key } => {
                    self.last_key = Some(key.clone());
                    tracing::info!("Key pressed: {}", key);
                }
                PendingInput::Scroll {
                    x,
                    y,
                    delta_x,
                    delta_y,
                } => {
                    self.last_scroll = Some((x, y, delta_x, delta_y));
                    tracing::info!("Scroll at ({}, {}) delta ({}, {})", x, y, delta_x, delta_y);
                }
            }
        }
    }

    /// Encode ColorImage to PNG bytes
    fn encode_png(image: &egui::ColorImage) -> Option<Vec<u8>> {
        let mut png_data = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_data);

        // Convert RGBA pixels to bytes
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
            .collect();

        encoder
            .write_image(
                &pixels,
                image.width() as u32,
                image.height() as u32,
                image::ExtendedColorType::Rgba8,
            )
            .ok()?;

        Some(png_data)
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process pending MCP inputs
        self.process_pending_inputs();

        // Check if screenshot is requested and send viewport command
        let screenshot_requested = self
            .runtime
            .block_on(self.mcp_client.take_screenshot_request());
        if screenshot_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        // Handle screenshot events
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Screenshot { image, .. } = event {
                    if let Some(png_data) = Self::encode_png(image) {
                        tracing::info!("Screenshot captured: {} bytes", png_data.len());
                        let client = self.mcp_client.clone();
                        self.runtime.spawn(async move {
                            client.set_screenshot(png_data).await;
                        });
                    } else {
                        tracing::error!("Failed to encode screenshot as PNG");
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui-mcp Demo");

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.name);
            });

            ui.horizontal(|ui| {
                if ui.button("-").clicked() {
                    self.counter -= 1;
                }
                ui.label(format!("Counter: {}", self.counter));
                if ui.button("+").clicked() {
                    self.counter += 1;
                }
            });

            ui.checkbox(&mut self.checkbox_value, "Enable feature");

            if ui.button("Reset").clicked() {
                self.name.clear();
                self.counter = 0;
                self.checkbox_value = false;
            }

            ui.separator();
            ui.label(format!(
                "Hello, {}!",
                if self.name.is_empty() {
                    "World"
                } else {
                    &self.name
                }
            ));

            // MCP Input Visualization Section
            ui.separator();
            ui.heading("MCP Input Monitor");

            egui::Grid::new("mcp_input_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Mouse Position:");
                    if let Some((x, y)) = self.last_mouse_pos {
                        ui.label(format!("({:.1}, {:.1})", x, y));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();

                    ui.label("Last Click:");
                    if let Some((x, y, button)) = &self.last_click {
                        ui.label(format!("({:.1}, {:.1}) [{}]", x, y, button));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();

                    ui.label("Last Double Click:");
                    if let Some((x, y, button)) = &self.last_double_click {
                        ui.label(format!("({:.1}, {:.1}) [{}]", x, y, button));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();

                    ui.label("Last Drag:");
                    if let Some(((sx, sy), (ex, ey))) = self.last_drag {
                        ui.label(format!("({:.1}, {:.1}) → ({:.1}, {:.1})", sx, sy, ex, ey));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();

                    ui.label("Last Key:");
                    if let Some(key) = &self.last_key {
                        ui.label(format!("'{}'", key));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();

                    ui.label("Last Scroll:");
                    if let Some((x, y, dx, dy)) = self.last_scroll {
                        ui.label(format!(
                            "at ({:.1}, {:.1}) delta ({:.1}, {:.1})",
                            x, y, dx, dy
                        ));
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();
                });
        });
    }
}
