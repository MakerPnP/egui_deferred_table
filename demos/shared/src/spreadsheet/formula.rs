use crate::spreadsheet::value::Value;

#[derive(Debug)]
pub struct Formula {
    pub(crate) formula: String,
}

impl Formula {
    pub fn new(formula: String) -> Self {
        Self { formula }
    }
}

#[derive(Debug)]
pub enum FormulaResult {
    Pending,
    Value(Value),
    Error(String),
}
