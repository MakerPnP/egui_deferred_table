extern crate core;

use std::sync::{Arc, Mutex};
use egui::{Ui, ViewportBuilder};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use egui_deferred_table::{CellIndex, DeferredTable, DeferredTableBuilder, DeferredTableDataSource, TableDimensions};

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - Spreadsheet demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp {
    inspection: bool,
    
    data: Arc<Mutex<MySource>>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            inspection: false,
            data: Arc::new(Mutex::new(MySource::new())),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Spreadsheet demo");
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            
            ui.label("content above");
            ui.separator();
            egui::ScrollArea::both()
                .show(ui, |ui| {

                    let mut data_source = self.data.lock().unwrap();
                    
                    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                        .show(ui, &mut *data_source, |builder: &mut DeferredTableBuilder<MySource>| {
                            
                            builder.header(|header_builder| {
                                
                                let TableDimensions { row_count: _, column_count } = header_builder.current_dimensions();

                                for index in 0..column_count {
                                    let column_name = MySource::make_column_name(index);
                                    header_builder
                                        .column(index, column_name);
                                }

                                // header_builder.create_group("Group 1", Some([0,1,2]));
                                // header_builder.create_group("Remainder", None);
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

struct MySource {
    data: Vec<Vec<Value>>,
}

impl MySource {
    pub fn new() -> Self {
        let data = vec![
            vec![
                Value::Text("Message".to_string()), 
                Value::Text("Value 1".to_string()), 
                Value::Text("Value 2".to_string()), 
                Value::Text("Result".to_string()),
            ],
            vec![
                Value::Text("Hello World".to_string()),
                Value::Decimal(dec!(42.0)),
                Value::Decimal(dec!(69.0)),
                Value::Text("=B2+C2".to_string())
            ],
            vec![
                Value::Text("Example data".to_string()),
                Value::Decimal(dec!(6.0)),
                Value::Decimal(dec!(9.0)),
                Value::Text("=B3+C3".to_string())
            ],
            
        ];

        Self {
            data
        }
    }

    fn calculate_value(&self, _formula: &Formula) -> FormulaResult {
        FormulaResult::Error("#NOT_IMPLEMENTED".to_string())
    }

    fn build_value(&self, value: Value) -> CellValue {
        match value {
            Value::Text(text) => {
                if text.starts_with("=") {
                    let formula = Formula::new(text);
                    let result = self.calculate_value(&formula);

                    CellValue::Calculated(formula, result)
                } else {
                    CellValue::Value(Value::Text(text))
                }
            }
            value @ Value::Decimal(_) => CellValue::Value(value)
        }
    }

    pub fn render_error(&self, ui: &mut Ui, message: String) {
        ui.colored_label(egui::Color32::RED, &message);
    }
    
    pub fn render_value(&self, ui: &mut Ui, value: Value) {
        match value {
            Value::Text(text) => {
                ui.label(text);   
            }
            Value::Decimal(decimal) => {
                ui.label(decimal.to_string());
            }
        }
    }

    fn get_cell_value(&self, cell_index: CellIndex) -> Option<CellValue> {
        let row_values = &self.data[cell_index.row];

        let cell_value = row_values.get(cell_index.column)
            .map(|value| self.build_value(value.clone()));

        cell_value
    }
    
    // given '0' the result is 'A', '25' is 'Z', given '26' the result is 'AA', given '27' the result is 'AB' and so on.
    fn make_column_name(index: usize) -> String {
        let mut result = String::new();
        let mut n = index + 1; // Add 1 to avoid special case for index 0

        while n > 0 {
            // Get the current character (remainder when divided by 26)
            let remainder = ((n - 1) % 26) as u8;
            // Convert to corresponding ASCII character (A-Z)
            let c = (b'A' + remainder) as char;
            // Prepend to result (we build the string from right to left)
            result.insert(0, c);
            // Integer division to get the next "digit"
            n = (n - 1) / 26;
        }

        result
    }
}

impl DeferredTableDataSource for MySource {
    fn get_dimensions(&self) -> TableDimensions {
        let rows  =self.data.len();
        let columns = self.data.iter().fold(0, |acc, row| {
            row.len().max(acc)
        });

        TableDimensions {
            row_count: rows,
            column_count: columns
        }
        
    }

    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex) {
        let possible_value = self.get_cell_value(cell_index);
        match possible_value {
            None => {}
            Some(value) => {
                match value {
                    CellValue::Calculated(formula, result) => {
                        match result {
                            FormulaResult::Value(value) => {
                                self.render_value(ui, value);
                            }
                            FormulaResult::Error(message) => {
                                self.render_error(ui, message);
                            }
                        }
                    }
                    CellValue::Value(value) => {
                        self.render_value(ui, value);   
                    }
                }
            }
        }
    }
}

enum CellValue {
    Calculated(Formula, FormulaResult),
    Value(Value),
}

#[derive(Clone)]
enum Value {
    Text(String),
    Decimal(Decimal),
}

struct Formula {
    formula: String,
}

impl Formula {
    fn new(formula: String) -> Self {
        Self { formula }
    }
}

enum FormulaResult {
    Value(Value),
    Error(String),
}