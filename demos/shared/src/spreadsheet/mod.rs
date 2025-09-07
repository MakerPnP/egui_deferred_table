///
///
use egui::Ui;
use log::{debug, trace};
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
                CellValue::Calculated(Formula::new("=B2+C2".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Example data".to_string())),
                CellValue::Value(Value::Decimal(dec!(6.0))),
                CellValue::Value(Value::Decimal(dec!(9.0))),
                CellValue::Calculated(Formula::new("=B3+C3".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Total".to_string())),
                CellValue::Calculated(Formula::new("=B2+B3".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=C2+C3".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=(B4+C4)+(D2+D3)".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Factor".to_string())),
                CellValue::Calculated(Formula::new("=5+(10/2)".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=B5*0.5".to_string()), FormulaResult::Pending),
                CellValue::Value(Value::Empty),
            ],
            vec![
                CellValue::Calculated(Formula::new("=1**".to_string()), FormulaResult::Pending),
                //CellValue::Value(Value::Empty),
                CellValue::Value(Value::Empty),
                CellValue::Value(Value::Text("Final Result".to_string())),
                CellValue::Calculated(Formula::new("=C5+(B4*C4)*(D2*D3)/D4".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Circular Refs".to_string())),
                CellValue::Calculated(Formula::new("=B7".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=B7*2".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=(C7*2)+D6".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Errors 1".to_string())),
                CellValue::Calculated(Formula::new("X".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=X".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=A1*2".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Errors 2".to_string())),
                CellValue::Calculated(Formula::new("=(A1".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=1*£".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=1/0".to_string()), FormulaResult::Pending),
            ],
            vec![
                CellValue::Value(Value::Text("Errors 3".to_string())),
                CellValue::Calculated(Formula::new("=1£".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=".to_string()), FormulaResult::Pending),
                CellValue::Calculated(Formula::new("=1%0".to_string()), FormulaResult::Pending),
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
            Value::Empty => {}
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
    pub fn update_formulas_for_column_move(&mut self, from: usize, to: usize) {
        // Create a mapping of old column indices to new column indices
        let mut column_mapping = std::collections::HashMap::new();

        // Build the mapping based on the move operation
        if from < to {
            // Moving column to the right
            for col in 0..self.data[0].len() {
                if col == from {
                    column_mapping.insert(col, to);
                } else if col > from && col <= to {
                    column_mapping.insert(col, col - 1);
                } else {
                    column_mapping.insert(col, col);
                }
            }
        } else {
            // Moving column to the left
            for col in 0..self.data[0].len() {
                if col == from {
                    column_mapping.insert(col, to);
                } else if col >= to && col < from {
                    column_mapping.insert(col, col + 1);
                } else {
                    column_mapping.insert(col, col);
                }
            }
        }

        // Debug the mapping
        for (old_col, new_col) in &column_mapping {
            trace!("Column mapping: {} -> {}", old_col, new_col);
        }

        // For each cell in the spreadsheet that has a formula
        for row_idx in 0..self.data.len() {
            for col_idx in 0..self.data[row_idx].len() {
                if let CellValue::Calculated(formula, _) = &mut self.data[row_idx][col_idx] {
                    let old_formula = formula.formula.clone();

                    // Extract cell references from the formula
                    let dependencies = Self::extract_dependencies(&old_formula);

                    // Create a list of replacements to make
                    let mut replacements = Vec::new();

                    // Identify all the replacements needed
                    for dep in dependencies {
                        if let Some((dep_row, dep_col)) = Self::parse_cell_reference(&dep) {
                            if let Some(&new_col) = column_mapping.get(&dep_col) {
                                if new_col != dep_col {
                                    replacements.push((
                                        dep.clone(),
                                        format!("{}{}", Self::make_column_name(new_col), dep_row + 1)
                                    ));
                                }
                            }
                        }
                    }

                    // Sort replacements by length in descending order to avoid partial matches
                    replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

                    // Apply all replacements to create the new formula
                    let mut new_formula = old_formula.clone();

                    // Use a temporary formula with markers to avoid replacing parts of already replaced references
                    let mut temp_formula = new_formula.clone();
                    for (i, (old_ref, new_ref)) in replacements.iter().enumerate() {
                        // Use a unique marker for each replacement
                        let marker = format!("__REF_MARKER_{}_", i);

                        // Replace the old reference with the marker
                        temp_formula = temp_formula.replace(old_ref, &marker);

                        // Track the replacement for logging
                        debug!("Will replace {} with {}", old_ref, new_ref);
                    }

                    // Now apply the actual replacements
                    new_formula = temp_formula.clone();
                    for (i, (_, new_ref)) in replacements.iter().enumerate() {
                        let marker = format!("__REF_MARKER_{}_", i);
                        new_formula = new_formula.replace(&marker, new_ref);
                    }

                    // Update the formula if it changed
                    if new_formula != old_formula {
                        debug!("old formula: \"{}\", new formula: \"{}\"", old_formula, new_formula);
                        formula.formula = new_formula;
                    }
                }
            }
        }
    }

    pub fn update_formulas_for_row_move(&mut self, from: usize, to: usize) {
        // Create a mapping of old row indices to new row indices
        let mut row_mapping = std::collections::HashMap::new();

        // Build the mapping based on the move operation
        if from < to {
            // Moving row down
            for row in 0..self.data.len() {
                if row == from {
                    row_mapping.insert(row, to);
                } else if row > from && row <= to {
                    row_mapping.insert(row, row - 1);
                } else {
                    row_mapping.insert(row, row);
                }
            }
        } else {
            // Moving row up
            for row in 0..self.data.len() {
                if row == from {
                    row_mapping.insert(row, to);
                } else if row >= to && row < from {
                    row_mapping.insert(row, row + 1);
                } else {
                    row_mapping.insert(row, row);
                }
            }
        }

        // Debug the mapping
        for (old_row, new_row) in &row_mapping {
            trace!("Row mapping: {} -> {}", old_row, new_row);
        }

        // For each cell in the spreadsheet that has a formula
        for row_idx in 0..self.data.len() {
            for col_idx in 0..self.data[row_idx].len() {
                if let CellValue::Calculated(formula, _) = &mut self.data[row_idx][col_idx] {
                    let old_formula = formula.formula.clone();

                    // Extract cell references from the formula
                    let dependencies = Self::extract_dependencies(&old_formula);

                    // Create a list of replacements to make
                    let mut replacements = Vec::new();

                    // Identify all the replacements needed
                    for dep in dependencies {
                        if let Some((dep_row, dep_col)) = Self::parse_cell_reference(&dep) {
                            if let Some(&new_row) = row_mapping.get(&dep_row) {
                                if new_row != dep_row {
                                    replacements.push((
                                        dep.clone(),
                                        format!("{}{}", Self::make_column_name(dep_col), new_row + 1)
                                    ));
                                }
                            }
                        }
                    }

                    // Sort replacements by length in descending order to avoid partial matches
                    replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

                    // Apply all replacements to create the new formula
                    let mut new_formula = old_formula.clone();

                    // Use a temporary formula with markers to avoid replacing parts of already replaced references
                    let mut temp_formula = new_formula.clone();
                    for (i, (old_ref, new_ref)) in replacements.iter().enumerate() {
                        // Use a unique marker for each replacement
                        let marker = format!("__REF_MARKER_{}_", i);

                        // Replace the old reference with the marker
                        temp_formula = temp_formula.replace(old_ref, &marker);

                        // Track the replacement for logging
                        debug!("Will replace {} with {}", old_ref, new_ref);
                    }

                    // Now apply the actual replacements
                    new_formula = temp_formula.clone();
                    for (i, (_, new_ref)) in replacements.iter().enumerate() {
                        let marker = format!("__REF_MARKER_{}_", i);
                        new_formula = new_formula.replace(&marker, new_ref);
                    }

                    // Update the formula if it changed
                    if new_formula != old_formula {
                        debug!("old formula: \"{}\", new formula: \"{}\"", old_formula, new_formula);
                        formula.formula = new_formula;
                    }
                }
            }
        }
    }
    pub fn move_column(&mut self, from: usize, to: usize) {
        // Skip if from and to are the same
        if from == to {
            return;
        }

        // First update all formulas to account for the move
        self.update_formulas_for_column_move(from, to);

        // Then perform the actual move
        for row in self.data.iter_mut() {
            let value = row.remove(from);
            row.insert(to, value);
        }

        self.recalculate();
    }

    pub fn move_row(&mut self, from: usize, to: usize) {
        // Skip if from and to are the same
        if from == to {
            return;
        }

        // First update all formulas to account for the move
        self.update_formulas_for_row_move(from, to);

        // Then perform the actual move
        let row = self.data.remove(from);
        self.data.insert(to, row);

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

        // Debug: print dependencies
        trace!("Dependencies:");
        for (cell, deps) in &dependencies {
            trace!("{} depends on: {:?}", cell, deps);
        }


        // Step 2: Create a reversed dependency graph (dependency -> dependents)
        let mut reversed_deps: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut missing_deps = std::collections::HashSet::new();

        // First pass: build the reversed deps and collect missing dependencies
        for (cell, deps) in &dependencies {
            for dep in deps {
                reversed_deps.entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(cell.clone());

                // Collect dependencies that don't have an entry yet
                if !dependencies.contains_key(dep) {
                    missing_deps.insert(dep.clone());
                }
            }
        }

        // Second pass: add missing dependencies
        for dep in missing_deps {
            dependencies.insert(dep, Vec::new());
        }

        // Debug: print reversed dependencies
        trace!("Reversed Dependencies:");
        for (cell, deps) in &reversed_deps {
            trace!("{} is used by: {:?}", cell, deps);
        }
        // Step 3: Perform topological sort to determine calculation order
        let mut calculation_order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        // Perform topological sort on all nodes
        let mut has_cycles = false;
        for cell in dependencies.keys().cloned().collect::<Vec<_>>() {
            if !visited.contains(&cell) {
                if Self::has_cycle(
                    &cell,
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

        if has_cycles {
            trace!("WARNING: Cycles detected in formula dependencies!");
        }

        trace!("Calculation order: {:?}", calculation_order);

        // Map of cell name to its calculated value
        let mut calculated_values = std::collections::HashMap::new();

        // Pre-populate with all non-formula cells
        for (row_idx, row) in self.data.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if let CellValue::Value(value) = cell {
                    let cell_name = format!("{}{}", Self::make_column_name(col_idx), row_idx + 1);
                    calculated_values.insert(cell_name, value.clone());
                }
            }
        }

        trace!("Initial non-formula values:");
        for (cell, value) in &calculated_values {
            trace!("{}: {:?}", cell, value);
        }

        for cell_name in calculation_order {
            // Find row and column from cell name
            if let Some((row, col)) = Self::parse_cell_reference(&cell_name) {
                if row < self.data.len() && col < self.data[row].len() {
                    if let CellValue::Calculated(formula, _) = &self.data[row][col] {
                        // Evaluate formula with the current set of calculated values
                        let result = self.evaluate_formula(formula, &calculated_values);
                        debug!("Cell: {}: Result: {:?}", cell_name, result);

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

    /// Evaluates a formula expression with support for:
    /// - Basic operations: +, -, *, /, %
    /// - Cell references (e.g., A1, B2)
    /// - Parentheses for sub-expressions
    /// - Chained operations (e.g., A1+B1*C1)
    ///
    /// AI prompt:
    /// ```text
    /// add support for other operators (*, /, -, +, %), and chained operators and brackets `( )`
    ///
    /// example formulas:
    ///
    /// =A1
    /// =A1*2
    /// =A1+B1
    /// =A1+(B1/2)
    /// =A1+(B1*(C1*D1))*5
    ///
    /// operator precedence is left to right, processing sub expressions (in brackets) as you go.
    /// ```
    fn evaluate_formula(
        &self,
        formula: &Formula,
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> FormulaResult {
        trace!("Evaluating formula: {}", formula.formula);

        let formula_text = &formula.formula;
        if !formula_text.starts_with('=') {
            return FormulaResult::Error("#INVALID_FORMULA".to_string());
        }

        let expression = &formula_text[1..]; // Remove the '=' prefix

        // Parse and evaluate the expression
        self.evaluate_expression(expression, calculated_values)
    }

    /// Parses and evaluates a formula expression
    fn evaluate_expression(
        &self,
        expression: &str,
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> FormulaResult {
        let expression = expression.trim();

        // Handle empty expression
        if expression.is_empty() {
            return FormulaResult::Error("#EMPTY_EXPRESSION".to_string());
        }

        // Tokenize the expression
        let tokens = self.tokenize_expression(expression);

        // Parse and evaluate tokens
        self.evaluate_tokens(&tokens, calculated_values)
    }

    /// Tokenizes an expression into operands and operators
    fn tokenize_expression(&self, expression: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_cell_ref = false;
        let mut paren_level = 0;

        for c in expression.chars() {
            match c {
                '(' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    paren_level += 1;
                    tokens.push("(".to_string());
                },
                ')' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    paren_level -= 1;
                    if paren_level < 0 {
                        // Unmatched parenthesis - this will be caught during evaluation
                        tokens.push(")".to_string());
                        paren_level = 0;
                    } else {
                        tokens.push(")".to_string());
                    }
                },
                '+' | '-' | '*' | '/' | '%' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    tokens.push(c.to_string());
                    in_cell_ref = false;
                },
                ' ' | '\t' | '\n' => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    in_cell_ref = false;
                },
                _ => {
                    // Start of a cell reference or number
                    if !in_cell_ref && c.is_ascii_alphabetic() {
                        in_cell_ref = true;
                    }
                    current_token.push(c);
                }
            }
        }

        // Don't forget the last token
        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }

    /// Evaluates a sequence of tokens
    fn evaluate_tokens(
        &self,
        tokens: &[String],
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> FormulaResult {
        if tokens.is_empty() {
            return FormulaResult::Error("#EMPTY_EXPRESSION".to_string());
        }

        // Find matching parentheses and evaluate sub-expressions
        let mut processed_tokens = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            if tokens[i] == "(" {
                // Find the matching closing parenthesis
                let mut paren_level = 1;
                let mut j = i + 1;

                while j < tokens.len() && paren_level > 0 {
                    if tokens[j] == "(" {
                        paren_level += 1;
                    } else if tokens[j] == ")" {
                        paren_level -= 1;
                    }
                    j += 1;
                }

                if paren_level != 0 {
                    return FormulaResult::Error("#UNMATCHED_PARENTHESIS".to_string());
                }

                // Extract the sub-expression within the parentheses
                let sub_expr_tokens = &tokens[(i+1)..(j-1)];

                // Evaluate the sub-expression
                let sub_result = self.evaluate_tokens(sub_expr_tokens, calculated_values);

                match sub_result {
                    FormulaResult::Value(value) => {
                        processed_tokens.push(match value {
                            Value::Decimal(d) => d.to_string(),
                            Value::Text(t) => t,
                            Value::Empty => "".to_string()
                        });
                    },
                    _ => return sub_result,
                }

                i = j;
            } else {
                processed_tokens.push(tokens[i].clone());
                i += 1;
            }
        }

        // Process the expression without parentheses
        self.evaluate_simple_expression(&processed_tokens, calculated_values)
    }

    /// Evaluates a simple expression with no parentheses, respecting standard mathematical operator precedence:
    /// 1. Multiplication, division, modulo (left to right)
    /// 2. Addition, subtraction (left to right)
    fn evaluate_simple_expression(
        &self,
        tokens: &[String],
        calculated_values: &std::collections::HashMap<String, Value>
    ) -> FormulaResult {
        if tokens.is_empty() {
            return FormulaResult::Error("#EMPTY_EXPRESSION".to_string());
        }

        // If there's only one token, it must be a cell reference or a literal
        if tokens.len() == 1 {
            let token = &tokens[0];

            // Check if it's a cell reference
            if token.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
                return self.get_cell_value_by_ref(token, calculated_values)
                    .map_or(FormulaResult::Error("#REF".to_string()),
                            |v| FormulaResult::Value(v));
            }

            // Check if it's a number
            if let Ok(num) = token.parse::<rust_decimal::Decimal>() {
                return FormulaResult::Value(Value::Decimal(num));
            }

            return FormulaResult::Error("#INVALID_TOKEN".to_string());
        }

        trace!("tokens: {:?}", tokens);

        // Step 1: Parse tokens into values and operators
        let mut values = Vec::new();
        let mut operators = Vec::new();


        // Track if we're expecting an operand or operator
        let mut expect_operand = true;

        for token in tokens {
            if expect_operand {
                // Parse operand (cell reference or number)
                let value = if token.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
                    // It's a cell reference
                    match self.get_cell_value_by_ref(token, calculated_values) {
                        Some(Value::Decimal(d)) => d,
                        _ => return FormulaResult::Error(format!("#REF_OR_TYPE_MISMATCH: {}", token)),
                    }
                } else {
                    // It's a number
                    match token.parse::<rust_decimal::Decimal>() {
                        Ok(num) => num,
                        Err(_) => return FormulaResult::Error(format!("#INVALID_NUMBER: {}", token)),
                    }
                };

                values.push(value);
                expect_operand = false;
            } else {
                // Parse operator
                match token.as_str() {
                    "+" | "-" | "*" | "/" | "%" => {
                        operators.push(token.clone());
                        expect_operand = true;
                    },
                    _ => return FormulaResult::Error(format!("#EXPECTED_OPERATOR_GOT: {}", token)),
                }
            }
        }

        // Validate we have the correct number of values and operators
        if values.len() != operators.len() + 1 {
            return FormulaResult::Error("#INVALID_EXPRESSION_STRUCTURE".to_string());
        }

        // Step 2: First process all higher precedence operators (* / %)
        let mut i = 0;
        while i < operators.len() {
            if operators[i] == "*" || operators[i] == "/" || operators[i] == "%" {
                let result = match operators[i].as_str() {
                    "*" => values[i] * values[i + 1],
                    "/" => {
                        if values[i + 1].is_zero() {
                            return FormulaResult::Error("#DIV_BY_ZERO".to_string());
                        }
                        values[i] / values[i + 1]
                    },
                    "%" => {
                        if values[i + 1].is_zero() {
                            return FormulaResult::Error("#DIV_BY_ZERO".to_string());
                        }
                        values[i] % values[i + 1]
                    },
                    _ => unreachable!(), // We've already filtered for valid operators
                };

                // Replace the first value with the result and remove the second value and the operator
                values[i] = result;
                values.remove(i + 1);
                operators.remove(i);

                trace!("After processing * / %: values={:?}, operators={:?}", values, operators);
            } else {
                i += 1; // Move to next operator
            }
        }

        // Step 3: Process lower precedence operators (+ -)
        while !operators.is_empty() {
            let result = match operators[0].as_str() {
                "+" => values[0] + values[1],
                "-" => values[0] - values[1],
                _ => unreachable!(), // Only + and - should remain
            };

            // Replace the first value with the result and remove the second value and the operator
            values[0] = result;
            values.remove(1);
            operators.remove(0);

            trace!("After processing + -: values={:?}, operators={:?}", values, operators);
        }

        // The final result should be the only value left
        assert!(values.len() == 1, "Expected exactly one value to remain");

        FormulaResult::Value(Value::Decimal(values[0]))
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
