extern crate core;

use std::cell::Cell;
use indexmap::IndexMap;
use std::fmt::Display;
use egui::{Color32, Ui, ViewportBuilder};
use fastrand::Rng;
use egui_deferred_table::{Action, CellIndex, DeferredTable, DeferredTableBuilder, DeferredTableDataSource, DeferredTableRenderer, TableDimensions};
use log::{debug, trace};
use names::Generator;

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

#[derive(Debug)]
enum CellKind {
    Float(f32),
    Boolean(bool),
    Text(String),
}

struct MyApp {
    inspection: bool,

    data: SparseMapSource<CellKind>,
    
    ui_state: UiState,

    rng: Rng,
    name_gen: Generator<'static>,

}

#[derive(Debug)]
struct SparseMapSource<V> {
    sparse_map: IndexMap<usize, IndexMap<usize, V>>,

    // cached dimensions, lazily calculated
    extents: Cell<Option<TableDimensions>>,
}

impl<V> SparseMapSource<V> {
    fn new() -> Self {
        Self {
            sparse_map: IndexMap::new(),
            extents: Cell::new(None),
        }
    }

    /// insert a new value at the location, returning the previous value at the location, if any.
    fn insert(&mut self, row_index: usize, column_index: usize, value: V) -> Option<V> {
        let previous = self.sparse_map.entry(row_index).or_default().insert(column_index, value);
        if previous.is_none() {
            self.extents.set(None);
        }
        previous
    }

    fn get(&self, row_index: usize, column_index: usize) -> Option<&V> {
        self.sparse_map.get(&row_index).and_then(|row| row.get(&column_index))
    }
}

impl<V> DeferredTableDataSource for SparseMapSource<V> {
    fn get_dimensions(&self) -> TableDimensions {
        if let Some(extents) = self.extents.get() {
            return extents;
        }

        let extents = self.sparse_map.iter().fold(
            None,
            |extents, (row_number, row)|
            {
                let (mut max_row_index, mut max_column_index) = extents.unwrap_or((0_usize, 0_usize));

                max_column_index = max_column_index.max(row.keys().fold(0_usize, |max_column_index_for_this_row, column_index| max_column_index_for_this_row.max(*column_index)));
                max_row_index = max_row_index.max(*row_number);

                Some((max_row_index, max_column_index))
            });

        let Some((max_row_index, max_column_index)) = extents else {
            return TableDimensions::default()
        };

        trace!("recalculated extents. max_row_index: {}, max_column_index: {}", max_row_index, max_column_index);

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
                    ui.add_enabled_ui(false, |ui|{
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

impl Default for MyApp {
    fn default() -> Self {

        let mut data = SparseMapSource::new();

        let mut rng = Rng::new();
        let mut name_gen = Generator::with_naming(names::Name::Plain);

        const MAX_ROWS: usize = 40;
        const MAX_COLUMNS: usize = 30;
        const MAX_CELL_VALUES: usize = 50;

        generate_data(&mut data, MAX_ROWS, MAX_COLUMNS, MAX_CELL_VALUES, &mut rng, &mut name_gen);
        
        Self {
            inspection: false,
            data,
            ui_state: UiState::default(),
            rng,
            name_gen,
        }
    }
}

fn generate_data(data: &mut SparseMapSource<CellKind>, max_rows: usize, max_columns: usize, max_cell_values: usize, rng: &mut Rng, name_gen: &mut Generator) {
    
    (0..max_cell_values).for_each(|_index| {
        let row_index = rng.usize(0..max_rows);
        let column_index = rng.usize(0..max_columns);

        let kind = rng.usize(0..3);
        let cell_kind= match kind {
            0 => CellKind::Float(rng.f32()),
            1 => CellKind::Boolean(rng.bool()),
            2 => CellKind::Text(name_gen.next().unwrap()),
            _ => unreachable!()
        };

        data.insert(row_index, column_index, cell_kind);
    });

    trace!("data: {:?}", data);
}

#[derive(Default)]
struct UiState {
    column: usize,
    row: usize,
    
    float_value: f32,
    boolean_value: bool,
    text_value: String,
    
    kind_choice: Option<CellKindChoice>
}

enum CellKindChoice {
    Float,
    Boolean,
    Text,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_panel_content(ui);
        });
        
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.left_panel_content(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel_content(ui);
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

impl MyApp {
    fn left_panel_content(&mut self, ui: &mut Ui) {
        egui::ScrollArea::both()
            .show(ui, |ui| {
                ui.label("Pan and scroll using mouse or scrollbars.");
                ui.label("Use the form to modify cells.");
                ui.label("The table adjusts dynamically as sparse data source is changed.");
                ui.label("For high performance, only the visible cells are rendered.");
            });
    }

    fn show_insert_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Row");
                        ui.add(egui::DragValue::new(&mut self.ui_state.column));
                        ui.label("Column");
                        ui.add(egui::DragValue::new(&mut self.ui_state.row));

                        ui.label("Kind");
                        egui::ComboBox::from_id_salt("kind_choice")
                            .selected_text(match self.ui_state.kind_choice {
                                None => "Select...",
                                Some(CellKindChoice::Float) => "Float",
                                Some(CellKindChoice::Boolean) => "Boolean",
                                Some(CellKindChoice::Text) => "Text",
                            })
                            .show_ui(ui, |ui| {
                                if ui.add(egui::Button::selectable(matches!(self.ui_state.kind_choice, None), "None")).clicked() {
                                    self.ui_state.kind_choice = None;
                                }
                                if ui.add(egui::Button::selectable(matches!(self.ui_state.kind_choice, Some(CellKindChoice::Float)), "Float")).clicked() {
                                    self.ui_state.kind_choice = Some(CellKindChoice::Float);
                                }
                                if ui.add(egui::Button::selectable(matches!(self.ui_state.kind_choice, Some(CellKindChoice::Boolean)), "Boolean")).clicked() {
                                    self.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                                }
                                if ui.add(egui::Button::selectable(matches!(self.ui_state.kind_choice, Some(CellKindChoice::Text)), "Text")).clicked() {
                                    self.ui_state.kind_choice = Some(CellKindChoice::Text);
                                }
                            });

                        match self.ui_state.kind_choice {
                            None => {}
                            Some(CellKindChoice::Boolean) => {
                                ui.add(egui::Checkbox::without_text(&mut self.ui_state.boolean_value));
                            }
                            Some(CellKindChoice::Float) => {
                                ui.add(egui::DragValue::new(&mut self.ui_state.float_value));
                            }
                            Some(CellKindChoice::Text) => {
                                ui.add(egui::TextEdit::singleline(&mut self.ui_state.text_value));
                            }
                        }

                        ui.add_enabled_ui(self.ui_state.kind_choice.is_some(), |ui| {
                            if ui.button("Apply").clicked() {
                                let value = match self.ui_state.kind_choice.as_ref().unwrap() {
                                    CellKindChoice::Float => CellKind::Float(self.ui_state.float_value),
                                    CellKindChoice::Boolean => CellKind::Boolean(self.ui_state.boolean_value),
                                    CellKindChoice::Text => CellKind::Text(self.ui_state.text_value.clone()),
                                };
                                self.data.insert(self.ui_state.row, self.ui_state.column, value);
                            }
                        });
                    })
                });

