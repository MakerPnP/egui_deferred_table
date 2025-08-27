use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, Context, CornerRadius, Id, Pos2, Rect, Response, Sense, StrokeKind, Style, Ui,
    UiBuilder, Vec2,
};
use indexmap::IndexMap;
use log::trace;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::ControlFlow;

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
            // TODO use a constant for this
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
        let ctx = ui.ctx();
        let style = ui.style();

        let mut actions = vec![];

        let cell_size: Vec2 = self.parameters.default_cell_size.unwrap_or(
            (
                (style.spacing.interact_size.x * 1.5) + (style.spacing.item_spacing.x * 2.0),
                style.spacing.interact_size.y + (style.spacing.item_spacing.y * 2.0),
            )
                .into(),
        );

        // XXX - remove this temporary hard-coded value
        // let cell_size: Vec2 = (50.0,25.0).into();

        let temp_state_id = self.id.with("temp_state");
        let mut temp_state = DeferredTableTempState::load_or_default(ctx, temp_state_id);

        let persistent_state_id = self.id.with("persistent_state");
        let mut state = DeferredTablePersistentState::load_or_default(ctx, persistent_state_id);

        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        // ensure there is a column width for each possible column
        if state.column_widths.len() < dimensions.column_count {
            // Note: We do not truncate the column widths, so that if a data source has `n` columns, then later `< n` columns
            //       then later again `>= n` columns, the previously used columns widths still apply.
            state
                .column_widths
                .resize(dimensions.column_count, cell_size.x);
        }

        // ensure there is a row height for each possible row
        if state.row_heights.len() < dimensions.row_count {
            // Note: We do not truncate the row heights, so that if a data source has `n` rows, then later `< n` rows
            //       then later again `>= n` rows, the previously used rows heights still apply.
            state.row_heights.resize(dimensions.row_count, cell_size.y);
        }

        // XXX - remove this temporary hard-coded value
        // state.column_widths[10] = 25.0;
        // state.column_widths[1] = 25.0;
        // state.column_widths[2] = 200.0;
        // state.column_widths[3] = 25.0;
        // state.column_widths[6] = 200.0;
        // state.column_widths[12] = 200.0;
        // state.row_heights[10] = 10.0;
        // state.row_heights[1] = 10.0;
        // state.row_heights[2] = 100.0;
        // state.row_heights[3] = 10.0;
        // state.row_heights[6] = 100.0;
        // state.row_heights[12] = 100.0;

        let mut source_state = SourceState { dimensions };

        let parent_max_rect = ui.max_rect();
        let parent_clip_rect = ui.clip_rect();

        // the x/y of this can have negative values if the OUTER scroll area is scrolled right or down, respectively.
        // i.e. if the outer scroll area scrolled down, the y will be negative, above the visible area.
        let outer_next_widget_position = ui.next_widget_position();
        trace!(
            "outer_next_widget_position: {:?}",
            outer_next_widget_position
        );

        // if there is content above the table, we use this min rect so we to define an area starting at the right place.
        let outer_min_rect =
            Rect::from_min_size(outer_next_widget_position, self.parameters.min_size.clone());
        // FIXME if the parent_max_rect is too small, min_size is not respected, but using
        //       ... `parent_max_rect.size().at_least(self.parameters.min_size)` causes rendering errors
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

            let inner_max_rect = ui.max_rect();

            let previous_cell_origin = temp_state.cell_origin;
            trace!("previous_cell_origin: {:?}", previous_cell_origin);

            let available_rows = source_state.dimensions.row_count;
            let available_columns = source_state.dimensions.column_count;
            trace!("available_rows: {}, available_columns: {}", available_rows, available_columns);

            let mut builder = DeferredTableBuilder::new(&mut source_state, data_source);

            build_table_view(&mut builder);


            let scroll_style = ui.spacing().scroll;

            //
            // container for the table and the scroll bars.
            //

            // add the width/height of the column/row headers to the sum of the column widths/row heights, respectively.
            let total_content_width = state.column_widths.iter().sum::<f32>() + cell_size.x;
            let total_content_height = state.row_heights.iter().sum::<f32>() + cell_size.y;

            let total_content_size = Vec2::new(
                total_content_width,
                total_content_height
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
                        //ui.painter().debug_rect(ui.max_rect(), Color32::RED, "mr");
                        trace!("viewport_rect: {:?}", viewport_rect);
                        let translated_viewport_rect = viewport_rect.translate(ui.max_rect().min.to_vec2());
                        if false {
                            ui.ctx().debug_painter().debug_rect(translated_viewport_rect, Color32::GREEN, "vr");
                        }

                        ui.set_height(total_content_size.y);
                        ui.set_width(total_content_size.x);

                        // FIXME the method for guessing does not take into account how the headers are rendered
                        //       in-place for smooth scrolling, a different approach should be used to properly
                        //       calculate the rectangle of the drawable cells should then be clipped to the
                        //       smooth scroll area below/right of the headers.
                        //       likely the calculation should create a new rect based on the viewport, minus the
                        //       row/column header areas and use that for calculations.

                        let (first_row_index, last_row_index_guess, first_row_top, last_row_bottom) =
                            get_visible_row_range(&state.row_heights, viewport_rect, cell_size);

                        let (first_column_index, last_column_index_guess, first_column_left, last_column_right) =
                            get_visible_column_range(&state.column_widths, viewport_rect, cell_size);

                        trace!("first_row_top: {}, last_row_bottom: {}, ", first_row_top, last_row_bottom);
                        trace!("first_column_left: {}, last_column_right: {}, ", first_column_left, last_column_right);

                        let y_min = ui.max_rect().top() + first_row_top;
                        let y_max = ui.max_rect().top() + last_row_bottom;
                        trace!("y_min: {}, y_max: {}", y_min, y_max);

                        let x_min = ui.max_rect().left() + first_column_left;
                        let x_max = ui.max_rect().left() + last_column_right;
                        trace!("x_min: {}, x_max: {}", x_min, x_max);

                        let rect = Rect::from_x_y_ranges(x_min..=x_max, y_min..=y_max);
                        trace!("rect: {:?}", rect);
                        if false {
                            ui.ctx().debug_painter().debug_rect(rect, Color32::CYAN, "table");
                        }

                        trace!("first_row_index: {}, last_row_index: {}", first_row_index, last_row_index_guess);
                        trace!("first_column_index: {}, last_column_index: {}, ", first_column_index, last_column_index_guess);

                        let cell_origin = CellIndex {
                            row: first_row_index,
                            column: first_column_index,
                        };
                        trace!("cell_origin: {:?}", cell_origin);
                        temp_state.cell_origin = cell_origin;

                        trace!("headers");
                        let mut accumulated_row_heights = 0.0;
                        let mut grid_row_index = 0;
                        let mut headers_done = false;
                        while !headers_done {
                            if grid_row_index + cell_origin.row > dimensions.row_count {
                                break
                            }

                            let row_number = grid_row_index + cell_origin.row;

                            let row_height = if grid_row_index > 0 {
                                state.row_heights[row_number - 1]
                            } else {
                                cell_size.y
                            };

                            let mut accumulated_column_widths = 0.0;

                            let mut grid_column_index = 0;
                            let mut first_on_row = true;
                            loop {
                                let column_number = grid_column_index + cell_origin.column;

                                if grid_row_index >= 1 && grid_column_index >= 1 {
                                    // no cell rendering
                                    break
                                }

                                if grid_column_index + cell_origin.column > dimensions.column_count {
                                    break
                                }

                                let start_pos = if grid_column_index > 0 || grid_row_index > 0 {
                                    rect.min
                                } else {
                                    table_max_rect.min
                                };

                                let column_width = if grid_column_index > 0 {
                                    let column_width = state.column_widths[column_number - 1];
                                    trace!("column width from state: {}", column_width);
                                    column_width
                                } else {
                                    let column_width = cell_size.x;
                                    trace!("column width from cell size: {}", column_width);
                                    cell_size.x
                                };

                                let mut y = start_pos.y + accumulated_row_heights;
                                let mut x = start_pos.x + accumulated_column_widths;
                                accumulated_column_widths += column_width;

                                if grid_row_index == 0 {
                                    y = table_max_rect.min.y;
                                }
                                if grid_column_index == 0 {
                                    x = table_max_rect.min.x;
                                }

                                let cell_rect = Rect::from_min_size(Pos2::new(x, y), (column_width, row_height).into());

                                let mut cell_clip_rect = cell_rect.intersect(translated_viewport_rect);

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

                                let cell_clip_rect_size = cell_clip_rect.size();
                                trace!("grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect_size);

                                if cell_clip_rect_size.x < 0.0 || cell_clip_rect_size.y < 0.0 {
                                    trace!("skipping non-visible/zero-sized");

                                    // if this is the first column, then none of this row can be rendered, so we're done
                                    if first_on_row {
                                        headers_done = true;
                                    }
                                    break;
                                }

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
                                    cell_ui.label(format!("{}*{} ({},{})", dimensions.column_count, dimensions.row_count, cell_origin.column, cell_origin.row));
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
                                grid_column_index += 1;
                                first_on_row = false;
                            }
                            grid_row_index += 1;
                            accumulated_row_heights += row_height;
                        }

                        trace!("cells");

                        let clip_rect = Rect::from_min_size(table_max_rect.min + cell_size, rect.size()).intersect(table_max_rect);
                        if false {
                            ui.painter().debug_rect(clip_rect, Color32::CYAN, "cr");
                        }

                        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                            ui.set_clip_rect(translated_viewport_rect);
                            // ui.skip_ahead_auto_ids(???); // TODO Make sure we get consistent IDs.

                            let table_max_rect = ui.max_rect();

                            //
                            // display the table
                            //

                            let start_pos = table_max_rect.min;

                            // start with an offset equal to header height, which is currently using the cell_size
                            let mut accumulated_row_heights = cell_size.y;
                            let mut cells_done = false;
                            let mut grid_row_index = 1;
                            while !cells_done {
                                if grid_row_index + cell_origin.row > dimensions.row_count {
                                    break
                                }

                                let row_number = grid_row_index + cell_origin.row;

                                let row_height = state.row_heights[row_number - 1];

                                let y = start_pos.y + accumulated_row_heights;

                                // start with an offset equal to header width, which is currently using the cell_size
                                let mut accumulated_column_widths = cell_size.x;

                                let mut grid_column_index = 1;
                                let mut first_on_row = true;
                                loop {
                                    if grid_column_index + cell_origin.column > dimensions.column_count {
                                        break
                                    }

                                    let row = cell_origin.row + (grid_row_index - 1);
                                    let column = cell_origin.column + (grid_column_index - 1);

                                    let column_width = state.column_widths[column];

                                    let cell_index = CellIndex {
                                        row,
                                        column,
                                    };

                                    let x = start_pos.x + accumulated_column_widths;
                                    accumulated_column_widths += column_width;

                                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), (column_width, row_height).into());
                                    let cell_clip_rect = cell_rect.intersect(clip_rect).intersect(translated_viewport_rect);
                                    let cell_clip_rect_size = cell_clip_rect.size();
                                    trace!("grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect_size);

                                    if cell_clip_rect_size.x < 0.0 || cell_clip_rect_size.y < 0.0 {
                                        trace!("skipping non-visible/zero-sized");

                                        // if this is the first column, then none of this row can be rendered, so we're done
                                        if first_on_row {
                                            cells_done = true;
                                        }
                                        break;
                                    }

                                    let response = ui.allocate_rect(cell_clip_rect, Sense::click());

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
                                        actions.push(Action::CellClicked(cell_index));
                                    }


                                    ui.painter()
                                        .with_clip_rect(cell_clip_rect)
                                        .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);

                                    let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(cell_rect));
                                    cell_ui.set_clip_rect(cell_clip_rect);
                                    cell_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                                    data_source.render_cell(&mut cell_ui, cell_index);
                                    grid_column_index += 1;
                                    first_on_row = false;
                                }
                                grid_row_index += 1;
                                accumulated_row_heights += row_height;
                            }
                        });
                    });
            });
        });

        DeferredTablePersistentState::store(ui.ctx(), persistent_state_id, state);
        DeferredTableTempState::store(ui.ctx(), temp_state_id, temp_state);

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

