use chrono::{DateTime, Local};
use egui::Ui;
use egui_deferred_table::{
    CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions,
};
use log::{debug, trace};

pub mod ui;

enum CellState<T> {
    Loading,
    Ready(T),
}

impl<T> Default for CellState<T> {
    fn default() -> Self {
        Self::Loading
    }
}

enum CellValue {
    String(String), //...
}

struct GrowingSource<T> {
    last_accessed_at: DateTime<Local>,
    pending_operations: Vec<(DateTime<Local>, Operations)>,
    data: Vec<Vec<T>>,
}

enum Operations {
    Grow,
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

        self.pending_operations
            .push((Local::now(), Operations::Grow));
        // here you could trigger a 'load' on another thread
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
                    Operations::Grow => now.signed_duration_since(time).num_milliseconds() > 500,
                });

        // Restore operations to keep
        self.pending_operations = to_keep;

        // Process the operations
        for (_, operation) in to_process {
            match operation {
                Operations::Grow => {
                    self.simulate_background_loading();
                }
            }
        }
    }

    fn simulate_background_loading(&mut self) {
        // fill-in random data in all cells with `Loading` state

        let (rows, _columns) = self.dimensions();

        for row in self.data.iter_mut().take(rows) {
            for value in row.iter_mut().filter(|it| matches!(it, CellState::Loading)) {
                *value = CellState::Ready(CellValue::String("test".to_string()));
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
struct GrowingSourceRenderer {}

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
        }
    }
}
