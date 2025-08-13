extern crate core;

use std::fmt::Display;
use egui::{ViewportBuilder};
use egui_deferred_table::{DeferredTable, DeferredTableBuilder, TableValue};

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Fixed data demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

#[derive(Debug)]
enum Kind {
    Human,
    Alien,
    Mutant,
    Robot,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

type RowType = (String, Kind, usize, f32);

impl TableValue for Kind {}

struct MyApp {
    inspection: bool,

    data: Vec<RowType>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            data: vec![
                ("Fry".to_string(), Kind::Human, 30, 69.0),
                ("Leela".to_string(), Kind::Mutant, 32, 42.0),
                ("Bender".to_string(), Kind::Robot, 28, 42.0),
                ("Zoidberg".to_string(), Kind::Alien, 40, 42.0),
                ("Nibbler".to_string(), Kind::Alien, 69, 42.0),
                ("Farnsworth".to_string(), Kind::Human, 90, 42.0),
            ],
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Fixed data demo");
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.label("content above");
            ui.separator();
            egui::ScrollArea::both()
                .show(ui, |ui| {

                    let data_source = self.data.as_slice();

                    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                        .show(ui, &data_source, |builder: &mut DeferredTableBuilder<'_, &[RowType]>| {

                            builder.header(|header_builder| {

                                header_builder
                                    .column(0, "Name".to_string());
                                header_builder
                                    .column(1, "Kind".to_string());
                                header_builder
                                    .column(2, "usize".to_string());
                                header_builder
                                    .column(3, "f32".to_string());
                            })
                        });

                    for action in actions {
                        println!("{:?}", action);
                    }

                });

            ui.separator();
            ui.label("content below");

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
