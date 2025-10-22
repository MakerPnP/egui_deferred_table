use chrono::{DateTime, Local};
use egui::Ui;
use egui_deferred_table::{
    CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions,
};
use log::{debug, trace};
use std::mem;

pub mod ui;

pub enum CellState<T> {
    Loading,
    Ready(T),
    Busy(T),
}

impl<T> Default for CellState<T> {
    fn default() -> Self {
        Self::Loading
    }
}

pub enum CellValue {
    String(String), //...
}

pub struct GrowingSource<T> {
    last_accessed_at: DateTime<Local>,
    pending_operations: Vec<(DateTime<Local>, Operation)>,
    data: Vec<Vec<T>>,
}

enum Operation {
    Grow {
        row_count: usize,
        column_count: usize,
    },
    Shrink {
        row_count: usize,
        column_count: usize,
    },
}

impl<T> GrowingSource<T> {
    pub fn dimensions(&self) -> (usize, usize) {
        let rows = self.data.len();
        let columns = self.data.iter().fold(0, |acc, row| row.len().max(acc));

        (rows, columns)
    }
}

impl<V> GrowingSource<CellState<V>> {
    /// grow the source by rows/columns
    pub fn grow(&mut self, row_count: usize, column_count: usize) {
        // this implementation can't grow if there are pending operations
        if !self.pending_operations.is_empty() {
            return;
        }
        let (existing_rows, existing_columns) = self.dimensions();
        let (total_rows, total_columns) =
            (existing_rows + row_count, existing_columns + column_count);

        debug!(
            "existing_rows: {}, existing_columns: {}, total_rows: {}, total_columns: {}",
            existing_rows, existing_columns, total_rows, total_columns
        );
        for row_index in 0..total_rows {
            let is_new_row = row_index >= existing_rows;
            if is_new_row {
                let row = Vec::with_capacity(total_columns);
                self.data.push(row);
            }

            let row = &mut self.data[row_index];

            while row.len() < total_columns {
                row.push(CellState::Loading)
            }
        }

        self.pending_operations.push((
            Local::now(),
            Operation::Grow {
                row_count,
                column_count,
            },
        ));
        // here you could trigger a 'load' on another thread
    }

    /// shrink the source by rows/columns
    pub fn shrink(&mut self, row_count: usize, column_count: usize) {
        // this implementation can't shrink if there are pending operations
        if !self.pending_operations.is_empty() {
            return;
        }
        let (existing_rows, existing_columns) = self.dimensions();

        for row_index in 0..existing_rows {
            let columns_to_remove = if row_index >= existing_rows - row_count {
                existing_columns
            } else {
                column_count
            };

            let row = &mut self.data[row_index];
            for cell in row.iter_mut().skip(existing_columns - columns_to_remove) {
                let taken = mem::take(cell);
                match taken {
                    CellState::Ready(value) => *cell = CellState::Busy(value),
                    _ => *cell = taken,
                }
            }
        }

        self.pending_operations.push((
            Local::now(),
            Operation::Shrink {
                row_count,
                column_count,
            },
        ));
        // here you could trigger an operation, such as deleting, on another thread
    }
}

impl<T: Default> Default for GrowingSource<T> {
    fn default() -> Self {
        let now = Local::now();
        Self {
            last_accessed_at: now,
            pending_operations: vec![],

            data: vec![],
        }
    }
}

impl GrowingSource<CellState<CellValue>> {
    pub fn get_cell_value(&self, cell_index: CellIndex) -> Option<&CellState<CellValue>> {
        let row_values = &self.data[cell_index.row];

        let cell_value = row_values.get(cell_index.column);

        cell_value
    }

