use crate::spreadsheet::SpreadsheetSource;
use egui::{Response, Ui};
use egui_deferred_table::{
    Action, AxisParameters, CellIndex, DeferredTable, DeferredTableDataSource,
};
use log::debug;

pub struct SpreadsheetState {
    data_source: SpreadsheetSource,
    value: Option<(CellIndex, String)>,
    automatic_recalculation: bool,

    column_parameters: Option<Vec<AxisParameters>>,
}

impl SpreadsheetState {
    pub fn is_automatic_recalculation_enabled(&self) -> bool {
        self.automatic_recalculation
    }

    pub fn needs_recalculation(&self) -> bool {
        self.data_source.requires_recalculation()
    }

    pub fn recalculate(&mut self) {
        self.data_source.recalculate();
    }

    pub fn build_and_show_table(&mut self, ui: &mut Ui) -> (Response, Vec<Action>) {
        let dimensions = self.data_source.get_dimensions();

        let rebuild = match &self.column_parameters {
            None => true,
            Some(column_parameters) => column_parameters.len() != dimensions.column_count,
        };

        if rebuild {
            let column_parameters = (0..dimensions.column_count)
                .map(|index| {
                    let column_name = SpreadsheetSource::make_column_name(index);
                    AxisParameters::default().name(column_name)
                })
                .collect();
            self.column_parameters = Some(column_parameters);
        }

        let column_params = self.column_parameters.as_ref().unwrap();

        DeferredTable::new(ui.make_persistent_id("table_1"))
            // in this example, the spreadsheet maintains the column parameters so we don't need
            // to build them every frame
            .column_parameters(column_params)
            .highlight_hovered_cell()
            .show(ui, &mut self.data_source)
    }
}

impl Default for SpreadsheetState {
    fn default() -> Self {
        Self {
            data_source: SpreadsheetSource::new(),
            value: None,
            automatic_recalculation: false,
            column_parameters: None,
        }
    }
}

pub fn show_table(ui: &mut Ui, state: &mut SpreadsheetState) -> (Response, Vec<Action>) {
    state.build_and_show_table(ui)
}

pub fn handle_actions(actions: Vec<Action>, state: &mut SpreadsheetState) {
    for action in actions {
        debug!("action: {:?}", action);
        match action {
            Action::CellClicked(cell_index) => {
                println!("cell clicked: {:?}", cell_index);
                state.value = state
                    .data_source
                    .get_cell_value(cell_index)
                    .map(|value| (cell_index, value.to_editable()));
            }
            Action::ColumnReorder { from, to } => {
                // we actually want to MOVE the column data itself, not re-order the columns
                state.data_source.move_column(from, to);
                state.value.take();
            }
            Action::RowReorder { from, to } => {
                // we actually want to MOVE the column data itself, not re-order the columns
                state.data_source.move_row(from, to);
                state.value.take();
            }
        }
    }
}

pub fn show_controls(ui: &mut Ui, state: &mut SpreadsheetState) {
    ui.horizontal(|ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(state.data_source.recalculation_required, |ui| {
                    if ui.button("Recalculate").clicked() {
                        state.data_source.recalculate();
                    }
                });

                ui.checkbox(&mut state.automatic_recalculation, "Automatic");
            });
        });

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Add row").clicked() {
                    state.data_source.add_row();
                }
                if ui.button("Add column").clicked() {
                    state.data_source.add_column();
                }
            });
        });

        if let Some((index, value_mut)) = state.value.as_mut() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Value");
                    if ui.text_edit_singleline(value_mut).changed() {
                        state.data_source.set_cell_value(index, &value_mut);
                    }
                });
            });
        }
    });
}
