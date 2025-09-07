use egui::{Response, Ui};
use log::debug;
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder, TableDimensions};
use crate::spreadsheet::SpreadsheetSource;

pub struct SpreadsheetState {
    data_source: SpreadsheetSource,
}

impl Default for SpreadsheetState {
    fn default() -> Self {
        Self {
            data_source: SpreadsheetSource::new(),
        }
    }
}

pub fn show_table(ui: &mut Ui, state: &mut SpreadsheetState) -> (Response, Vec<Action>) {
    let data_source = &mut state.data_source;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .show(ui, &mut *data_source, |builder: &mut DeferredTableBuilder<SpreadsheetSource>| {

            builder.header(|header_builder| {

                let TableDimensions { row_count: _, column_count } = header_builder.current_dimensions();

                for index in 0..column_count {
                    let column_name = SpreadsheetSource::make_column_name(index);
                    header_builder
                        .column(index, column_name);
                }

                // header_builder.create_group("Group 1", Some([0,1,2]));
                // header_builder.create_group("Remainder", None);
            })
        })
}

pub fn handle_actions(actions: Vec<Action>, state: &mut SpreadsheetState) {
    for action in actions {
        debug!("action: {:?}", action);
        match action {
            Action::CellClicked(cell_index) => {
                println!("cell clicked: {:?}", cell_index);
            }
            Action::ColumnReorder { from, to } => {
                // we actually want to MOVE the column data itself, not re-order the columns
                state.data_source.move_column(from, to);
            }
            Action::RowReorder { from, to } => {
                // we actually want to MOVE the column data itself, not re-order the columns
                state.data_source.move_row(from, to);
            }
        }
    }
}
