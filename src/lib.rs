use std::fmt::Display;
use std::marker::PhantomData;
use egui::{Align, Color32, CornerRadius, Id, Layout, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2};
use indexmap::IndexMap;

pub struct DeferredTable<DataSource> {
    id: Id,
    parameters: DeferredTableParameters,
    phantom_data: PhantomData<DataSource>
}

#[derive(Default)]
struct DeferredTableParameters {
    default_cell_size: Option<Vec2>,
    default_origin: Option<CellIndex>,
}


impl<DataSource> DeferredTable<DataSource> {

    pub fn new(id: Id) -> Self {
        Self {
            id,
            parameters: DeferredTableParameters::default(),
            phantom_data: PhantomData,
        }
    }

    pub fn default_cell_size(mut self, size: Vec2) -> Self {
        self.parameters.default_cell_size = Some(size);
        self
    }

    pub fn default_origin(mut self, origin: CellIndex) -> Self {
        self.parameters.default_origin = Some(origin);
        self
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        data_source: &DataSource,
        build_table_view: impl FnOnce(&mut DeferredTableBuilder<'_, DataSource>),
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource + DeferredTableRenderer,
    {
        let mut actions = vec![];

        let style = ui.style();

        // TODO load from egui memory
        let mut state = DeferredTableState {
            cell_size: self.parameters.default_cell_size.unwrap_or((
                style.spacing.interact_size.x + (style.spacing.item_spacing.x * 2.0),
                style.spacing.interact_size.y + (style.spacing.item_spacing.y * 2.0),
            ).into()),

            cell_origin: self.parameters.default_origin.unwrap_or(CellIndex::default()),

            // TODO use a constant for this
            min_size: (400.0, 200.0).into(),
            ..DeferredTableState::default()
        };

        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        let mut source_state = SourceState {
            dimensions,
        };

        let parent_max_rect = ui.max_rect();
        let parent_clip_rect = ui.clip_rect();
        if false {
            ui.painter().debug_rect(parent_max_rect, Color32::GREEN, "pmr");
            ui.painter().debug_rect(parent_clip_rect, Color32::RED, "pcr");
        }
        
        let outer_min_rect = Rect::from_min_size(ui.next_widget_position(), state.min_size.clone());
        let outer_max_rect = outer_min_rect.union(parent_max_rect);

        //println!("frame");
        ui.scope_builder(UiBuilder::new().max_rect(outer_max_rect), |ui|{
            
            let inner_clip_rect = ui.clip_rect();
            let inner_max_rect = ui.max_rect();
            
            let cell_size = state.cell_size.clone();
            let cell_origin = state.cell_origin.clone();
            // uncomment to test offset
            //let cell_origin = CellIndex::from((2,3));
    
            let visible_rows = source_state.dimensions.row_count - cell_origin.row;
            let visible_columns = source_state.dimensions.column_count - cell_origin.column;
    
            let mut builder = DeferredTableBuilder::new(&mut state, &mut source_state, data_source);
    
            build_table_view(&mut builder);
    
            //
            // display the table
            //
    
            //ui.painter().debug_rect(inner_max_rect, Color32::CYAN, "imr");
            //ui.painter().debug_rect(inner_clip_rect, Color32::PURPLE, "ic");
    
            ui.scope_builder(UiBuilder::new().max_rect(inner_max_rect), |ui|{
    
                let mut start_pos = ui.cursor().min;
    
                for grid_row_index in 0..=visible_rows {
    
                    let row_number = grid_row_index + cell_origin.row;
    
                    let y = start_pos.y + (grid_row_index as f32 * cell_size.y);
    
                    // TODO handle individual column sizes
                    for grid_column_index in 0..=visible_columns {
    
                        let column_number = grid_column_index + cell_origin.column;
                        
                        let cell_index = if grid_row_index > 0 && grid_column_index > 0 {
                            Some(CellIndex {
                                row: cell_origin.row + (grid_row_index - 1),
                                column: cell_origin.column + (grid_column_index - 1),
                            })
                        } else {
                            None
                        };
    
                        let x = start_pos.x + (grid_column_index as f32 * cell_size.x);
    
                        let cell_rect = Rect::from_min_size(Pos2::new(x, y), cell_size);
                        let cell_clip_rect = cell_rect.intersect(inner_max_rect);
                        //ui.painter().debug_rect(render_rect, Color32::GRAY, "rr");
    
                        if !inner_max_rect.intersects(cell_clip_rect) {
                            continue;
                        }

                        //println!("rendering. grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect.size());
                        let response = ui.allocate_rect(cell_clip_rect, Sense::click());



                        if cell_index.is_some() {
                            let bg_color = if response.contains_pointer() {
                                ui.style().visuals.widgets.hovered.bg_fill
                        } else {
                                ui.style().visuals.panel_fill
                            };
                            ui.painter()
                                .with_clip_rect(cell_clip_rect)
                                .rect_filled(cell_rect, 0.0, bg_color);
                        }
    
                        ui.painter()
                            .with_clip_rect(cell_clip_rect)
                            .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);
    
                        let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(cell_rect));
                        cell_ui.set_clip_rect(cell_clip_rect);
                        cell_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    
                        if let Some(cell_index) = cell_index {
                            data_source.render_cell(&mut cell_ui, cell_index);
                        } else {
                            if grid_row_index == 0 && grid_column_index == 0 {
                                cell_ui.label("!");
                            } else if grid_row_index == 0 {
    
                                let cell_column_index = cell_origin.column + (grid_column_index - 1);
    
                                if let Some(column_name) = builder.table.columns.get(&cell_column_index) {
                                    cell_ui.label(column_name);
                                }
                                else {
                                    cell_ui.label(column_number.to_string());
                                }
                            } else {
                                cell_ui.label(row_number.to_string());
                            }
                        }
                    }
                }
            });
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellIndex {
    pub row: usize,
    pub column: usize,
}

impl From<(usize, usize)> for CellIndex {
    // column then row ordering in tuple to align with x/y so it's easier to remember
    fn from(value: (usize, usize)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TableDimensions {
    pub row_count: usize,
    pub column_count: usize,
}

impl From<(usize, usize)> for TableDimensions {
    // column then row ordering in tuple to align with x/y so it's easier to remember
    fn from(value: (usize, usize)) -> Self {
        Self {
            column_count: value.0,
            row_count: value.1,
        }
    }
}


#[derive(Default)]
struct DeferredTableState {
    min_size: Vec2,
    cell_size: Vec2,
    cell_origin: CellIndex,
    // TODO column ordering
    // TODO column visibility
    // TODO cell selection
}

pub trait DeferredTableDataSource {
    fn get_dimensions(&self) -> TableDimensions;
}

pub trait DeferredTableRenderer {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex);
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
        DataSource: DeferredTableDataSource + DeferredTableRenderer,
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
    dimensions: TableDimensions,
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

    pub fn current_dimensions(&self) -> TableDimensions {
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
        impl<$($T),*> DeferredTableDataSource for &[($($T),*)] {
            fn get_dimensions(&self) -> TableDimensions {
                TableDimensions {
                    row_count: self.len(),
                    column_count: $size,
                }
            }
        }

        impl<$($T: Display),*> DeferredTableRenderer for &[($($T),*)] {
            fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex) {
                if let Some(row_data) = self.get(cell_index.row) {
                    match cell_index.column {
                        $( $idx => ui.label(row_data.$field.to_string()), )*
                        _ => panic!("cell_index out of bounds. {:?}", cell_index),
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