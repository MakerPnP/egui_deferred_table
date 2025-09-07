use crate::growing::{CellState, CellValue, GrowingSource};
use egui::{Response, Ui};
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder};

pub struct GrowingTableState {
    data: GrowingSource<CellState<CellValue>>,
}

impl Default for GrowingTableState {
    fn default() -> Self {
        Self {
            data: GrowingSource::default(),
        }
    }
}

pub fn show_table(ui: &mut Ui, state: &mut GrowingTableState) -> (Response, Vec<Action>) {
    let data_source = &mut state.data;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .show(
            ui,
            data_source,
            |builder: &mut DeferredTableBuilder<'_, GrowingSource<CellState<CellValue>>>| {
                builder.header(|_header_builder| {

                    // no need to define every column unless there's something specific
                })
            },
        )
}

pub fn show_controls(ui: &mut Ui, state: &mut GrowingTableState) {
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
