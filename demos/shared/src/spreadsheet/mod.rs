///
///
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


    /// AI prompt (Clause 3.7 Sonnet):
    /// ```text
    /// we're making a spreadsheet calculation function
    ///
    /// spreadsheets contain formulas, e.g. =B1, or =B1+C1
    ///
    /// however, when calculating A1's formula, which is =B1+C1, if B1 contains a formula, eg. =C2*2, then B1's formula needs to be evaluated first, and so on.
    ///
    /// so, first we need to create a calculation order for each cell with a formula, i.e. a set of dependencies.
    ///
    /// e.g. [A1 => [C1,B1], B1 => [C1]]
    ///
    /// then, we need to make a unique set of cells that need calculating so that we don't recalculate any cell twice.
    ///
    /// e.g. A1,B1,C1
    ///
    /// then we need to somehow order this set of cells that need calculating so that when we process each cell, it's dependencies have already been calculated.
    ///
    /// in this example, the order would be C1, B1, A1.
    ///
    /// if there any cells with dependencies that cannot be met, we need to record this. e.g. if cell A1 had a formula =A1 that would be a self-reference. which can never be evalulated since it depends on itself.
    /// ```
    pub fn recalculate(&mut self) {
        // Step 1: Build dependency graph
        let mut dependencies: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut cells_with_formulas: Vec<(usize, usize, &Formula)> = Vec::new();

        // Collect all cells with formulas and build initial dependency map
        for (row_idx, row) in self.data.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if let CellValue::Calculated(formula, _) = cell {
                    let cell_name = format!("{}{}", Self::make_column_name(col_idx), row_idx + 1);
                    cells_with_formulas.push((row_idx, col_idx, formula));
                    dependencies.insert(cell_name, vec![]);
                }
            }
        }

        // Parse formulas to determine dependencies
        for (row_idx, col_idx, formula) in &cells_with_formulas {
            let cell_name = format!("{}{}", Self::make_column_name(*col_idx), row_idx + 1);

            // Extract referenced cells from formula
            // This is a simplified parser - in a real implementation, you'd need a proper formula parser
            let formula_deps = Self::extract_dependencies(&formula.formula);

            if let Some(deps) = dependencies.get_mut(&cell_name) {
                deps.extend(formula_deps);
            }
        }

        // Step 2: Detect circular dependencies and create calculation order
        let mut calculation_order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut has_cycles = false;

        for cell in dependencies.keys() {
            if !visited.contains(cell) {
                if Self::has_cycle(
                    cell,
                    &dependencies,
                    &mut visited,
                    &mut temp_visited,
                    &mut calculation_order
                ) {
                    has_cycles = true;
                    // Mark cells in cycles with errors
                    self.mark_cycle_errors(&temp_visited);
                }
            }
        }

        // If we have cycles, we can't proceed with calculation in a reliable way
        if has_cycles {
            return;
        }

        // Step 3: Calculate cells in topological order
        calculation_order.reverse(); // Reverse to get correct order (leaf nodes first)

        // Map of cell name to its calculated value
        let mut calculated_values = std::collections::HashMap::new();

        for cell_name in calculation_order {
            // Find row and column from cell name
            if let Some((row, col)) = Self::parse_cell_reference(&cell_name) {
                if row < self.data.len() && col < self.data[row].len() {
                    if let CellValue::Calculated(formula, _) = &self.data[row][col] {
                        // Evaluate formula with the current set of calculated values
                        let result = self.evaluate_formula(formula, &calculated_values);

                        // Store the calculated value
                        if let FormulaResult::Value(value) = &result {
                            calculated_values.insert(cell_name.clone(), value.clone());
                        }

                        // Update the cell with the result
                        if let CellValue::Calculated(formula, old_result) = &mut self.data[row][col] {
                            *old_result = result;
                        }
                    }
                }
            }
        }
    }

    fn extract_dependencies(formula: &str) -> Vec<String> {
        let mut dependencies = Vec::new();
        let formula = formula.trim();

        // Skip the '=' at the beginning
        if !formula.starts_with('=') {
            return dependencies;
        }

        // Simple regex-like parser for cell references (like A1, B2, etc.)
        // In a real implementation, you would use a proper formula parser
        let chars: Vec<char> = formula[1..].chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // If we find a letter, it could be the start of a cell reference
            if chars[i].is_ascii_alphabetic() {
                let mut col = String::new();
                let mut row = String::new();

                // Parse column letters (A, B, AA, etc.)
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    col.push(chars[i]);
                    i += 1;
                }

                // Parse row numbers
                while i < chars.len() && chars[i].is_ascii_digit() {
                    row.push(chars[i]);
                    i += 1;
                }

                // If we have both a column and row, it's a valid cell reference
                if !col.is_empty() && !row.is_empty() {
                    dependencies.push(format!("{}{}", col, row));
                }
            } else {
                i += 1;
            }
        }

        dependencies
    }

    fn has_cycle(
        node: &str,
        graph: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<String>
    ) -> bool {
        if temp_visited.contains(node) {
            return true; // Cycle detected
        }

        if visited.contains(node) {
            return false; // Already processed, no cycle through this node
        }

        temp_visited.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if Self::has_cycle(neighbor, graph, visited, temp_visited, result) {
                    return true;
                }
            }
        }

        // Remove from temporary set after processing
        temp_visited.remove(node);
        // Mark as visited and add to result
        visited.insert(node.to_string());
        result.push(node.to_string());

        false
    }

    fn mark_cycle_errors(&mut self, cycle_cells: &std::collections::HashSet<String>) {
        for cell_name in cycle_cells {
            if let Some((row, col)) = Self::parse_cell_reference(cell_name) {
                if row < self.data.len() && col < self.data[row].len() {
                    if let CellValue::Calculated(_, result) = &mut self.data[row][col] {
                        *result = FormulaResult::Error("#CIRCULAR_REF".to_string());
                    }
                }
            }
        }
    }

    fn parse_cell_reference(cell_ref: &str) -> Option<(usize, usize)> {
        let mut col_str = String::new();
        let mut row_str = String::new();

        for c in cell_ref.chars() {
            if c.is_ascii_alphabetic() {
                col_str.push(c);
            } else if c.is_ascii_digit() {
                row_str.push(c);
            }
        }

        let row = row_str.parse::<usize>().ok()?.checked_sub(1)?; // 1-indexed to 0-indexed

        // Convert column letters to 0-indexed number (A=0, B=1, etc.)
        let mut col = 0;
        for c in col_str.chars() {
            col = col * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1);
        }
        col = col.checked_sub(1)?; // Convert to 0-indexed

        Some((row, col))
    }

    /// This is a simplified implementation
    /// In a real spreadsheet, you'd have a proper formula evaluator.
    ///
    /// the only formulas currently supported are:
    /// 1. simple additions, e.g. =B1+B2
    /// 2. cell reference, e.g. =C2
    fn evaluate_formula(
        &self,
        formula: &Formula,
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> FormulaResult {

        // For now, just parse basic operations like addition between cells
        let formula_text = &formula.formula;
        if !formula_text.starts_with('=') {
            return FormulaResult::Error("#INVALID_FORMULA".to_string());
        }

        let expression = &formula_text[1..]; // Remove the '=' prefix

        // Check for simple addition (e.g., "=A1+B1")
        if let Some(pos) = expression.find('+') {
            let left = &expression[..pos].trim();
            let right = &expression[pos+1..].trim();

            let left_value = self.get_cell_value_by_ref(left, calculated_values);
            let right_value = self.get_cell_value_by_ref(right, calculated_values);

            match (left_value, right_value) {
                (Some(Value::Decimal(d1)), Some(Value::Decimal(d2))) => {
                    FormulaResult::Value(Value::Decimal(d1 + d2))
                },
                (Some(Value::Text(t1)), Some(Value::Text(t2))) => {
                    FormulaResult::Value(Value::Text(format!("{}{}", t1, t2)))
                },
                _ => FormulaResult::Error("#TYPE_MISMATCH".to_string()),
            }
        }
        // Check for cell reference (e.g., "=A1")
        else if expression.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            self.get_cell_value_by_ref(expression, calculated_values)
                .map_or(FormulaResult::Error("#REF".to_string()), |v| FormulaResult::Value(v))
        }
        else {
            FormulaResult::Error("#SYNTAX_ERROR".to_string())
        }
    }

    fn get_cell_value_by_ref(
        &self,
        cell_ref: &str,
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> Option<Value> {
        // If the value is already calculated, return it
        if let Some(value) = calculated_values.get(cell_ref) {
            return Some(value.clone());
        }

        // Otherwise try to get it from the spreadsheet
        if let Some((row, col)) = Self::parse_cell_reference(cell_ref) {
            if row < self.data.len() && col < self.data[row].len() {
                match &self.data[row][col] {
                    CellValue::Value(val) => Some(val.clone()),
                    CellValue::Calculated(_, FormulaResult::Value(val)) => Some(val.clone()),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
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
