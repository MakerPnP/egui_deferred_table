extern crate core;

use std::fmt::Display;
use egui::{ViewportBuilder};
use egui_deferred_table::{DeferredTable, DeferredTableBuilder};
use crate::futurama::{Kind, RowType};

mod futurama;

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

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

struct MyApp {
    inspection: bool,

    data: Vec<RowType>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            data: futurama::characters(),
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

            ui.label("content above scroll area");
            ui.separator();
            
            egui::ScrollArea::both()
                .max_height(400.0)
                .show(ui, |ui| {
                    // FIXME the table renders on top of this
                    ui.label("content above table, inside scroll area");
                    
                    let data_source = self.data.as_slice();

                    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                        .show(ui, &data_source, |builder: &mut DeferredTableBuilder<'_, &[RowType]>| {

                            builder.header(|header_builder| {

                                for (index, field) in futurama::fields().iter().enumerate() {
                                    header_builder
                                        .column(index, field.to_string());
                                }
                            })
                        });

                    for action in actions {
                        println!("{:?}", action);
                    }

                    ui.label("content below table, inside scroll area");
                });
            
            ui.separator();
            ui.label("content below scroll area");

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
