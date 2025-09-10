use crate::sparse::{CellKind, CellKindChoice, SparseMapSource, generate_data};
use egui::{Response, Ui};
use egui_deferred_table::{Action, DeferredTable, apply_reordering};
use fastrand::Rng;
use log::debug;
use names::Generator;

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

        const MAX_ROWS: usize = 30;
        const MAX_COLUMNS: usize = 15;
        const MAX_CELL_VALUES: usize = 100;

        generate_data(
            &mut data,
            MAX_ROWS,
            MAX_COLUMNS,
            MAX_CELL_VALUES,
            &mut rng,
            &mut name_gen,
        );

        for (index, (column, row)) in (0..MAX_COLUMNS).zip(0..MAX_ROWS).enumerate() {
            data.insert(
                row,
                column,
                CellKind::Text(format!("{} - {},{}", index, column, row)),
            );
        }

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

    row_ordering_input: String,
    column_ordering_input: String,
}

pub fn show_table(ui: &mut Ui, state: &mut SparseTableState) -> (Response, Vec<Action>) {
    let data_source = &mut state.data;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .highlight_hovered_cell()
        .show(ui, data_source, |builder| {
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
                        }
                        CellKind::Boolean(value) => {
                            state.ui_state.boolean_value = *value;
                            state.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                        }
                        CellKind::Text(value) => {
                            state.ui_state.text_value = value.clone();
                            state.ui_state.kind_choice = Some(CellKindChoice::Text);
                        }
                    }
                }
            }
            Action::ColumnReorder { from, to } => {
                apply_reordering(&mut state.data.column_ordering, from, to);

                // Update UI to reflect changes
                state.ui_state.column_ordering_input =
                    list_to_string(state.data.column_ordering.as_mut().unwrap());
            }
            Action::RowReorder { from, to } => {
                apply_reordering(&mut state.data.row_ordering, from, to);

                // Update UI to reflect changes
                state.ui_state.row_ordering_input =
                    list_to_string(state.data.row_ordering.as_mut().unwrap());
            }
        }
    }
}

pub fn show_controls(ui: &mut Ui, state: &mut SparseTableState) {
    ui.horizontal(|ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
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
                        if ui
                            .add(egui::Button::selectable(
                                matches!(state.ui_state.kind_choice, None),
                                "None",
                            ))
                            .clicked()
                        {
                            state.ui_state.kind_choice = None;
                        }
                        if ui
                            .add(egui::Button::selectable(
                                matches!(state.ui_state.kind_choice, Some(CellKindChoice::Float)),
                                "Float",
                            ))
                            .clicked()
                        {
                            state.ui_state.kind_choice = Some(CellKindChoice::Float);
                        }
                        if ui
                            .add(egui::Button::selectable(
                                matches!(state.ui_state.kind_choice, Some(CellKindChoice::Boolean)),
                                "Boolean",
                            ))
                            .clicked()
                        {
                            state.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                        }
                        if ui
                            .add(egui::Button::selectable(
                                matches!(state.ui_state.kind_choice, Some(CellKindChoice::Text)),
                                "Text",
                            ))
                            .clicked()
                        {
                            state.ui_state.kind_choice = Some(CellKindChoice::Text);
                        }
                    });

                match state.ui_state.kind_choice {
                    None => {}
                    Some(CellKindChoice::Boolean) => {
                        ui.add(egui::Checkbox::without_text(
                            &mut state.ui_state.boolean_value,
                        ));
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
                            CellKindChoice::Boolean => {
                                CellKind::Boolean(state.ui_state.boolean_value)
                            }
                            CellKindChoice::Text => {
                                CellKind::Text(state.ui_state.text_value.clone())
                            }
                        };
                        state
                            .data
                            .insert(state.ui_state.row, state.ui_state.column, value);
                    }
                });
            })
        });

        ui.separator();

        egui::Frame::group(ui.style()).show(ui, |ui| {
            if ui.button("Generate random data").clicked() {
                generate_data(
                    &mut state.data,
                    state.rng.usize(1..1000),
                    state.rng.usize(1..1000),
                    state.rng.usize(1..1000),
                    &mut state.rng,
                    &mut state.name_gen,
                );
            }
        });
    });

    ui.horizontal(|ui| {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            const FILTERING_HINT: &'static str = "0,3-7,42,6";

            ui.label("Filter rows");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut state.ui_state.filter_rows_input)
                        .desired_width(100.0)
                        .hint_text(FILTERING_HINT),
                )
                .changed()
            {
                state.data.rows_to_filter =
                    Some(range_string_to_list(&state.ui_state.filter_rows_input));
            }

            ui.label("Filter columns");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut state.ui_state.filter_columns_input)
                        .desired_width(100.0)
                        .hint_text(FILTERING_HINT),
                )
                .changed()
            {
                state.data.columns_to_filter =
                    Some(range_string_to_list(&state.ui_state.filter_columns_input));
            }
        });

        ui.separator();
        egui::Frame::group(ui.style()).show(ui, |ui| {
            const ORDERING_HINT: &'static str = "0,2,1";

            ui.label("Row ordering");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut state.ui_state.row_ordering_input)
                        .desired_width(100.0)
                        .hint_text(ORDERING_HINT),
                )
                .changed()
            {
                state.data.row_ordering = Some(string_to_list(&state.ui_state.row_ordering_input));
            }

            ui.label("Column ordering");
            if ui
                .add(
                    egui::TextEdit::singleline(&mut state.ui_state.column_ordering_input)
                        .desired_width(100.0)
                        .hint_text(ORDERING_HINT),
                )
                .changed()
            {
                state.data.column_ordering =
                    Some(string_to_list(&state.ui_state.column_ordering_input));
            }
        });
    });
}

#[allow(dead_code)]
fn list_to_string(list: &[usize]) -> String {
    list.iter()
        .map(|it| it.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn string_to_list(value: &String) -> Vec<usize> {
    value
        .split(",")
        .filter_map(|it| it.trim().parse::<usize>().ok())
        .collect()
}

/// converts user input string containing numbers and ranges.
///
/// input: '1,2,4-6,8', output: 'vec![1,2,4,5,6,8]
fn range_string_to_list(value: &String) -> Vec<usize> {
    let mut result = Vec::new();

    for part in value.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(start_num), Ok(end_num)) =
                (start.trim().parse::<usize>(), end.trim().parse::<usize>())
            {
                result.extend(start_num..=end_num);
            }
        } else if let Ok(num) = part.trim().parse::<usize>() {
            result.push(num);
        }
    }

    result
}

#[cfg(test)]
mod input_parsing_tests {
    use super::range_string_to_list;
    use rstest::rstest;

    #[rstest]
    #[case("", vec![])]
    #[case("1", vec![1])]
    #[case("1,2,3", vec![1, 2, 3])]
    #[case("1-3", vec![1, 2, 3])]
    #[case("1,2,4-6,8", vec![1, 2, 4, 5, 6, 8])]
    #[case(" 1 , 2 , 4 - 6 , 8 ", vec![1, 2, 4, 5, 6, 8])]
    #[case("1,,2,,", vec![1, 2])]
    #[case("1,abc,2", vec![1, 2])]
    #[case("1,2-abc,3", vec![1, 3])]
    #[case("1,abc-2,3", vec![1, 3])]
    #[case("10-15,20,25-27", vec![10, 11, 12, 13, 14, 15, 20, 25, 26, 27])]
    fn test_range_string_to_list(#[case] input: &str, #[case] expected: Vec<usize>) {
        assert_eq!(range_string_to_list(&input.to_string()), expected);
    }
}
