extern crate core;

use egui::{Ui, ViewportBuilder};
use shared::spreadsheet::ui::SpreadsheetState;

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Spreadsheet demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    inspection: bool,

    state: SpreadsheetState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            state: SpreadsheetState::default(),
        }
    }
}

impl MyApp {
    fn top_panel_content(&mut self, ui: &mut Ui) {
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.label("Spreadsheet demo");
            },
            |ui| {
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            },
        );
    }

    fn central_panel_content(&mut self, ui: &mut Ui) {
        shared::spreadsheet::ui::show_controls(ui, &mut self.state);

        ui.label("content above");
        ui.separator();
        egui::ScrollArea::both().show(ui, |ui| {
            let (_response, actions) = shared::spreadsheet::ui::show_table(ui, &mut self.state);

            shared::spreadsheet::ui::handle_actions(actions, &mut self.state);
        });

        ui.separator();
        ui.label("content below");
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_panel_content(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel_content(ui);
        });

        // Inspection window
        egui::Window::new("🔍 Inspection")
            .open(&mut self.inspection)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });

        if self.state.is_automatic_recalculation_enabled() && self.state.needs_recalculation() {
            self.state.recalculate();
            ctx.request_repaint();
        }
    }
}
