use crate::growing::{CellState, CellValue, GrowingSource, GrowingSourceRenderer};
use egui::{Response, Ui};
use egui_deferred_table::{Action, DeferredTable};
use std::collections::BTreeSet;

pub struct GrowingTableState {
    data: GrowingSource<CellState<CellValue>>,
    renderer: GrowingSourceRenderer,
    row_selection: BTreeSet<usize>,
}

impl GrowingTableState {
    pub fn update_row_selection(&mut self, selection: BTreeSet<usize>) {
        self.row_selection = selection;
    }

    pub fn selected_rows(&self) -> &BTreeSet<usize> {
        &self.row_selection
    }
}

impl Default for GrowingTableState {
    fn default() -> Self {
        Self {
            data: GrowingSource::default(),
            renderer: GrowingSourceRenderer::default(),
            row_selection: BTreeSet::new(),
        }
    }
}

pub fn show_table(ui: &mut Ui, state: &mut GrowingTableState) -> (Response, Vec<Action>) {
    let data_source = &mut state.data;
    let renderer = &mut state.renderer;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .show(ui, data_source, renderer)
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
            if ui.button("shrink").clicked() {
                state.data.shrink(1, 1);
            }

            ui.separator();
            let selected_rows = state.selected_rows();
            let message = if selected_rows.is_empty() {
                "None".to_string()
            } else {
                format!(
                    "{}",
                    selected_rows
                        .iter()
                        .map(|it| it.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            ui.label(format!("Selected rows: {}", message))
        });
    });
}
