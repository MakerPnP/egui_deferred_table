//! A table of numbered rows, shown in a harness with no window, for tests of the table's behaviour.

use egui::{Align, Id, Ui};
use egui_deferred_table::{
    Action, CellIndex, DeferredTable, DeferredTableDataSource, DeferredTableRenderer,
    TableDimensions,
};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

pub const ROW_COUNT: usize = 100;

pub struct Rows {
    pub row_count: usize,
}

impl DeferredTableDataSource for Rows {
    fn get_dimensions(&self) -> TableDimensions {
        TableDimensions {
            row_count: self.row_count,
            column_count: 2,
        }
    }
}

#[derive(Default)]
pub struct RowsRenderer {
    pub row_ordering: Option<Vec<usize>>,
}

impl DeferredTableRenderer<Rows> for RowsRenderer {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, _source: &Rows) {
        ui.label(cell_label(cell_index.row, cell_index.column));
    }

    fn row_ordering(&self) -> Option<&[usize]> {
        self.row_ordering.as_deref()
    }
}

pub fn cell_label(row: usize, column: usize) -> String {
    format!("row {} column {}", row, column)
}

/// What the test changes between frames, and what the table did.
pub struct TableState {
    pub rows: Rows,
    pub renderer: RowsRenderer,
    /// Taken on the next frame, so the table scrolls once.
    pub scroll_to_row: Option<(usize, Option<Align>)>,
    pub actions: Vec<Action>,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            rows: Rows {
                row_count: ROW_COUNT,
            },
            renderer: RowsRenderer::default(),
            scroll_to_row: None,
            actions: vec![],
        }
    }
}

pub fn table_harness(state: TableState) -> Harness<'static, TableState> {
    let mut harness = Harness::builder().with_size((400.0, 300.0)).build_ui_state(
        |ui, state: &mut TableState| {
            let mut table = DeferredTable::new(Id::new("table"));
            if let Some((row, align)) = state.scroll_to_row.take() {
                table = table.scroll_to_row(row, align);
            }
            let (_response, actions) = table.show(ui, &mut state.rows, &mut state.renderer);
            state.actions.extend(actions);
        },
        state,
    );
    harness.run();
    harness
}

pub fn is_shown(harness: &Harness<'_, TableState>, row: usize) -> bool {
    harness.query_by_label(&cell_label(row, 0)).is_some()
}
