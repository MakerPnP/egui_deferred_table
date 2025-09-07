use crate::spreadsheet::value::Value;

pub struct Formula {
    formula: String,
}

impl Formula {
    pub fn new(formula: String) -> Self {
        Self { formula }
    }
}

pub enum FormulaResult {
    Pending,
    Value(Value),
    Error(String),
}
