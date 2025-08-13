use std::fmt::Display;
use std::marker::PhantomData;
use egui::{Id, Response, Ui};
use indexmap::IndexMap;

pub struct DeferredTable<DataSource> {
    id: Id,
    phantom_data: PhantomData<DataSource>
}

impl<DataSource> DeferredTable<DataSource> {

    pub fn new(id: Id) -> Self {
        Self {
            id,
            phantom_data: PhantomData,
        }
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        data_source: &DataSource,
        build_table_view: impl FnOnce(&mut DeferredTableBuilder<'_, DataSource>),
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
    {
        let mut actions = vec![];

        // TODO load from egui memory
        let mut state = DeferredTableState::default();

        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        let mut source_state = SourceState {
            dimensions,
        };

        let mut builder = DeferredTableBuilder::new(&mut state, &mut source_state, data_source);

        build_table_view(&mut builder);

        //
        // display the table
        //
        ui.horizontal(|ui|{
            let column_count = builder.table.columns.len();
            for column_number in 0..=column_count {
                ui.vertical(|ui| {

                    let column_index = if column_number == 0 { None } else { Some(column_number - 1)};

                    //
                    // heading
                    //

                    if let Some(column_index) = column_index {
                        let column = builder.table.columns.get(&column_index);
                        if let Some(column) = column {
                            ui.label(column);
                        }
                    } else {
                        ui.label("!");
                    }

                    //
                    // values
                    //
                    for row_index in 0..source_state.dimensions.0 {
                        if let Some(column_index) = column_index {
                            data_source.render_cell(ui, row_index, column_index);
                        } else {
                            ui.label((row_index + 1).to_string());
                        }
                    }
                });
            }
        });

        // TODO save state to egui memory

        (ui.response(), actions)
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Placeholder,
    // SelectionChanged, etc.
}

#[derive(Default)]
struct DeferredTableState {
    // TODO column widths
    // TODO column ordering
    // TODO column visibility
    // TODO cell selection
}

pub trait DeferredTableDataSource {
    fn get_dimensions(&self) -> (usize, usize);
    fn render_cell(&self, ui: &mut Ui, row: usize, col: usize);
}

pub struct DeferredTableBuilder<'a, DataSource> {
    table: Table,

    state: &'a mut DeferredTableState,
    source_state: &'a mut SourceState,

    data_source: &'a DataSource,
}

impl<'a, DataSource> DeferredTableBuilder<'a, DataSource> {
    pub fn header(&mut self, builder_header_view: fn(&'_ mut HeaderBuilder<'_, DataSource>)) {

        let mut header_builder = HeaderBuilder::new(&mut self.table, self.state, &mut self.source_state, self.data_source);

        builder_header_view(&mut header_builder);

    }
}

struct Table {
    columns: IndexMap<usize, String>,

    // TODO column groups here..
}

impl<'a, DataSource> DeferredTableBuilder<'a, DataSource> {
    fn new(
        state: &'a mut DeferredTableState,
        source_state: &'a mut SourceState,
        data_source: &'a DataSource,
    ) -> Self
    where
        DataSource: DeferredTableDataSource,
    {
        let table = Table {
            columns: IndexMap::new(),
        };

        Self {
            table,
            state,
            source_state,
            data_source,
        }
    }

    pub fn source(&mut self) -> &DataSource {
        self.data_source
    }
}

#[derive(Debug)]
struct SourceState {
    /// (rows, columns) aka (x,y)
    dimensions: (usize, usize),
}

pub struct HeaderBuilder<'a, DataSource> {
    table: &'a mut Table,
    state: &'a mut DeferredTableState,
    source_state: &'a mut SourceState,
    data_source: &'a DataSource,
}

impl<'a, DataSource> HeaderBuilder<'a, DataSource> {
    fn new(
        table: &'a mut Table,
        state: &'a mut DeferredTableState,
        source_state: &'a mut SourceState,
        data_source: &'a DataSource,
    ) -> Self {
        Self {
            table,
            state,
            source_state,
            data_source,
        }
    }

    pub fn source(&mut self) -> &DataSource {
        self.data_source
    }

    pub fn current_dimensions(&self) -> (usize, usize) {
        self.source_state.dimensions
    }

    pub fn column(&mut self, index: usize, name: String) {
        self.table.columns.insert(index, name);
    }
}

// define a macro that handles the implementation for a specific tuple size
macro_rules! impl_tuple_for_size {
    // Pattern: tuple type names, tuple size, match arms for indexing
    (($($T:ident),*), $size:expr, $( ($idx:expr, $field:tt) ),* ) => {
        impl<$($T: Display),*> DeferredTableDataSource for &[($($T),*)] {
            fn get_dimensions(&self) -> (usize, usize) {
                (self.len(), $size)
            }

            fn render_cell(&self, ui: &mut Ui, row: usize, col: usize) {
                if let Some(row_data) = self.get(row) {
                    match col {
                        $( $idx => ui.label(row_data.$field.to_string()), )*
                        _ => unreachable!(),
                    };
                }
            }
        }
    };
}

// use a front-end macro that calls the implementation macro with the right parameters
macro_rules! impl_deferred_table_for_tuple {
    // 2-tuple
    ((A, B), 2) => {
        impl_tuple_for_size!((A, B), 2, (0, 0), (1, 1));
    };
    
    // 3-tuple
    ((A, B, C), 3) => {
        impl_tuple_for_size!((A, B, C), 3, (0, 0), (1, 1), (2, 2));
    };
    
    // 4-tuple
    ((A, B, C, D), 4) => {
        impl_tuple_for_size!((A, B, C, D), 4, (0, 0), (1, 1), (2, 2), (3, 3));
    };
    
    // 5-tuple
    ((A, B, C, D, E), 5) => {
        impl_tuple_for_size!((A, B, C, D, E), 5, (0, 0), (1, 1), (2, 2), (3, 3), (4, 4));
    };
}

impl_deferred_table_for_tuple!((A, B), 2);
impl_deferred_table_for_tuple!((A, B, C), 3);
impl_deferred_table_for_tuple!((A, B, C, D), 4);
impl_deferred_table_for_tuple!((A, B, C, D, E), 5);