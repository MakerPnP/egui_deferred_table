use crate::CellIndex;
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub enum Action {
    CellClicked(CellIndex),

    /// Generated when the user drags-and-drops one column onto another.
    ///
    /// Handle it as follows:
    /// a) updating the column ordering information appropriately.
    /// d) updating the underlying data source, without re-ordering columns themselves.
    /// c) ignore it, e.g. if it's unsupported, or the columns/data are locked.
    ///
    /// See also:
    /// 1. [`crate::DeferredTableDataSource::column_ordering`]
    /// 2. [`ordering::apply_reordering`]
    ColumnReorder {
        from: usize,
        to: usize,
    },

    /// Generated when the user drags-and-drops one row onto another.
    ///
    /// Handle it as follows:
    /// a) updating the row ordering information appropriately.
    /// d) updating the underlying data source, without re-ordering rows themselves.
    /// c) ignore it, e.g. if it's unsupported, or the rows/data are locked.
    ///
    /// See also:
    /// 1. [`crate::DeferredTableDataSource::row_ordering`]
    /// 2. [`ordering::apply_reordering`]
    RowReorder {
        from: usize,
        to: usize,
    },

    /// Generated when the user selected or deselects one or more rows.
    RowSelectionChanged {
        selection: BTreeSet<usize>,
    },
}
