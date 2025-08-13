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
    
    pub fn show<V>(
        &self, 
        ui: &mut Ui,
        data_source: &mut DataSource,
        build_table_view: impl FnOnce(&mut DeferredTableBuilder<'_, DataSource, V>),
    ) -> (Response, Vec<Action>) 
    where
        DataSource: DeferredTableDataSource<V>,
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
                    
                    let mut column_index = if column_number == 0 { None } else { Some(column_number - 1)};
                    
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

pub trait DeferredTableDataSource<V> {
    fn get_dimensions(&self) -> (usize, usize);
    fn get_cell_value(&self, row: usize, col: usize) -> Option<V>;
    fn column_name(&self, index: usize) -> String;
    fn render_cell(&self, ui: &mut Ui, row: usize, col: usize);
}

pub struct DeferredTableBuilder<'a, DataSource, V> {
    table: Table,
    
    state: &'a mut DeferredTableState,
    source_state: &'a mut SourceState,
    
    data_source: &'a mut DataSource,
    
    phantom_data: PhantomData<V>,
}

impl<'a, DataSource, V> DeferredTableBuilder<'a, DataSource, V> {
    pub fn header(&mut self, builder_header_view: fn(&'_ mut HeaderBuilder<'_, DataSource, V>)) {
        
        let mut header_builder = HeaderBuilder::new(&mut self.table, self.state, &mut self.source_state, self.data_source);
        
        builder_header_view(&mut header_builder);
        
    }
}

struct Table {
    columns: IndexMap<usize, String>,
    
    // TODO column groups here..
}

impl<'a, DataSource, V> DeferredTableBuilder<'a, DataSource, V> {
    fn new(
        state: &'a mut DeferredTableState, 
        source_state: &'a mut SourceState,
        data_source: &'a mut DataSource, 
    ) -> Self
    where
        DataSource: DeferredTableDataSource<V>,
    {
        let table = Table {
            columns: IndexMap::new(),
        };
        
        Self {
            table,
            state,
            source_state,
            data_source,
            phantom_data: Default::default(),
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

pub struct HeaderBuilder<'a, DataSource, V> {
    table: &'a mut Table,
    state: &'a mut DeferredTableState,
    source_state: &'a mut SourceState,
    data_source: &'a mut DataSource,
    
    phantom_data: PhantomData<V>,
}

impl<'a, DataSource, V> HeaderBuilder<'a, DataSource, V> {
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
            phantom_data: Default::default(),
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