    fn simulate_background_thread_processing(&mut self, now: DateTime<Local>) {
        //
        // a background thread /could/ update the data source, we simulate this by directly processing operations here
        // don't use this approach in production though, as joining threads probably isn't immediate-mode-friendly...
        // (i.e. might take too long and cause rendering delays)
        //
        // this kind of 'operation processing' should probably orchestrated by the main thread, not the UI thread.
        //

        // Take ownership of pending_operations
        let pending_operations = std::mem::take(&mut self.pending_operations);

        // Partition into operations to process and operations to keep
        let (to_process, to_keep): (Vec<_>, Vec<_>) =
            pending_operations
                .into_iter()
                .partition(|(time, operation)| match operation {
                    Operation::Grow { .. } => {
                        now.signed_duration_since(time).num_milliseconds() > 500
                    }
                    Operation::Shrink { .. } => {
                        now.signed_duration_since(time).num_milliseconds() > 1000
                    }
                });

        // Restore operations to keep
        self.pending_operations = to_keep;

        // Process the operations
        for (_, operation) in to_process {
            match operation {
                Operation::Grow {
                    row_count,
                    column_count,
                } => {
                    self.simulate_background_loading(row_count, column_count);
                }
                Operation::Shrink {
                    row_count,
                    column_count,
                } => {
                    self.simulate_background_deletion(row_count, column_count);
                }
            }
        }
    }

    fn simulate_background_loading(&mut self, _row_count: usize, _column_count: usize) {
        // fill-in random data in all cells with `Loading` state

        let (rows, _columns) = self.dimensions();

        for row in self.data.iter_mut().take(rows) {
            for value in row.iter_mut().filter(|it| matches!(it, CellState::Loading)) {
                *value = CellState::Ready(CellValue::String("test".to_string()));
            }
        }
    }

    fn simulate_background_deletion(&mut self, row_count: usize, column_count: usize) {
        let (rows, columns) = self.dimensions();

        if rows >= row_count {
            // this dummy implementation ignores the 'busy' state and just truncates rows/columns regardless
            self.data.truncate(rows - row_count);
        }
        for row in self.data.iter_mut() {
            if columns >= column_count {
                row.truncate(columns - column_count);
            }
        }
    }
}

impl DeferredTableDataSource for GrowingSource<CellState<CellValue>> {
    fn prepare(&mut self) {
        let now = Local::now();
        self.last_accessed_at = now;

        self.simulate_background_thread_processing(now);
    }

    fn finalize(&mut self) {
        trace!("finalize called");
    }

    fn get_dimensions(&self) -> TableDimensions {
        let (rows, columns) = self.dimensions();

        TableDimensions {
            row_count: rows,
            column_count: columns,
        }
    }
}

#[derive(Default)]
pub struct GrowingSourceRenderer {}

impl DeferredTableRenderer<GrowingSource<CellState<CellValue>>> for GrowingSourceRenderer {
    fn render_cell(
        &self,
        ui: &mut Ui,
        cell_index: CellIndex,
        data_source: &GrowingSource<CellState<CellValue>>,
    ) {
        let Some(cell_state) = data_source.get_cell_value(cell_index) else {
            return;
        };

        match cell_state {
            CellState::Loading => {
                ui.spinner();
            }
            CellState::Ready(value) => match value {
                CellValue::String(s) => {
                    ui.label(s);
                }
            },
            CellState::Busy(value) => match value {
                CellValue::String(s) => {
                    ui.horizontal(|ui| {
                        ui.label(s);
                        ui.spinner();
                    });
                }
            },
        }
    }
}

#[derive(Default)]
pub struct GrowingSourceAlternativeRenderer {}

impl DeferredTableRenderer<GrowingSource<CellState<CellValue>>>
    for GrowingSourceAlternativeRenderer
{
    fn render_cell(
        &self,
        ui: &mut Ui,
        cell_index: CellIndex,
        data_source: &GrowingSource<CellState<CellValue>>,
    ) {
        let Some(cell_state) = data_source.get_cell_value(cell_index) else {
            return;
        };

        match cell_state {
            CellState::Loading => {
                ui.label("loading...");
            }
            CellState::Ready(value) => Self::render_value(ui, value),
            CellState::Busy(value) => {
                ui.horizontal(|ui| {
                    ui.label("*");
                    Self::render_value(ui, value);
                });
            }
        }
    }
}

impl GrowingSourceAlternativeRenderer {
    fn render_value(ui: &mut Ui, value: &CellValue) {
        match value {
            CellValue::String(s) => {
                ui.monospace(s);
            }
        }
    }
}
