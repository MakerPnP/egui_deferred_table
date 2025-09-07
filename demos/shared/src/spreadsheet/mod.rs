use egui::Ui;
use egui_deferred_table::{CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions};
use rust_decimal_macros::dec;
use crate::spreadsheet::formula::{Formula, FormulaResult};
use crate::spreadsheet::value::{CellValue, Value};

pub mod ui;
pub mod value;
pub mod formula;

pub struct SpreadsheetSource {
    data: Vec<Vec<CellValue>>,
}

impl SpreadsheetSource {
    pub fn new() -> Self {
        let data = vec![
            vec![
                CellValue::Value(Value::Text("Message".to_string())),
                CellValue::Value(Value::Text("Value 1".to_string())),
                CellValue::Value(Value::Text("Value 2".to_string())),
                CellValue::Value(Value::Text("Result".to_string())),
            ],
            vec![
                CellValue::Value(Value::Text("Hello World".to_string())),
                CellValue::Value(Value::Decimal(dec!(42.0))),
                CellValue::Value(Value::Decimal(dec!(69.0))),
                CellValue::Calculated(Formula::new("=B2+C2".to_string()), FormulaResult::Pending)
            ],
            vec![
                CellValue::Value(Value::Text("Example data".to_string())),
                CellValue::Value(Value::Decimal(dec!(6.0))),
                CellValue::Value(Value::Decimal(dec!(9.0))),
                CellValue::Calculated(Formula::new("=B3+C3".to_string()), FormulaResult::Pending)
            ],

        ];

        Self {
            data,
        }
    }

    fn calculate_value(&self, _formula: &Formula) -> FormulaResult {
        FormulaResult::Error("#NOT_IMPLEMENTED".to_string())
    }

    pub fn render_spinner(&self, ui: &mut Ui) {
        ui.spinner();
    }

    pub fn render_error(&self, ui: &mut Ui, message: &String) {
        ui.colored_label(egui::Color32::RED, message);
    }

    pub fn render_value(&self, ui: &mut Ui, value: &Value) {
        match value {
            Value::Text(text) => {
                ui.label(text);
            }
            Value::Decimal(decimal) => {
                ui.label(decimal.to_string());
            }
        }
    }

    pub fn get_cell_value(&self, cell_index: CellIndex) -> Option<&CellValue> {
        let row_values = &self.data[cell_index.row];

        let cell_value = row_values.get(cell_index.column);

        cell_value
    }

    // given '0' the result is 'A', '25' is 'Z', given '26' the result is 'AA', given '27' the result is 'AB' and so on.
    pub fn make_column_name(index: usize) -> String {
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

    pub fn move_column(&mut self, from: usize, to: usize) {
        for row in self.data.iter_mut() {
            let value = row.remove(from);
            row.insert(to, value);
        }

        // FUTURE update formulas

        self.recalculate();
    }

    pub fn move_row(&mut self, from: usize, to: usize) {
        let row = self.data.remove(from);
        self.data.insert(to, row);

        // FUTURE update formulas

        self.recalculate();
    }

    pub fn recalculate(&mut self) {
        // TODO
    }
}

impl DeferredTableDataSource for SpreadsheetSource {
    fn get_dimensions(&self) -> TableDimensions {
        let rows = self.data.len();
        let columns = self.data.iter().fold(0, |acc, row| {
            row.len().max(acc)
        });

        TableDimensions {
            row_count: rows,
            column_count: columns
        }
    }
}

impl DeferredTableRenderer for SpreadsheetSource {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex) {
        let possible_value = self.get_cell_value(cell_index);
        match possible_value {
            None => {}
            Some(value) => {
                match value {
                    CellValue::Calculated(formula, result) => {
                        match result {
                            FormulaResult::Pending => {
                                self.render_spinner(ui);
                            }
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
