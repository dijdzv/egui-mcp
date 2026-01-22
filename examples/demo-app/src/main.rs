//! Demo egui application for testing egui-mcp
//!
//! This app demonstrates integration with egui-mcp-client for:
//! - Screenshots
//! - (Future) Coordinate-based input
//!
//! Note: UI tree access is handled via AT-SPI on the server side
//! and doesn't require any special code in the egui application.

use eframe::egui;
use egui_mcp_client::McpClient;
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
        });
    }
}
