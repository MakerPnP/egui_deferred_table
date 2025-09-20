extern crate core;

use egui::{Response, Ui, UiBuilder, ViewportBuilder};
use egui_deferred_table::{Action, DeferredTable};
use log::debug;
use shared::growing::{
    CellState, CellValue, GrowingSource, GrowingSourceAlternativeRenderer, GrowingSourceRenderer,
};

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Projections demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    inspection: bool,

    state: ProjectionsState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            state: ProjectionsState::default(),
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
            ui.label("This demonstrates using a single data-source, but with multiple independent");
            ui.label("table renderers, each renderer can have different filtering, ordering, and");
            ui.label("rendering behavior.");
        });
    }

    fn central_panel_content(&mut self, ui: &mut Ui) {
        let state = &mut self.state;

        // one set of controls, which controls the data source for two table instances
        show_controls(ui, state);

        ui.separator();

        for table_index in 0..2 {
            ui.scope_builder(UiBuilder::new().id_salt(ui.next_auto_id()), |ui| {
                ui.heading(format!("Table {}", table_index));
                egui::Resize::default()
                    .min_size((100.0, 100.0))
                    .default_size((640.0, 240.0))
                    .max_size((1024.0, 768.0))
                    .show(ui, |ui| {
                        let (_response, actions) = show_table(ui, state, table_index);

                        handle_actions(actions, state, table_index);
                    });

                ui.separator();
            });
        }

        ui.label("content below");
    }

    fn top_panel_content(&mut self, ui: &mut Ui) {
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.label("Projections demo");
            },
            |ui| {
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            },
        );
    }
}

// Define an enum for your renderer types
pub enum ProjectionRenderer {
    Standard(GrowingSourceRenderer),
    Alternative(GrowingSourceAlternativeRenderer),
}

pub struct ProjectionsState {
    data: GrowingSource<CellState<CellValue>>,
    renderers: Vec<ProjectionRenderer>,
}

impl Default for ProjectionsState {
    fn default() -> Self {
        Self {
            data: GrowingSource::default(),
            renderers: vec![
                ProjectionRenderer::Standard(GrowingSourceRenderer::default()),
                ProjectionRenderer::Alternative(GrowingSourceAlternativeRenderer::default()),
            ],
        }
    }
}

pub fn show_table(
    ui: &mut Ui,
    state: &mut ProjectionsState,
    table_index: usize,
) -> (Response, Vec<Action>) {
    let data_source = &mut state.data;
    let renderer = &mut state.renderers[table_index];

    // here, match arms are the same, but the render types are different
    match renderer {
        ProjectionRenderer::Standard(renderer) => {
            DeferredTable::new(ui.make_persistent_id(ui.id().with(table_index)))
                .zero_based_headers()
                .show(ui, data_source, renderer)
        }
        ProjectionRenderer::Alternative(renderer) => {
            DeferredTable::new(ui.make_persistent_id(ui.id().with(table_index)))
                .zero_based_headers()
                .show(ui, data_source, renderer)
        }
    }
}

pub fn show_controls(ui: &mut Ui, state: &mut ProjectionsState) {
    ui.horizontal(|ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            let (rows, columns) = state.data.dimensions();
            ui.label(format!("Size: {},{}", rows, columns));

            ui.separator();

            if ui.button("grow").clicked() {
                state.data.grow(1, 1);
            }
        });
    });
}

pub fn handle_actions(actions: Vec<Action>, _state: &mut ProjectionsState, table_index: usize) {
    for action in actions {
        debug!("table_index: {:?}, action: {:?}", table_index, action);
    }
}
