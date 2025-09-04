use egui::{Response, Ui};
use fastrand::Rng;
use log::debug;
use names::Generator;
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder};
use crate::sparse::{generate_data, CellKind, CellKindChoice, SparseMapSource};

pub struct SparseTableState {
    data: SparseMapSource<CellKind>,

    ui_state: UiState,

    rng: Rng,
    name_gen: Generator<'static>,
}

impl Default for SparseTableState {
    fn default() -> Self {
        let mut data = SparseMapSource::new();

        let mut rng = Rng::new();
        let mut name_gen = Generator::with_naming(names::Name::Plain);

        const MAX_ROWS: usize = 15;
        const MAX_COLUMNS: usize = 15;
        const MAX_CELL_VALUES: usize = 50;

        generate_data(&mut data, MAX_ROWS, MAX_COLUMNS, MAX_CELL_VALUES, &mut rng, &mut name_gen);

        Self {
            data,
            ui_state: UiState::default(),
            rng,
            name_gen,
        }
    }
}

#[derive(Default)]
struct UiState {
    column: usize,
    row: usize,

    float_value: f32,
    boolean_value: bool,
    text_value: String,

    kind_choice: Option<CellKindChoice>,

    filter_rows_input: String,
    filter_columns_input: String,
    filter_rows: Vec<usize>,
    filter_columns: Vec<usize>,
}

pub fn show_table(ui: &mut Ui, state: &mut SparseTableState) -> (Response, Vec<Action>) {

    let data_source = &mut state.data;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .filter_rows(&state.ui_state.filter_rows)
        .filter_columns(&state.ui_state.filter_columns)
        .show(ui, data_source, |builder: &mut DeferredTableBuilder<'_, SparseMapSource<CellKind>>| {
            builder.header(|_header_builder| {

                // no need to define every column unless there's something specific

            })
        })
}

pub fn handle_actions(actions: Vec<Action>, state: &mut SparseTableState) {
    for action in actions {
        debug!("action: {:?}", action);
        match action {
            Action::CellClicked(cell_index) => {
                state.ui_state.column = cell_index.column;
                state.ui_state.row = cell_index.row;

                if let Some(value) = state.data.get(cell_index.row, cell_index.column) {
                    match value {
                        CellKind::Float(value) => {
                            state.ui_state.float_value = *value;
                            state.ui_state.kind_choice = Some(CellKindChoice::Float);
                        },
                        CellKind::Boolean(value) => {
                            state.ui_state.boolean_value = *value;
                            state.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                        },
                        CellKind::Text(value) => {
                            state.ui_state.text_value = value.clone();
                            state.ui_state.kind_choice = Some(CellKindChoice::Text);
                        },
                    }
                }
            }
        }
    }
}

pub fn show_controls(ui: &mut Ui, state: &mut SparseTableState) {
    ui.horizontal(|ui| {
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Row");
                    ui.add(egui::DragValue::new(&mut state.ui_state.column));
                    ui.label("Column");
                    ui.add(egui::DragValue::new(&mut state.ui_state.row));

                    ui.label("Kind");
                    egui::ComboBox::from_id_salt("kind_choice")
                        .selected_text(match state.ui_state.kind_choice {
                            None => "Select...",
                            Some(CellKindChoice::Float) => "Float",
                            Some(CellKindChoice::Boolean) => "Boolean",
                            Some(CellKindChoice::Text) => "Text",
                        })
                        .show_ui(ui, |ui| {
                            if ui.add(egui::Button::selectable(matches!(state.ui_state.kind_choice, None), "None")).clicked() {
                                state.ui_state.kind_choice = None;
                            }
                            if ui.add(egui::Button::selectable(matches!(state.ui_state.kind_choice, Some(CellKindChoice::Float)), "Float")).clicked() {
                                state.ui_state.kind_choice = Some(CellKindChoice::Float);
                            }
                            if ui.add(egui::Button::selectable(matches!(state.ui_state.kind_choice, Some(CellKindChoice::Boolean)), "Boolean")).clicked() {
                                state.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                            }
                            if ui.add(egui::Button::selectable(matches!(state.ui_state.kind_choice, Some(CellKindChoice::Text)), "Text")).clicked() {
                                state.ui_state.kind_choice = Some(CellKindChoice::Text);
                            }
                        });

                    match state.ui_state.kind_choice {
                        None => {}
                        Some(CellKindChoice::Boolean) => {
                            ui.add(egui::Checkbox::without_text(&mut state.ui_state.boolean_value));
                        }
                        Some(CellKindChoice::Float) => {
                            ui.add(egui::DragValue::new(&mut state.ui_state.float_value));
                        }
                        Some(CellKindChoice::Text) => {
                            ui.add(egui::TextEdit::singleline(&mut state.ui_state.text_value));
                        }
                    }

                    ui.add_enabled_ui(state.ui_state.kind_choice.is_some(), |ui| {
                        if ui.button("Apply").clicked() {
                            let value = match state.ui_state.kind_choice.as_ref().unwrap() {
                                CellKindChoice::Float => CellKind::Float(state.ui_state.float_value),
                                CellKindChoice::Boolean => CellKind::Boolean(state.ui_state.boolean_value),
                                CellKindChoice::Text => CellKind::Text(state.ui_state.text_value.clone()),
                            };
                            state.data.insert(state.ui_state.row, state.ui_state.column, value);
                        }
                    });
                })
            });

        ui.separator();

        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                if ui.button("Generate random data").clicked() {
                    generate_data(
                        &mut state.data,
                        state.rng.usize(1..1000),
                        state.rng.usize(1..1000),
                        state.rng.usize(1..1000),
                        &mut state.rng,
                        &mut state.name_gen
                    );
                }
            });
    });

    ui.horizontal(|ui| {
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Filter rows");
                if ui.add(egui::TextEdit::singleline(&mut state.ui_state.filter_rows_input)
                    .hint_text("Comma separated list of row indices")).changed() {
                    state.ui_state.filter_rows = string_to_list(&state.ui_state.filter_rows_input);
                }

                ui.label("Filter columns");
                if ui.add(egui::TextEdit::singleline(&mut state.ui_state.filter_columns_input)
                    .hint_text("Comma separated list of column indices")).changed() {
                    state.ui_state.filter_columns = string_to_list(&state.ui_state.filter_columns_input);
                }
            });
    });
}

#[allow(dead_code)]
fn list_to_string(list: &[usize]) -> String {
    list.iter().map(|it|it.to_string()).collect::<Vec<_>>().join(",")
}

fn string_to_list(value: &String) -> Vec<usize> {
    value.split(",").filter_map(|it| it.parse::<usize>().ok()).collect()
}
