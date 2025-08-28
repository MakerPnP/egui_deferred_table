extern crate core;

use egui::{Ui, ViewportBuilder};

use egui::Color32;
use egui::Response;
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder};
use egui_deferred_table::{
    CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions,
};
use fastrand::Rng;
use indexmap::map::IndexMap;
use log::debug;
use log::trace;
use names::Generator;
use std::cell::Cell;

pub fn generate_data(
    data: &mut SparseMapSource<CellKind>,
    max_rows: usize,
    max_columns: usize,
    max_cell_values: usize,
    rng: &mut Rng,
    name_gen: &mut Generator,
) {
    (0..max_cell_values).for_each(|_index| {
        let row_index = rng.usize(0..max_rows);
        let column_index = rng.usize(0..max_columns);

        let kind = rng.usize(0..3);
        let cell_kind = match kind {
            0 => CellKind::Float(rng.f32()),
            1 => CellKind::Boolean(rng.bool()),
            2 => CellKind::Text(name_gen.next().unwrap()),
            _ => unreachable!(),
        };

        data.insert(row_index, column_index, cell_kind);
    });

    trace!("data: {:?}", data);
}

#[derive(Debug)]
pub enum CellKind {
    Float(f32),
    Boolean(bool),
    Text(String),
}

#[derive(Debug)]
pub struct SparseMapSource<V> {
    sparse_map: IndexMap<usize, IndexMap<usize, V>>,

    // cached dimensions, lazily calculated
    extents: Cell<Option<TableDimensions>>,
}

impl<V> SparseMapSource<V> {
    pub fn new() -> Self {
        Self {
            sparse_map: IndexMap::new(),
            extents: Cell::new(None),
        }
    }

    /// insert a new value at the location, returning the previous value at the location, if any.
    pub fn insert(&mut self, row_index: usize, column_index: usize, value: V) -> Option<V> {
        let previous = self
            .sparse_map
            .entry(row_index)
            .or_default()
            .insert(column_index, value);
        if previous.is_none() {
            self.extents.set(None);
        }
        previous
    }

    pub fn get(&self, row_index: usize, column_index: usize) -> Option<&V> {
        self.sparse_map
            .get(&row_index)
            .and_then(|row| row.get(&column_index))
    }
}

impl<V> DeferredTableDataSource for SparseMapSource<V> {
    fn get_dimensions(&self) -> TableDimensions {
        if let Some(extents) = self.extents.get() {
            return extents;
        }

        let extents = self
            .sparse_map
            .iter()
            .fold(None, |extents, (row_number, row)| {
                let (mut max_row_index, mut max_column_index) =
                    extents.unwrap_or((0_usize, 0_usize));

                max_column_index = max_column_index.max(row.keys().fold(
                    0_usize,
                    |max_column_index_for_this_row, column_index| {
                        max_column_index_for_this_row.max(*column_index)
                    },
                ));
                max_row_index = max_row_index.max(*row_number);

                Some((max_row_index, max_column_index))
            });

        let Some((max_row_index, max_column_index)) = extents else {
            return TableDimensions::default();
        };

        trace!(
            "recalculated extents. max_row_index: {}, max_column_index: {}",
            max_row_index, max_column_index
        );

        let extents = TableDimensions {
            row_count: max_row_index + 1,
            column_count: max_column_index + 1,
        };

        self.extents.set(Some(extents));

        extents
    }
}

impl DeferredTableRenderer for SparseMapSource<CellKind> {
    fn render_cell(&self, ui: &mut egui::Ui, cell_index: CellIndex) {
        if let Some(value) = self.get(cell_index.row, cell_index.column) {
            match value {
                // use some arbitrary formatting and color so we can tell the difference between the data types
                CellKind::Float(value) => {
                    ui.colored_label(Color32::LIGHT_GREEN, format!("{:.2}", value));
                }
                CellKind::Boolean(value) => {
                    ui.add_enabled_ui(false, |ui| {
                        let mut value = *value;
                        ui.add(egui::Checkbox::without_text(&mut value));
                    });
                }
                CellKind::Text(value) => {
                    ui.colored_label(Color32::LIGHT_BLUE, value);
                }
            }
        }
    }
}

pub enum CellKindChoice {
    Float,
    Boolean,
    Text,
}

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

        const MAX_ROWS: usize = 40;
        const MAX_COLUMNS: usize = 30;
        const MAX_CELL_VALUES: usize = 50;

        generate_data(
            &mut data,
            MAX_ROWS,
            MAX_COLUMNS,
            MAX_CELL_VALUES,
            &mut rng,
            &mut name_gen,
        );

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
}

pub fn show_table(ui: &mut Ui, state: &mut SparseTableState) -> (Response, Vec<Action>) {
    let data_source = &state.data;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .show(
            ui,
            data_source,
            |builder: &mut DeferredTableBuilder<'_, SparseMapSource<CellKind>>| {
                builder.header(|_header_builder| {

                    // no need to define every column unless there's something specific
                })
            },
        )
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
}

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Sparse data demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    inspection: bool,

    state: SparseTableState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            state: SparseTableState::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = &mut self.state;
            let (_response, actions) = show_table(ui, state);
            handle_actions(actions, state);
        });
    }
}
