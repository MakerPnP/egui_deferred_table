use egui::{Pos2, Ui, ViewportBuilder};

use egui::Color32;
use egui::Response;
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder};
use egui_deferred_table::{
    CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions,
};

use indexmap::map::IndexMap;

#[derive(Debug, Clone, Default)]
pub struct MyCell {
    pub count: i32,
}

impl MyCell {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

#[derive(Debug)]
pub struct MySource {
    data: IndexMap<usize, IndexMap<usize, MyCell>>,
    dimensions: TableDimensions,
}

impl MySource {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut data: IndexMap<usize, IndexMap<usize, MyCell>> = IndexMap::new();
        for row in 0..rows {
            for col in 0..cols {
                data.entry(row).or_default().insert(col, MyCell::new());
            }
        }
        Self {
            data,
            dimensions: TableDimensions {
                row_count: rows,
                column_count: cols,
            },
        }
    }

    pub fn get(&mut self, row_index: usize, column_index: usize) -> Option<&MyCell> {
        if let Some(row) = self.data.get_mut(&row_index) {
            if let Some(cell) = row.get_mut(&column_index) {
                cell.count += 1;
                Some(cell)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl DeferredTableDataSource for MySource {
    fn get_dimensions(&self) -> TableDimensions {
        self.dimensions
    }
}

impl DeferredTableRenderer for MySource {
    fn render_cell(&mut self, ui: &mut egui::Ui, cell_index: CellIndex) {
        if let Some(coord) = self.get(cell_index.row, cell_index.column) {
            ui.colored_label(Color32::LIGHT_BLUE, format!("{}", coord.count));
        }
    }
}

pub struct DenseTableState {
    pub data: MySource,
}

impl Default for DenseTableState {
    fn default() -> Self {
        Self {
            data: MySource::new(50, 50),
        }
    }
}

pub fn show_table(ui: &mut Ui, state: &mut DenseTableState) -> (Response, Vec<Action>) {
    let data_source = &mut state.data;

    DeferredTable::new(ui.make_persistent_id("table_1"))
        .zero_based_headers()
        .show(
            ui,
            data_source,
            |builder: &mut DeferredTableBuilder<'_, MySource>| {
                builder.header(|header_builder| {
                    for index in 0..10 {
                        header_builder
                            .column(index, format!("c{}", index))
                            .default_width(20.0);
                    }
                });
            },
        )
}

struct MyApp {
    frame_count: usize,
    state: DenseTableState,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            frame_count: 0,
            state: DenseTableState::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;
        egui::CentralPanel::default().show(ctx, |ui| {
            show_table(ui, &mut self.state);
        });
        if self.frame_count == 1 {
            // read the data and see the count, print it
            for (row_index, row) in self.state.data.data.iter() {
                for (column_index, cell) in row.iter() {
                    println!(
                        "frame #1 row: {}, column: {}, count: {}",
                        row_index, column_index, cell.count
                    );
                    if *row_index < 7 && *column_index < 5 {
                        assert_eq!(cell.count, 1i32);
                    } else {
                        assert_eq!(cell.count, 0i32);
                    }
                }
            }
        } else if self.frame_count == 2 {
            // read the data and see the count, print it
            for (row_index, row) in self.state.data.data.iter() {
                for (column_index, cell) in row.iter() {
                    println!(
                        "frame #2 row: {}, column: {}, count: {}",
                        row_index, column_index, cell.count
                    );
                    // TODO
                    // if *row_index < 7 && *column_index < 5 {
                    //     assert_eq!(cell.count, 1i32);
                    // } else {
                    //     assert_eq!(cell.count, 0i32);
                    // }
                }
            }
        } else if self.frame_count == 3 {
            // read the data and see the count, print it
            for (row_index, row) in self.state.data.data.iter() {
                for (column_index, cell) in row.iter() {
                    println!(
                        "frame #3 row: {}, column: {}, count: {}",
                        row_index, column_index, cell.count
                    );
                    // TODO
                    // if *row_index < 7 && *column_index < 5 {
                    //     assert_eq!(cell.count, 1i32);
                    // } else {
                    //     assert_eq!(cell.count, 0i32);
                    // }
                }
            }
        } else {
            println!("--------------------------------");
            println!("all ok, exiting");
            std::process::exit(0);
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.frame_count == 1 {
            // Activate click
            raw_input
                .events
                .push(egui::Event::PointerMoved(Pos2::new(10.0, 10.0)));
            // Scroll down and right
            raw_input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::Vec2::new(-1000.0, -1000.0),
                modifiers: egui::Modifiers::NONE,
            });
        }
    }
}

fn main() -> eframe::Result<()> {
    // set info log level
    log::set_max_level(log::LevelFilter::Info);
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([20.0 * 10.0, 20.0 * 10.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Scroll test",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}
