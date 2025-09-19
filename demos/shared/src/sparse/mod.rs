use egui::Color32;
use egui_deferred_table::{
    CellIndex, DeferredTableDataSource, DeferredTableRenderer, TableDimensions,
};
use fastrand::Rng;
use indexmap::map::IndexMap;
use log::trace;
use names::Generator;
use std::cell::Cell;

pub mod ui;
#[derive(Debug)]
pub enum CellKind {
    Float(f32),
    Boolean(bool),
    Text(String),
}

#[derive(Debug)]
pub struct SparseMapSource<V> {
    sparse_map: IndexMap<usize, IndexMap<usize, V>>,

    // cached dimensions, lazily calculated
    extents: Cell<Option<TableDimensions>>,
}

#[derive(Debug)]
pub struct SparseMapRenderer {
    rows_to_filter: Option<Vec<usize>>,
    columns_to_filter: Option<Vec<usize>>,

    row_ordering: Option<Vec<usize>>,
    column_ordering: Option<Vec<usize>>,
}

impl SparseMapRenderer {
    pub fn new() -> Self {
        Self {
            rows_to_filter: None,
            columns_to_filter: None,

            row_ordering: None,
            column_ordering: None,
        }
    }
}

impl DeferredTableRenderer<SparseMapSource<CellKind>> for SparseMapRenderer {
    fn render_cell(
        &self,
        ui: &mut egui::Ui,
        cell_index: CellIndex,
        data_source: &SparseMapSource<CellKind>,
    ) {
        if let Some(value) = data_source.get(cell_index.row, cell_index.column) {
            match value {
                // use some arbitrary formatting and color so we can tell the difference between the data types
                CellKind::Float(value) => {
                    ui.colored_label(Color32::LIGHT_GREEN, format!("{:.2}", value));
                }
                CellKind::Boolean(value) => {
                    ui.add_enabled_ui(false, |ui| {
                        let mut value = *value;
                        ui.add(egui::Checkbox::without_text(&mut value));
                    });
                }
                CellKind::Text(value) => {
                    ui.colored_label(Color32::LIGHT_BLUE, value);
                }
            }
        }
    }

    fn rows_to_filter(&self) -> Option<&[usize]> {
        self.rows_to_filter.as_ref().map(|v| v.as_slice())
    }

    fn columns_to_filter(&self) -> Option<&[usize]> {
        self.columns_to_filter.as_ref().map(|v| v.as_slice())
    }

    fn row_ordering(&self) -> Option<&[usize]> {
        self.row_ordering.as_ref().map(|v| v.as_slice())
    }

    fn column_ordering(&self) -> Option<&[usize]> {
        self.column_ordering.as_ref().map(|v| v.as_slice())
    }
}

impl<V> SparseMapSource<V> {
    pub fn new() -> Self {
        Self {
            sparse_map: IndexMap::new(),
            extents: Cell::new(None),
        }
    }

    /// insert a new value at the location, returning the previous value at the location, if any.
    pub fn insert(&mut self, row_index: usize, column_index: usize, value: V) -> Option<V> {
        let previous = self
            .sparse_map
            .entry(row_index)
            .or_default()
            .insert(column_index, value);
        if previous.is_none() {
            self.extents.set(None);
        }
        previous
    }

    pub fn get(&self, row_index: usize, column_index: usize) -> Option<&V> {
        self.sparse_map
            .get(&row_index)
            .and_then(|row| row.get(&column_index))
    }
}

impl<V> DeferredTableDataSource for SparseMapSource<V> {
    fn get_dimensions(&self) -> TableDimensions {
        if let Some(extents) = self.extents.get() {
            return extents;
        }

        let extents = self
            .sparse_map
            .iter()
            .fold(None, |extents, (row_number, row)| {
                let (mut max_row_index, mut max_column_index) =
                    extents.unwrap_or((0_usize, 0_usize));

                max_column_index = max_column_index.max(row.keys().fold(
                    0_usize,
                    |max_column_index_for_this_row, column_index| {
                        max_column_index_for_this_row.max(*column_index)
                    },
                ));
                max_row_index = max_row_index.max(*row_number);

                Some((max_row_index, max_column_index))
            });

        let Some((max_row_index, max_column_index)) = extents else {
            return TableDimensions::default();
        };

        trace!(
            "recalculated extents. max_row_index: {}, max_column_index: {}",
            max_row_index, max_column_index
        );

        let extents = TableDimensions {
            row_count: max_row_index + 1,
            column_count: max_column_index + 1,
        };

        self.extents.set(Some(extents));

        extents
    }
}

pub enum CellKindChoice {
    Float,
    Boolean,
    Text,
}

#[cfg(test)]
mod sparse_map_source_tests {
    use super::SparseMapSource;
    use egui_deferred_table::{DeferredTableDataSource, TableDimensions};

    #[test]
    pub fn dimensions_for_empty_source() {
        // given
        let source = SparseMapSource::<usize>::new();
        // when
        assert_eq!(
            source.get_dimensions(),
            TableDimensions {
                row_count: 0,
                column_count: 0
            }
        );
    }

    #[test]
    pub fn dimensions_for_1x2_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(0, 1, 69);

        // when
        assert_eq!(
            source.get_dimensions(),
            TableDimensions {
                row_count: 1,
                column_count: 2
            }
        );
    }

    #[test]
    pub fn dimensions_for_2x1_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(1, 0, 69);

        // when
        assert_eq!(
            source.get_dimensions(),
            TableDimensions {
                row_count: 2,
                column_count: 1
            }
        );
    }

    #[test]
    pub fn dimensions_for_2x2_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(0, 0, 42);
        source.insert(0, 1, 69);

        source.insert(1, 0, 0x42);
        source.insert(1, 1, 0x69);

        // when
        assert_eq!(
            source.get_dimensions(),
            TableDimensions {
                row_count: 2,
                column_count: 2
            }
        );
    }

    #[test]
    pub fn dimensions_for_sparse_source() {
        // given
        let mut source = SparseMapSource::<usize>::new();
        source.insert(4, 9, 0x42);

        source.insert(0, 0, 42);
        source.insert(1, 1, 69);

        source.insert(9, 4, 0x69);

        // when
        assert_eq!(
            source.get_dimensions(),
            TableDimensions {
                row_count: 10,
                column_count: 10
            }
        );
    }
}

pub fn generate_data(
    data: &mut SparseMapSource<CellKind>,
    max_rows: usize,
    max_columns: usize,
    max_cell_values: usize,
    rng: &mut Rng,
    name_gen: &mut Generator,
) {
    (0..max_cell_values).for_each(|_index| {
        let row_index = rng.usize(0..max_rows);
        let column_index = rng.usize(0..max_columns);

        let kind = rng.usize(0..3);
        let cell_kind = match kind {
            0 => CellKind::Float(rng.f32()),
            1 => CellKind::Boolean(rng.bool()),
            2 => CellKind::Text(name_gen.next().unwrap()),
            _ => unreachable!(),
        };

        data.insert(row_index, column_index, cell_kind);
    });

    trace!("data: {:?}", data);
}
