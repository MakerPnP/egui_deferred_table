use rust_decimal::Decimal;
use crate::spreadsheet::formula::{Formula, FormulaResult};

#[derive(Debug)]
pub enum CellValue {
    Calculated(Formula, FormulaResult),
    Value(Value),
}

impl CellValue {
    pub fn to_editable(&self) -> String {
        match self {
            CellValue::Calculated(formula, result) => formula.formula.clone(),
            CellValue::Value(value) => {
                match value {
                    Value::Text(text) => text.clone(),
                    Value::Decimal(decimal) => decimal.to_string(),
                    Value::Empty => "".to_string(),
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Text(String),
    Decimal(Decimal),
}
