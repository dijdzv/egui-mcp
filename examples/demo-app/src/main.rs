//! Demo egui application for testing egui-mcp

use eframe::egui;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-mcp Demo App",
        options,
        Box::new(|_cc| Ok(Box::new(DemoApp::default()))),
    )
}

#[derive(Default)]
struct DemoApp {
    name: String,
    counter: i32,
    checkbox_value: bool,
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
            ui.label(format!("Hello, {}!", if self.name.is_empty() { "World" } else { &self.name }));
        });
    }
}
