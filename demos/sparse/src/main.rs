extern crate core;

use egui::{Ui, ViewportBuilder};
use shared::sparse::ui::SparseTableState;

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Sparse data demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    inspection: bool,

    state: SparseTableState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            state: SparseTableState::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_panel_content(ui);
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.left_panel_content(ui);
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
    }
}

impl MyApp {
    fn left_panel_content(&mut self, ui: &mut Ui) {
        egui::ScrollArea::both().show(ui, |ui| {
            ui.label("Pan and scroll using mouse or scrollbars.");
            ui.label("Use the form to modify cells.");
            ui.label("The table adjusts dynamically as sparse data source is changed.");
            ui.label("For high performance, only the visible cells are rendered.");
        });
    }

    fn central_panel_content(&mut self, ui: &mut Ui) {
        let state = &mut self.state;

        shared::sparse::ui::show_controls(ui, state);

        ui.separator();

        egui::Resize::default()
            .min_size((100.0, 100.0))
            .default_size((640.0, 480.0))
            .max_size((1024.0, 768.0))
            .show(ui, |ui| {
                let (_response, actions) = shared::sparse::ui::show_table(ui, state);

                shared::sparse::ui::handle_actions(actions, state);
            });

        ui.separator();

        ui.label("content below");
    }

    fn top_panel_content(&mut self, ui: &mut Ui) {
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.label("Sparse data demo");
            },
            |ui| {
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            },
        );
    }
}
