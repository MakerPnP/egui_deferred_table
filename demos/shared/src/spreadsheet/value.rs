use rust_decimal::Decimal;
use crate::spreadsheet::formula::{Formula, FormulaResult};

#[derive(Debug)]
pub enum CellValue {
    Calculated(Formula, FormulaResult),
    Value(Value),
}

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    Text(String),
    Decimal(Decimal),
}
