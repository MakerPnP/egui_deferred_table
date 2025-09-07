use rust_decimal::Decimal;
use crate::spreadsheet::formula::{Formula, FormulaResult};

pub enum CellValue {
    Calculated(Formula, FormulaResult),
    Value(Value),
}

#[derive(Clone)]
pub enum Value {
    Text(String),
    Decimal(Decimal),
}