            ui.separator();

            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    if ui.button("Generate random data").clicked() {
                        generate_data(
                            &mut self.data, 
                            self.rng.usize(1..1000),
                            self.rng.usize(1..1000),
                            self.rng.usize(1..1000),
                            &mut self.rng,
                            &mut self.name_gen
                        );
                    }
                });
        });
    }

    fn central_panel_content(&mut self, ui: &mut Ui) {
        self.show_insert_controls(ui);
        
        ui.separator();
        
        egui::Resize::default()
            .min_size((100.0, 100.0))
            .max_size((640.0, 480.0))
            .show(ui, |ui| {
                let data_source = &self.data;

                let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                    .zero_based_headers()
                    .show(ui, data_source, |builder: &mut DeferredTableBuilder<'_, SparseMapSource<CellKind>>| {
                        builder.header(|header_builder| {

                            // no need to define every column unless there's something specific

                        })
                    });

                for action in actions {
                    match action {
                        Action::CellClicked(cell_index) => {
                            self.ui_state.column = cell_index.column;
                            self.ui_state.row = cell_index.row;
                            
                            if let Some(value) = self.data.get(cell_index.row, cell_index.column) {
                                match value {
                                    CellKind::Float(value) => {
                                        self.ui_state.float_value = *value;
                                        self.ui_state.kind_choice = Some(CellKindChoice::Float);
                                    },
                                    CellKind::Boolean(value) => {
                                        self.ui_state.boolean_value = *value;
                                        self.ui_state.kind_choice = Some(CellKindChoice::Boolean);
                                    },
                                    CellKind::Text(value) => {
                                        self.ui_state.text_value = value.clone();
                                        self.ui_state.kind_choice = Some(CellKindChoice::Text);
                                    },
                                }
                            }
                        }
                    }
                }
            });

        ui.separator();
        
        ui.label("content below");
    }

    fn top_panel_content(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Sparse data demo");
            ui.checkbox(&mut self.inspection, "🔍 Inspection");
        });
    }
}


#[cfg(test)]
mod sparse_map_source_tests {
    use egui_deferred_table::{DeferredTableDataSource, TableDimensions};
    use crate::SparseMapSource;

    #[test]
    pub fn dimensions_for_empty_source() {
        // given
        let source = SparseMapSource::<usize>::new();
        // when
        assert_eq!(source.get_dimensions(), TableDimensions { row_count: 0, column_count: 0 });
    }

    #[test]
    pub fn dimensions_for_1x2_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(0, 1, 69);

        // when
        assert_eq!(source.get_dimensions(), TableDimensions { row_count: 1, column_count: 2 });
    }

    #[test]
    pub fn dimensions_for_2x1_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(1, 0, 69);

        // when
        assert_eq!(source.get_dimensions(), TableDimensions { row_count: 2, column_count: 1 });
    }
    
    #[test]
    pub fn dimensions_for_2x2_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(0, 1, 69);

        source.insert(1, 0, 0x42);
        source.insert(1, 1, 0x69);

        // when
        assert_eq!(source.get_dimensions(), TableDimensions { row_count: 2, column_count: 2 });
    }

    #[test]
    pub fn dimensions_for_sparse_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(4, 9, 0x42);

        source.insert(0, 0, 42);
        source.insert(1, 1, 69);

        source.insert(9, 4, 0x69);

        // when
        assert_eq!(source.get_dimensions(), TableDimensions { row_count: 10, column_count: 10 });
    }
}
