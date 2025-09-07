use crate::spreadsheet::value::Value;

pub struct Formula {
    formula: String,
}

impl Formula {
    fn new(formula: String) -> Self {
        Self { formula }
    }
}

pub enum FormulaResult {
    Value(Value),
    Error(String),
}
