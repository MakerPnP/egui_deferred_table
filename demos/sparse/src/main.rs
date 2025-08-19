extern crate core;

use std::cell::Cell;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt::Display;
use egui::{Color32, ViewportBuilder};
use egui_deferred_table::{CellIndex, DeferredTable, DeferredTableBuilder, DeferredTableDataSource, DeferredTableRenderer, TableDimensions};
use log::trace;

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

        println!("recalculated extents. max_row_index: {}, max_column_index: {}", max_row_index, max_column_index);

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

        let mut rng = fastrand::Rng::new();
        let mut name_gen = names::Generator::with_naming(names::Name::Plain);

        const MAX_ROWS: usize = 40;
        const MAX_COLUMNS: usize = 30;
        const MAX_CELL_VALUES: usize = 50;
        
        debug_assert!(MAX_CELL_VALUES <= MAX_ROWS * MAX_COLUMNS);
        
        (0..MAX_CELL_VALUES).for_each(|index| {
            let (row_index, column_index) = loop {
                let row_index = rng.usize(0..MAX_ROWS);
                let column_index = rng.usize(0..MAX_COLUMNS);
                let value = data.get(row_index, column_index);
                if value.is_none() {
                    break (row_index, column_index);
                }
            };

            let kind = rng.usize(0..3);
            let cell_kind= match kind {
                0 => CellKind::Float(rng.f32()),
                1 => CellKind::Boolean(rng.bool()),
                2 => CellKind::Text(name_gen.next().unwrap()),
                _ => unreachable!()
            };

            data.insert(row_index, column_index, cell_kind);
        });

        println!("data: {:?}", data);

        Self {
            inspection: false,
            data,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Sparse data demo");
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.label("content above");
            ui.separator();
            egui::Resize::default()
                .min_size((100.0, 100.0))
                .max_size((640.0, 480.0))
                .show(ui, |ui| {

                    let data_source = &self.data;

                    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                        .show(ui, data_source, |builder: &mut DeferredTableBuilder<'_, SparseMapSource<CellKind>>| {

                            builder.header(|header_builder| {

                                // no need to define every columns unless there's something specific
                                
                            })
                        });

                    for action in actions {
                        println!("{:?}", action);
                    }

                });

            ui.separator();
            ui.label("content below");

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