fn get_visible_row_range(
    row_heights: &[f32],
    viewport_rect: Rect,
    cell_size: Vec2,
) -> (usize, usize, f32, f32) {
    let mut first_row_top = 0.0;
    let mut first_row_bottom = 0.0;
    let mut first_row_index = 0;
    loop {
        let Some(row_height) = row_heights.get(first_row_index) else {
            break;
        };
        first_row_bottom += row_height;
        if first_row_bottom > viewport_rect.min.y {
            break;
        }
        first_row_index += 1;
        first_row_top += row_height;
    }

    let mut last_row_top = first_row_top;
    let mut last_row_bottom = first_row_bottom;
    let mut last_row_index_guess = first_row_index;
    loop {
        let Some(row_height) = row_heights.get(last_row_index_guess) else {
            break;
        };
        if last_row_bottom > viewport_rect.max.y {
            break;
        }
        last_row_index_guess += 1;
        last_row_top += row_height;
        last_row_bottom += row_height;
    }

    (
        first_row_index,
        last_row_index_guess,
        first_row_top,
        last_row_bottom,
    )
}

fn get_visible_column_range(
    column_widths: &[f32],
    viewport_rect: Rect,
    cell_size: Vec2,
) -> (usize, usize, f32, f32) {
    let mut first_column_left = 0.0;
    let mut first_column_right = 0.0;
    let mut first_column_index = 0;
    loop {
        let Some(column_width) = column_widths.get(first_column_index) else {
            break;
        };
        first_column_right += column_width;
        if first_column_right > viewport_rect.min.x {
            break;
        }
        first_column_index += 1;
        first_column_left += column_width;
    }

    let mut last_column_left = first_column_left;
    let mut last_column_right = first_column_right;
    let mut last_column_index_guess = first_column_index;
    loop {
        let Some(&column_width) = column_widths.get(last_column_index_guess) else {
            break;
        };
        if last_column_right > viewport_rect.max.x {
            break;
        }
        last_column_index_guess += 1;
        last_column_left += column_width;
        last_column_right += column_width;
    }

    (
        first_column_index,
        last_column_index_guess,
        first_column_left,
        last_column_right,
    )
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

/// State that could be stored between application restarts
#[derive(Default, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct DeferredTablePersistentState {
    // TODO column ordering
    // TODO column visibility
    // TODO cursor/focus position
    // TODO cell selection (multi-select)
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
}

