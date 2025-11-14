use crate::CellIndex;
use egui::Ui;

pub trait DeferredTableRenderer<DataSource> {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &DataSource);

    /// return a list of rows indexes to filter/exclude.
    fn rows_to_filter(&self) -> Option<&[usize]> {
        None
    }

    /// return a list of column indexes to filter/exclude.
    fn columns_to_filter(&self) -> Option<&[usize]> {
        None
    }

    /// return a list of row indexes to set the ordering of rows
    ///
    /// the index of the slice corresponds to the index of the visible row
    /// the value of the slace at the index corresponds to the index of the data
    ///
    /// e.g. `Some(vec![1,0])` would swap rows 0 and 1.
    fn row_ordering(&self) -> Option<&[usize]> {
        None
    }

    /// return a list of row indexes to set the ordering of columns
    ///
    /// the index of the slice corresponds to the index of the visible column
    /// the value of the slace at the index corresponds to the index of the data
    ///
    /// e.g. `Some(vec![1,0])` would swap columns 0 and 1.
    fn column_ordering(&self) -> Option<&[usize]> {
        None
    }
}
