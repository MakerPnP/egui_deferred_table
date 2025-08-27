use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, CornerRadius, Id, Pos2, Rect, Response, Sense, StrokeKind, Style, Ui, UiBuilder, Vec2,
};
use indexmap::IndexMap;
use log::trace;
use std::fmt::Display;
use std::marker::PhantomData;

pub struct DeferredTable<DataSource> {
    id: Id,
    parameters: DeferredTableParameters,
    phantom_data: PhantomData<DataSource>,
}

struct DeferredTableParameters {
    default_cell_size: Option<Vec2>,
    zero_based_headers: bool,
    min_size: Vec2,
}

impl Default for DeferredTableParameters {
    fn default() -> Self {
        Self {
            default_cell_size: None,
            zero_based_headers: false,
            min_size: Vec2::new(400.0, 200.0),
        }
    }
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

    pub fn zero_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = true;
        self
    }

    pub fn one_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = false;
        self
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.parameters.min_size = size;
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
        trace!("table");

        let mut actions = vec![];

        let style = ui.style();

        let mut state = DeferredTableState {
            cell_size: self.parameters.default_cell_size.unwrap_or(
                (
                    style.spacing.interact_size.x + (style.spacing.item_spacing.x * 2.0),
                    style.spacing.interact_size.y + (style.spacing.item_spacing.y * 2.0),
                )
                    .into(),
            ),

            // TODO use a constant for this
            min_size: self.parameters.min_size,
            ..DeferredTableState::default()
        };

        // TODO override some state from egui memory, e.g. individual column widths

        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        let mut source_state = SourceState { dimensions };

        let available_rect_before_wrap = ui.available_rect_before_wrap();
        if false {
            ui.painter()
                .debug_rect(available_rect_before_wrap, Color32::WHITE, "arbr");
        }

        let parent_max_rect = ui.max_rect();
        let parent_clip_rect = ui.clip_rect();
        if false {
            ui.painter()
                .debug_rect(parent_max_rect, Color32::GREEN, "pmr");
            ui.painter()
                .debug_rect(parent_clip_rect, Color32::RED, "pcr");
        }

        // the x/y of this can have negative values if the OUTER scroll area is scrolled right or down, respectively.
        // i.e. if the outer scroll area scrolled down, the y will be negative, above the visible area.
        let outer_next_widget_position = ui.next_widget_position();
        trace!(
            "outer_next_widget_position: {:?}",
            outer_next_widget_position
        );

        // if there is content above the table, we use this min rect so we to define an area starting at the right place.
        let outer_min_rect =
            Rect::from_min_size(outer_next_widget_position, state.min_size.clone());
        // FIXME if the parent_max_rect is too small, min_size is not respected, but using
        //       ... `parent_max_rect.size().at_least(state.min_size)` causes rendering errors
        let outer_max_rect =
            Rect::from_min_size(outer_next_widget_position, parent_max_rect.size());
        trace!("outer_min_rect: {:?}", outer_min_rect);
        trace!("outer_max_rect: {:?}", outer_max_rect);


        if false {
            ui.painter()
                .debug_rect(outer_min_rect, Color32::GREEN, "omnr");
            ui.painter()
                .debug_rect(outer_max_rect, Color32::RED, "omxr");
        }
        ui.scope_builder(UiBuilder::new().max_rect(outer_max_rect), |ui|{

            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

            let inner_clip_rect = ui.clip_rect();
            let inner_max_rect = ui.max_rect();

            let cell_size = state.cell_size.clone();

            let y_size = inner_max_rect.size().y;
            let x_size = inner_max_rect.size().x;
            let possible_rows = (y_size / cell_size.y).ceil() as usize;
            let possible_columns = (x_size / cell_size.x).ceil() as usize;
            trace!("possible_rows: {}, possible_columns: {}", possible_rows, possible_columns);

            let y_size = inner_clip_rect.size().y;
            let x_size = inner_clip_rect.size().x;
            let visible_possible_rows = (y_size / cell_size.y).ceil() as usize;
            let visible_possible_columns = (x_size / cell_size.x).ceil() as usize;
            trace!("visible_possible_rows: {}, visible_possible_columns: {}", visible_possible_rows, visible_possible_columns);

            let available_rows = source_state.dimensions.row_count;
            let available_columns = source_state.dimensions.column_count;
            trace!("available_rows: {}, available_columns: {}", available_rows, available_columns);

            let mut builder = DeferredTableBuilder::new(&mut state, &mut source_state, data_source);

            build_table_view(&mut builder);


            //ui.painter().debug_rect(inner_max_rect, Color32::CYAN, "imr");
            //ui.painter().debug_rect(inner_clip_rect, Color32::PURPLE, "ic");


            let scroll_style = ui.spacing().scroll;

            //
            // container for the table and the scroll bars.
            //

            let total_content_size = Vec2::new(
                (dimensions.column_count + 1) as f32 * cell_size.x,
                (dimensions.row_count + 1) as f32 * cell_size.y
            );

            ui.scope_builder(UiBuilder::new().max_rect(inner_max_rect), |ui|{

                // table_max_rect is the rect INSIDE any OUTER scroll area, e.g. when *this* table is rendered inside a scrollarea
                // as the outer scroll area is scrolled,
                let table_max_rect = Rect::from_min_size(
                    inner_max_rect.min,
                    (
                        inner_max_rect.size().x - scroll_style.bar_width,
                        inner_max_rect.size().y - scroll_style.bar_width,
                    ).into()
                );
                //ui.ctx().debug_painter().debug_rect(table_max_rect, Color32::MAGENTA, "tmr");
                trace!("table_max_rect: {:?}", table_max_rect);

                if false {
                    ui.painter().debug_rect(inner_max_rect, Color32::PURPLE, "imr");
                    ui.painter().debug_rect(table_max_rect, Color32::MAGENTA, "tmr");
                }


                egui::ScrollArea::both()
                    .id_salt("table_scroll_area")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .show_viewport(ui, |ui, viewport_rect| {
                        trace!("max_rect: {:?}", ui.max_rect());
                        trace!("viewport_rect: {:?}", viewport_rect);
                        //ui.ctx().debug_painter().debug_rect(viewport_rect, Color32::CYAN, "vr");

                        ui.set_height(total_content_size.y);
                        ui.set_width(total_content_size.x);


                        let scroll_row_min = (viewport_rect.min.y / cell_size.y).floor() as usize;
                        let scroll_row_max = (viewport_rect.max.y / cell_size.y).ceil() as usize + 1;
                        trace!("scroll_row_min: {}, scroll_row_max: {}, ", scroll_row_min, scroll_row_max);

                        let scroll_column_min = (viewport_rect.min.x / cell_size.x).floor() as usize;
                        let scroll_column_max = (viewport_rect.max.x / cell_size.x).ceil() as usize + 1;
                        trace!("scroll_column_min: {}, scroll_column_max: {}, ", scroll_column_min, scroll_column_max);

                        let cell_origin = CellIndex {
                            row: scroll_row_min,
                            column: scroll_column_min,
                        };


                        let y_min = ui.max_rect().top() + scroll_row_min as f32 * cell_size.y;
                        let y_max = ui.max_rect().top() + scroll_row_max as f32 * cell_size.y;

                        let x_min = ui.max_rect().left() + scroll_column_min as f32 * cell_size.x;
                        let x_max = ui.max_rect().left() + scroll_column_max as f32 * cell_size.x;

                        let rect = Rect::from_x_y_ranges(x_min..=x_max, y_min..=y_max);
                        trace!("rect: {:?}", rect);


                        for grid_row_index in 0..=visible_possible_rows {
                            let row_number = grid_row_index + cell_origin.row;

                            // TODO handle individual column sizes
                            for grid_column_index in 0..=visible_possible_columns {
                                let column_number = grid_column_index + cell_origin.column;

                                if grid_row_index >= 1 && grid_column_index >= 1 {
                                    // no cell rendering
                                    break
                                }

                                if grid_row_index + cell_origin.row > dimensions.row_count ||
                                    grid_column_index + cell_origin.column > dimensions.column_count {
                                    break
                                }


                                let start_pos = if grid_column_index > 0 || grid_row_index > 0 {
                                    rect.min
                                } else {
                                    table_max_rect.min
                                };

                                let mut y = start_pos.y + (grid_row_index as f32 * cell_size.y);
                                let mut x = start_pos.x + (grid_column_index as f32 * cell_size.x);

                                if grid_row_index == 0 {
                                    y = table_max_rect.min.y;
                                }
                                if grid_column_index == 0 {
                                    x = table_max_rect.min.x;
                                }

                                let cell_rect = Rect::from_min_size(Pos2::new(x, y), cell_size);

                                let mut cell_clip_rect = cell_rect.intersect(table_max_rect);
                                if grid_row_index == 1 {
                                    cell_clip_rect.min.y = table_max_rect.min.y + cell_size.y;
                                }
                                if grid_column_index == 1 {
                                    cell_clip_rect.min.x = table_max_rect.min.x + cell_size.x;
                                }
                                let cell_clip_rect = cell_clip_rect.intersect(parent_clip_rect);

                                if false {
                                    ui.painter().debug_rect(cell_clip_rect, Color32::ORANGE, "ccr");
                                }

                                if !table_max_rect.intersects(cell_clip_rect) {
                                    continue;
                                }

                                trace!("rendering headers. grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect.size());
                                let _response = ui.allocate_rect(cell_clip_rect, Sense::click());

                                let bg_color = striped_row_color(row_number, &ui.style()).unwrap_or(ui.style().visuals.widgets.noninteractive.weak_bg_fill);

                                ui.painter()
                                    .with_clip_rect(cell_clip_rect)
                                    .rect_filled(cell_rect, 0.0, bg_color);

                                ui.painter()
                                    .with_clip_rect(cell_clip_rect)
                                    .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);

                                let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(cell_rect));
                                cell_ui.set_clip_rect(cell_clip_rect);
                                cell_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                                if grid_row_index == 0 && grid_column_index == 0 {
                                    cell_ui.label(format!("{}*{}", dimensions.column_count, dimensions.row_count));
                                } else if grid_row_index == 0 {

                                    let cell_column_index = cell_origin.column + (grid_column_index - 1);

                                    if let Some(column_name) = builder.table.columns.get(&cell_column_index) {
                                        cell_ui.label(column_name);
                                    } else if self.parameters.zero_based_headers {
                                        cell_ui.label(cell_column_index.to_string());
                                    } else {
                                        cell_ui.label(column_number.to_string());
                                    }
                                } else {
                                    let cell_row_index = cell_origin.row + (grid_row_index - 1);

                                    if self.parameters.zero_based_headers {
                                        cell_ui.label(cell_row_index.to_string());
                                    } else {
                                        cell_ui.label(row_number.to_string());
                                    }

                                }
                            }
                        }


                        let clip_rect = Rect::from_min_size(table_max_rect.min + cell_size, rect.size()).intersect(table_max_rect);
                        if false {
                            ui.painter().debug_rect(clip_rect, Color32::CYAN, "cr");
                        }

                        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                            ui.set_clip_rect(clip_rect);
                            // ui.skip_ahead_auto_ids(???); // TODO Make sure we get consistent IDs.

                            let table_max_rect = ui.max_rect();

                            //
                            // display the table
                            //

                            let start_pos = table_max_rect.min;

                            for grid_row_index in 1..=visible_possible_rows {

                                let row_number = grid_row_index + cell_origin.row;

                                let y = start_pos.y + (grid_row_index as f32 * cell_size.y);

                                // TODO handle individual column sizes
                                for grid_column_index in 1..=visible_possible_columns {

                                    let column_number = grid_column_index + cell_origin.column;

                                    if grid_row_index + cell_origin.row > dimensions.row_count ||
                                        grid_column_index + cell_origin.column > dimensions.column_count {
                                        break
                                    }

                                    let cell_index = if grid_row_index > 0 && grid_column_index > 0 {
                                        let row = cell_origin.row + (grid_row_index - 1);
                                        let column = cell_origin.column + (grid_column_index - 1);

                                        Some(CellIndex {
                                            row,
                                            column,
                                        })
                                    } else {
                                        None
                                    };

                                    let x = start_pos.x + (grid_column_index as f32 * cell_size.x);

                                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), cell_size);
                                    let cell_clip_rect = cell_rect.intersect(clip_rect).intersect(parent_clip_rect);

                                    if !table_max_rect.intersects(cell_clip_rect) {
                                        continue;
                                    }

                                    trace!("rendering cells. grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect.size());
                                    let response = ui.allocate_rect(cell_clip_rect, Sense::click());

                                    if cell_index.is_some() {
                                        let bg_color = if response.contains_pointer() {
                                            ui.style().visuals.widgets.hovered.bg_fill
                                        } else {
                                            striped_row_color(row_number, &ui.style()).unwrap_or(ui.style().visuals.panel_fill)
                                        };
                                        ui.painter()
                                            .with_clip_rect(cell_clip_rect)
                                            .rect_filled(cell_rect, 0.0, bg_color);

                                        // note: cannot use 'response.clicked()' here as the the cell 'swallows' the click if the contents are interactive.
                                        if response.contains_pointer() && ui.ctx().input(|i|i.pointer.primary_released()) {
                                            actions.push(Action::CellClicked(cell_index.unwrap()));
                                        }
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
                                            unreachable!();
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
            });
        });

        // TODO save state to egui memory

        (ui.response(), actions)
    }
}

fn striped_row_color(row: usize, style: &Style) -> Option<Color32> {
    if row % 2 == 1 {
        Some(style.visuals.faint_bg_color)
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    CellClicked(CellIndex),
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
        let mut header_builder = HeaderBuilder::new(
            &mut self.table,
            self.state,
            &mut self.source_state,
            self.data_source,
        );

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
    ((A, B), 2) => {
        impl_tuple_for_size!((A, B), 2, (0, 0), (1, 1));
    };

    ((A, B, C), 3) => {
        impl_tuple_for_size!((A, B, C), 3, (0, 0), (1, 1), (2, 2));
    };

    ((A, B, C, D), 4) => {
        impl_tuple_for_size!((A, B, C, D), 4, (0, 0), (1, 1), (2, 2), (3, 3));
    };

    ((A, B, C, D, E), 5) => {
        impl_tuple_for_size!((A, B, C, D, E), 5, (0, 0), (1, 1), (2, 2), (3, 3), (4, 4));
    };

    ((A, B, C, D, E, F), 6) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F),
            6,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5)
        );
    };

    ((A, B, C, D, E, F, G), 7) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G),
            7,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6)
        );
    };

    ((A, B, C, D, E, F, G, H), 8) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H),
            8,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7)
        );
    };

    ((A, B, C, D, E, F, G, H, I), 9) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I),
            9,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J), 10) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J),
            10,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K), 11) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K),
            11,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L), 12) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L),
            12,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M), 13) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M),
            13,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N), 14) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
            14,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O), 15) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O),
            15,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13),
            (14, 14)
        );
    };

    ((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P), 16) => {
        impl_tuple_for_size!(
            (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P),
            16,
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (5, 5),
            (6, 6),
            (7, 7),
            (8, 8),
            (9, 9),
            (10, 10),
            (11, 11),
            (12, 12),
            (13, 13),
            (14, 14),
            (15, 15)
        );
    };
}

impl_deferred_table_for_tuple!((A, B), 2);
impl_deferred_table_for_tuple!((A, B, C), 3);
impl_deferred_table_for_tuple!((A, B, C, D), 4);
impl_deferred_table_for_tuple!((A, B, C, D, E), 5);
impl_deferred_table_for_tuple!((A, B, C, D, E, F), 6);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G), 7);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H), 8);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I), 9);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J), 10);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K), 11);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L), 12);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M), 13);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N), 14);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O), 15);
impl_deferred_table_for_tuple!((A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P), 16);
