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
        data_source: &mut DataSource,
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

    fn column_name(&self, index: usize) -> String {
        unimplemented!()
    }
}

pub struct DeferredTableBuilder<'a, DataSource> {
    table: Table,

    state: &'a mut DeferredTableState,
    source_state: &'a mut SourceState,

    data_source: &'a mut DataSource,
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
        data_source: &'a mut DataSource,
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

    pub fn data_source(&mut self) -> &DataSource {
        self.data_source
    }

    pub fn data_source_mut(&mut self) -> &mut DataSource {
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
    data_source: &'a mut DataSource,
}

impl<'a, DataSource> HeaderBuilder<'a, DataSource> {
    fn new(
        table: &'a mut Table,
        state: &'a mut DeferredTableState,
        source_state: &'a mut SourceState,
        data_source: &'a mut DataSource,
    ) -> Self {
        Self {
            table,
            state,
            source_state,
            data_source,
        }
    }

    pub fn source(&mut self) -> &mut DataSource {
        self.data_source
    }

    pub fn current_dimensions(&self) -> (usize, usize) {
        self.source_state.dimensions
    }

    pub fn column(&mut self, index: usize, name: String) {
        self.table.columns.insert(index, name);
    }
}


pub trait TableValue: Sized + Display {}

// TODO add more types
impl TableValue for f32 {}
impl TableValue for String {}
impl TableValue for usize {}

// convert into a macro for various tuple sizes
impl<A: TableValue, B: TableValue, C: TableValue, D: TableValue> DeferredTableDataSource for &mut [(A,B,C,D)] {
    fn get_dimensions(&self) -> (usize, usize) {
        (self.len(), 4)
    }

    fn column_name(&self, index: usize) -> String {
        "N/A".to_string()
    }

    fn render_cell(&self, ui: &mut Ui, row: usize, col: usize) {
        let row = self.get(row).unwrap();
        let value = match col {
            0 => row.0.to_string(),
            1 => row.1.to_string(),
            2 => row.2.to_string(),
            3 => row.3.to_string(),
            _ => unreachable!(),
        };

        ui.label(value);
    }
}