//! A table of numbered rows, shown in a harness with no window, for tests of the table's behaviour.

// each test uses only some of these.
#![allow(dead_code)]

use egui::{Align, Event, Id, Modifiers, PointerButton, Pos2, Ui};
use egui_deferred_table::{
    Action, CellIndex, DeferredTable, DeferredTableDataSource, DeferredTableRenderer,
    TableDimensions,
};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, Queryable};
use std::collections::BTreeSet;

pub const ROW_COUNT: usize = 100;
pub const WINDOW_WIDTH: f32 = 400.0;
pub const WINDOW_HEIGHT: f32 = 300.0;

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
    /// How many lines of text are shown above the table.
    pub lines_above: usize,
    pub selectable_rows: bool,
    pub select_rows_on_cell_click: bool,
    /// The rows selected, when the app holds them.
    pub row_selection: Option<BTreeSet<usize>>,
}

impl TableState {
    /// The rows selected, as the last `Action::RowSelectionChanged` gave them.
    pub fn last_selection_changed(&self) -> Option<&BTreeSet<usize>> {
        self.actions.iter().rev().find_map(|action| match action {
            Action::RowSelectionChanged { selection } => Some(selection),
            _ => None,
        })
    }

    pub fn selection_changes(&self) -> usize {
        self.actions
            .iter()
            .filter(|action| matches!(action, Action::RowSelectionChanged { .. }))
            .count()
    }
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
            lines_above: 0,
            selectable_rows: true,
            select_rows_on_cell_click: false,
            row_selection: None,
        }
    }
}

pub fn table_harness(state: TableState) -> Harness<'static, TableState> {
    let mut harness = Harness::builder()
        .with_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .build_ui_state(
            |ui, state: &mut TableState| {
                for line in 0..state.lines_above {
                    ui.label(format!("above the table {}", line));
                }
                let mut table = DeferredTable::new(Id::new("table"));
                if let Some((row, align)) = state.scroll_to_row.take() {
                    table = table.scroll_to_row(row, align);
                }
                if !state.selectable_rows {
                    table = table.selectable_rows_disabled();
                }
                if state.select_rows_on_cell_click {
                    table = table.select_rows_on_cell_click();
                }
                if let Some(row_selection) = state.row_selection.as_mut() {
                    table = table.row_selection(row_selection);
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

/// Where a value cell is shown.
pub fn cell_position(harness: &Harness<'_, TableState>, row: usize, column: usize) -> Pos2 {
    harness
        .get_by_label(&cell_label(row, column))
        .rect()
        .center()
}

/// Where a row's header is shown, to the left of its first cell.  The row headers are one-based.
pub fn row_header_position(harness: &Harness<'_, TableState>, row: usize) -> Pos2 {
    let first_cell = harness.get_by_label(&cell_label(row, 0)).rect();
    let header = harness
        .query_all(By::new().label(&(row + 1).to_string()))
        .map(|header| header.rect())
        .find(|header| {
            header.right() <= first_cell.left()
                && (header.center().y - first_cell.center().y).abs() < 4.0
        })
        .expect("the row has a header");
    header.center()
}

/// Presses and releases the primary button at the position, with the modifiers held.
pub fn click_at(harness: &mut Harness<'_, TableState>, position: Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(position));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        });
        harness.step();
    }
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

/// Presses the primary button at one position, and releases it at another.
pub fn drag(harness: &mut Harness<'_, TableState>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.step();
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    for step in 1..=5 {
        harness.event(Event::PointerMoved(from.lerp(to, step as f32 / 5.0)));
        harness.step();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}
