use crate::spreadsheet::{SpreadsheetRenderer, SpreadsheetSource};
use egui::{Response, Ui};
use egui_deferred_table::{
    Action, AxisParameters, CellIndex, DeferredTable, DeferredTableDataSource,
    EditableTableRenderer, EditorState,
};
use log::debug;

pub struct SpreadsheetState {
    data_source: SpreadsheetSource,
    renderer: SpreadsheetRenderer,
    editor: SpreadsheetEditor,
    edit_state: EditorState<String, String>,

    value: Option<(CellIndex, String)>,
    automatic_recalculation: bool,

    column_parameters: Option<Vec<AxisParameters>>,
    row_parameters: Option<Vec<AxisParameters>>,
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

        let rebuild_column_parameters = match &self.column_parameters {
            None => true,
            Some(column_parameters) => column_parameters.len() != dimensions.column_count,
        };

        if rebuild_column_parameters {
            let column_parameters = (0..dimensions.column_count)
                .map(|index| {
                    let column_name = SpreadsheetSource::make_column_name(index);
                    AxisParameters::default().monospace(true).name(column_name)
                })
                .collect();
            self.column_parameters = Some(column_parameters);
        }

        let rebuild_row_parameters = match &self.row_parameters {
            None => true,
            Some(row_parameters) => row_parameters.len() != dimensions.row_count,
        };

        if rebuild_row_parameters {
            // This is an example of an expensive operation (ilog10) which you do NOT want to repeat for every single
            // row and where you only want to re-run this block when the number of rows changes.

            let digits_required = match dimensions.row_count {
                0 => 1, // Handle empty tables (though unlikely to be called)
                n => n.ilog10() as usize + 1,
            };

            let row_parameters = (0..dimensions.row_count)
                .map(|index| {
                    let row_name = SpreadsheetSource::make_row_name(index, digits_required);
                    AxisParameters::default().monospace(true).name(row_name)
                })
                .collect();
            self.row_parameters = Some(row_parameters);
        }

        let column_params = self.column_parameters.as_ref().unwrap();
        let row_params = self.row_parameters.as_ref().unwrap();

        DeferredTable::new(ui.make_persistent_id("table_1"))
            // in this example, the spreadsheet maintains the column parameters so we don't need
            // to build them every frame
            .column_parameters(column_params)
            .row_parameters(row_params)
            .highlight_hovered_cell()
            .selectable_rows_disabled()
            .show_and_edit(
                ui,
                &mut self.data_source,
                &mut self.renderer,
                &mut self.editor,
                &mut self.edit_state,
            )
    }
}

impl Default for SpreadsheetState {
    fn default() -> Self {
        Self {
            data_source: SpreadsheetSource::new(),
            renderer: SpreadsheetRenderer::default(),
            editor: SpreadsheetEditor::default(),
            edit_state: EditorState::default(),
            value: None,
            automatic_recalculation: false,
            column_parameters: None,
            row_parameters: None,
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
            Action::RowSelectionChanged { selection } => {
                let _ = selection;
                // row selection currently disabled
                unreachable!()
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

#[derive(Default)]
struct SpreadsheetEditor {}

impl EditableTableRenderer<SpreadsheetSource> for SpreadsheetEditor {
    // in a spreadsheet all the cells are the same type, and all cells can be edited using a string
    // conversion from a string back to a CellValue happens when editing is finished
    type Value = String;
    type ItemState = String;

    fn build_item_state(
        &self,
        cell_index: CellIndex,
        source: &mut SpreadsheetSource,
    ) -> Option<(Self::ItemState, Self::Value)> {
        let value = source.get_cell_value(cell_index).unwrap().to_editable();

        Some((value.clone(), value))
    }

    fn on_edit_complete(
        &mut self,
        index: CellIndex,
        state: Self::ItemState,
        _original_item: Self::Value,
        source: &mut SpreadsheetSource,
    ) {
        source.set_cell_value(&index, &state);
    }

    fn render_cell_editor(
        &self,
        ui: &mut Ui,
        cell_index: &CellIndex,
        state: &mut Self::ItemState,
        _original_item: &Self::Value,
        source: &mut SpreadsheetSource,
    ) {
        let editor = ui.add(
            egui::TextEdit::singleline(state)
                .min_size(ui.available_size())
                .frame(false),
        );

        editor.request_focus();

        if editor.changed() {
            // Note: here we attempt to use the value, regardless of if it's a valid formula, etc.
            //       this gives us a 'live-update' functionality.
            //       we could just do this once `on_edit_complete`, but then the spreadsheet wouldn't change as the user types
            source.set_cell_value(&cell_index, &state);
        }
    }
}