impl DeferredTablePersistentState {
    pub fn load_or_default(ctx: &Context, id: Id) -> Self {
        ctx.data_mut(|d| {
            d.get_persisted::<DeferredTablePersistentState>(id)
                .unwrap_or(DeferredTablePersistentState::default())
        })
    }

    pub fn store(ctx: &Context, id: Id, instance: Self) {
        ctx.data_mut(|d| d.insert_persisted(id, instance));
    }
}

/// State that should not be persisted between application restarts
#[derive(Default, Clone)]
struct DeferredTableTempState {
    /// holds the index of the top-left cell
    cell_origin: CellIndex,
}

impl DeferredTableTempState {
    pub fn load_or_default(ctx: &Context, id: Id) -> Self {
        ctx.data_mut(|d| {
            d.get_temp::<DeferredTableTempState>(id)
                .unwrap_or(DeferredTableTempState::default())
        })
    }

    pub fn store(ctx: &Context, id: Id, instance: Self) {
        ctx.data_mut(|d| d.insert_temp(id, instance));
    }
}

pub trait DeferredTableDataSource {
    fn get_dimensions(&self) -> TableDimensions;
}

pub trait DeferredTableRenderer {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex);
}

pub struct DeferredTableBuilder<'a, DataSource> {
    table: Table,

    source_state: &'a mut SourceState,

    data_source: &'a DataSource,
}

impl<'a, DataSource> DeferredTableBuilder<'a, DataSource> {
    pub fn header(&mut self, builder_header_view: fn(&'_ mut HeaderBuilder<'_, DataSource>)) {
        let mut header_builder =
            HeaderBuilder::new(&mut self.table, &mut self.source_state, self.data_source);

        builder_header_view(&mut header_builder);
    }
}

struct Table {
    columns: IndexMap<usize, String>,
    // TODO column groups here..
}

impl<'a, DataSource> DeferredTableBuilder<'a, DataSource> {
    fn new(source_state: &'a mut SourceState, data_source: &'a DataSource) -> Self
    where
        DataSource: DeferredTableDataSource + DeferredTableRenderer,
    {
        let table = Table {
            columns: IndexMap::new(),
        };

        Self {
            table,
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
    source_state: &'a mut SourceState,
    data_source: &'a DataSource,
}

impl<'a, DataSource> HeaderBuilder<'a, DataSource> {
    fn new(
        table: &'a mut Table,
        source_state: &'a mut SourceState,
        data_source: &'a DataSource,
    ) -> Self {
        Self {
            table,
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
