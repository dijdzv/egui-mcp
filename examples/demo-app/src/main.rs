//! Demo egui application for testing egui-mcp

use eframe::egui;
use egui_mcp_client::{McpClient, UiTreeBuilder};
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
    fn new(
        _cc: &eframe::CreationContext<'_>,
        mcp_client: McpClient,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self {
            name: String::new(),
            counter: 0,
            checkbox_value: false,
            mcp_client,
            runtime,
        }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // Update UI tree from AccessKit
        ctx.output(|output| {
            if let Some(ref update) = output.accesskit_update {
                let tree = UiTreeBuilder::from_accesskit(update);
                let client = self.mcp_client.clone();
                self.runtime.spawn(async move {
                    client.update_ui_tree(tree).await;
                });
            }
        });
    }
}
