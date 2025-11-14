use crate::CellIndex;
use egui::Ui;

///
/// Editable table support
///

#[derive(Debug, Clone)]
pub enum CellEditState<E, T> {
    /// the pivot point for selections, etc.
    Pivot(CellIndex),
    /// when editing a cell, we need state for the cell and a copy of the original row to be able to track changes
    Editing(CellIndex, E, T),
}

pub trait ApplyChange<T, E> {
    fn apply_change(&mut self, value: T) -> Result<(), E>;
}

/// Implement this to enable data source editing support.
pub trait EditableTableRenderer<DataSource> {
    /// Usually a type containing the data for a single row.
    type Value;
    /// Usually an enum, with variants for each type of cell that can be edited.
    /// e.g. `Number(f32)`, `Text(String)`...
    type ItemState;

    /// Called when the cell needs to be edited.
    ///
    /// Return None to prevent editing or a tuple containing the ItemState and the original value.
    fn build_item_state(
        &self,
        cell_index: CellIndex,
        source: &mut DataSource,
    ) -> Option<(Self::ItemState, Self::Value)>;

    /// Called when the cell is no-longer being edited.
    ///
    /// Implementations usually modify the data source directly, or build and send a command that will change
    /// eventually update the datasource, e.g. via a message queue and/or in a background thread.
    fn on_edit_complete(
        &mut self,
        cell_index: CellIndex,
        state: Self::ItemState,
        original_item: Self::Value,
        source: &mut DataSource,
    );

    /// item state is what the editor should actually edit
    /// original item is supplied so that editor can show differences indicators when state has changed
    /// data source is supplied in case it's needed
    fn render_cell_editor(
        &self,
        ui: &mut Ui,
        cell_index: &CellIndex,
        state: &mut Self::ItemState,
        original_item: &Self::Value,
        source: &mut DataSource,
    );
}

#[derive(Debug, Clone)]
pub struct EditorState<IS, V> {
    pub state: Option<CellEditState<IS, V>>,
}

impl<IS, V> Default for EditorState<IS, V> {
    fn default() -> Self {
        Self { state: None }
    }
}

/// A dummy editor to keep the compiler happy, should get compiled out.
pub struct NullEditor {}

impl<DataSource> EditableTableRenderer<DataSource> for NullEditor {
    type Value = ();
    type ItemState = ();

    fn build_item_state(
        &self,
        cell_index: CellIndex,
        source: &mut DataSource,
    ) -> Option<(Self::ItemState, Self::Value)> {
        let (_, _) = (cell_index, source);
        unreachable!()
    }

    fn on_edit_complete(
        &mut self,
        index: CellIndex,
        state: Self::ItemState,
        original_item: Self::Value,
        source: &mut DataSource,
    ) {
        let (_, _, _, _) = (index, state, original_item, source);
        unreachable!()
    }

    fn render_cell_editor(
        &self,
        ui: &mut Ui,
        cell_index: &CellIndex,
        state: &mut Self::ItemState,
        original_item: &Self::Value,
        source: &mut DataSource,
    ) {
        let (_, _, _, _, _) = (ui, cell_index, state, original_item, source);
        unreachable!()
    }
}
