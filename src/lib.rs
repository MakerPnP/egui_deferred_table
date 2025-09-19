use egui::emath::GuiRounding;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, Context, CornerRadius, Id, NumExt, Painter, PointerButton, PopupAnchor, Pos2, Rangef,
    Rect, Response, RichText, Sense, StrokeKind, Style, Tooltip, Ui, UiBuilder, Vec2,
};
use log::{info, trace};
use std::marker::PhantomData;
use std::ops::Range;

const SHOW_HEADER_CELL_BORDERS: bool = false;
const SHOW_CELL_BORDERS: bool = false;

pub struct DeferredTable<'a, DataSource> {
    id: Id,
    parameters: DeferredTableParameters<'a>,
    phantom_data: PhantomData<DataSource>,
}

struct DeferredTableParameters<'a> {
    default_cell_size: Option<Vec2>,
    zero_based_headers: bool,
    highlight_hovered_cell: bool,
    min_size: Vec2,
    column_parameters: Option<&'a Vec<AxisParameters>>,
    row_parameters: Option<&'a Vec<AxisParameters>>,
}

impl<'a> Default for DeferredTableParameters<'a> {
    fn default() -> Self {
        Self {
            default_cell_size: None,
            zero_based_headers: false,
            highlight_hovered_cell: false,
            // TODO use a constant for this
            min_size: Vec2::new(400.0, 200.0),
            column_parameters: None,
            row_parameters: None,
        }
    }
}

impl<'a, DataSource> DeferredTable<'a, DataSource> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            parameters: DeferredTableParameters::default(),
            phantom_data: PhantomData,
        }
    }

    /// this currently also controls the row/column header and corner sizes
    pub fn default_cell_size(mut self, size: Vec2) -> Self {
        self.parameters.default_cell_size = Some(size);
        self
    }

    /// default: disabled
    pub fn zero_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = true;
        self
    }

    /// default: enabled
    pub fn one_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = false;
        self
    }

    /// default: disabled
    pub fn highlight_hovered_cell(mut self) -> Self {
        self.parameters.highlight_hovered_cell = true;
        self
    }

    /// default: 400x200
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.parameters.min_size = size;
        self
    }

    pub fn column_parameters(mut self, column_parameters: &'a Vec<AxisParameters>) -> Self {
        self.parameters.column_parameters = Some(column_parameters);
        self
    }

    pub fn row_parameters(mut self, row_parameters: &'a Vec<AxisParameters>) -> Self {
        self.parameters.row_parameters = Some(row_parameters);
        self
    }

    pub fn show<Renderer>(
        &self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
    {
        data_source.prepare();
        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        let result = if !dimensions.is_empty() {
            self.show_inner(ui, data_source, renderer, dimensions)
        } else {
            (ui.response(), vec![])
        };

        data_source.finalize();

        result
    }

    fn outer_size(cell_size: Vec2, style: &Style) -> Vec2 {
        cell_size + style.spacing.item_spacing
    }

    /// Safety: only call if the dimensions are non-empty
    fn show_inner<Renderer>(
        &self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
        dimensions: TableDimensions,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
    {
        let ctx = ui.ctx().clone();
        let style = ui.style();
        let pixels_per_point = ctx.pixels_per_point();

        let mut actions = vec![];

        let inner_cell_size: Vec2 = self.parameters.default_cell_size.unwrap_or(Vec2::new(
            style.spacing.interact_size.x * 1.5,
            style.spacing.interact_size.y,
        ));

        // XXX - remove this temporary hard-coded value
        // let inner_cell_size: Vec2 = (50.0, 25.0).into();

        let outer_cell_size = Self::outer_size(inner_cell_size, style);

        // FIXME if the column/row is too narrow/short then the hover/drag isn't detected, even though it's visible.
        //       to replicate, set 3 columns/rows to their minimum width/heights and then try resizing the middle one.
        //       as a workaround we clamp the minimum column/row width/heights to this.
        let minimum_resize_size = (style.interaction.resize_grab_radius_side * 2.0) + 2.0;

        let mut clear_drag_state = false;

        // TODO allow these to be overridden
        let default_column_parameters = AxisParameters::default();
        let default_row_parameters = AxisParameters::default();

        enum DragAction {
            SetWidth(usize, f32),
            SetHeight(usize, f32),
        }
        let mut drag_action = None;

        let pointer_pos = ui.ctx().pointer_latest_pos();

        let temp_state_id = self.id.with("temp_state");
        let mut temp_state = DeferredTableTempState::load_or_default(&ctx, temp_state_id);

        let persistent_state_id = self.id.with("persistent_state");
        let mut state = DeferredTablePersistentState::load_or_default(&ctx, persistent_state_id);

        trace!("dimensions: {:?}", dimensions);

        let parent_max_rect = ui.max_rect();
        let parent_clip_rect = ui.clip_rect();
        let ui_layer_id = ui.layer_id();

        // the x/y of this can have negative values if the OUTER scroll area is scrolled right or down, respectively.
        // i.e. if the outer scroll area scrolled down, the y will be negative, above the visible area.
        let outer_next_widget_position = ui.next_widget_position();
        trace!(
            "outer_next_widget_position: {:?}",
            outer_next_widget_position
        );

        // CRITICAL - we *must* round to pixels, otherwise we get out-by-one pixel errors when rendering lines

        // if there is content above the table, we use this min rect so we to define an area starting at the right place.
        let outer_min_rect =
            Rect::from_min_size(outer_next_widget_position, self.parameters.min_size.clone())
                .round_to_pixels(pixels_per_point);
        // FIXME if the parent_max_rect is too small, min_size is not respected, but using
        //       ... `parent_max_rect.size().at_least(self.parameters.min_size)` causes rendering errors
        let outer_max_rect =
            Rect::from_min_size(outer_next_widget_position, parent_max_rect.size())
                .round_to_pixels(pixels_per_point);

        trace!(
            "outer_min_rect: {:?}, outer_max_rect: {:?}",
            outer_min_rect, outer_max_rect
        );

        if false {
            ui.painter()
                .debug_rect(outer_min_rect, Color32::GREEN, "omnr");
            ui.painter()
                .debug_rect(outer_max_rect, Color32::RED, "omxr");
        }

        ui.scope_builder(UiBuilder::new().max_rect(outer_max_rect), |ui|{

            ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();

            let inner_max_rect = ui.max_rect();
            // FUTURE since these are the same, we can clean-up one or the other...
            debug_assert_eq!(inner_max_rect, outer_max_rect);

            let previous_cell_origin = temp_state.cell_origin;
            trace!("previous_cell_origin: {:?}", previous_cell_origin);

            // ensure there is a column width for each possible column
            if state.column_widths.len() < dimensions.column_count {
                // Note: We do not truncate the column widths, so that if a data source has `n` columns, then later `< n` columns
                //       then later again `>= n` columns, the previously used columns widths still apply.
                state.column_widths.resize(dimensions.column_count, inner_cell_size.x);

                // apply default widths
                if let Some(column_parameters) = self.parameters.column_parameters {
                    column_parameters.iter().enumerate().for_each(|(index, column)| {
                        if let Some(default_width) = column.default_dimension {
                            let sanitized_width = if column.resizable {
                                column.dimension_range.clamp(default_width)
                            } else {
                                default_width
                            };
                            state.column_widths[index] = sanitized_width;
                        }
                    });
                }
            }

            // ensure there is a row height for each possible row
            if state.row_heights.len() < dimensions.row_count {
                // Note: We do not truncate the row heights, so that if a data source has `n` rows, then later `< n` rows
                //       then later again `>= n` rows, the previously used rows heights still apply.
                state.row_heights.resize(dimensions.row_count, inner_cell_size.y);

                // apply default heights
                if let Some(row_parameters) = self.parameters.row_parameters {
                    row_parameters.iter().enumerate().for_each(|(index, row)| {
                        if let Some(default_height) = row.default_dimension {
                            let sanitized_width = if row.resizable {
                                row.dimension_range.clamp(default_height)
                            } else {
                                default_height
                            };
                            state.row_heights[index] = sanitized_width;
                        }
                    });
                }
            }

            // XXX - remove this temporary hard-coded value
            // //state.column_widths[10] = 25.0;
            // state.column_widths[1] = 25.0;
            // state.column_widths[2] = 200.0;
            // state.column_widths[3] = 25.0;
            // state.column_widths[6] = 200.0;
            // state.column_widths[12] = 200.0;
            // // state.row_heights[10] = 10.0;
            // state.row_heights[1] = 10.0;
            // state.row_heights[2] = 100.0;
            // state.row_heights[3] = 10.0;
            // state.row_heights[6] = 100.0;
            // state.row_heights[12] = 100.0;

            let scroll_style = ui.spacing().scroll;

            //
            // container for the table and the scroll bars.
            //

            let column_ordering = renderer.column_ordering().unwrap_or_default();
            let row_ordering = renderer.row_ordering().unwrap_or_default();

            let outer_inner_difference = outer_cell_size - inner_cell_size;
            // pre-calculate to avoid doing the divide for every cell.
            let outer_inner_half_difference = outer_inner_difference / 2.0;

            // add the width/height of the column/row headers to the sum of the column widths/row heights, respectively.
            let total_content_width = state.column_widths.iter().sum::<f32>() + ((outer_inner_difference.x + 1.0) * dimensions.column_count as f32) + outer_cell_size.x;
            let total_content_height = state.row_heights.iter().sum::<f32>() + ((outer_inner_difference.y + 1.0) * dimensions.row_count as f32) + outer_cell_size.y;

            let columns_to_filter = renderer.columns_to_filter();
            let filtered_content_width = columns_to_filter.map_or(0.0,|columns|{
                columns.iter().map(|index| {
                    let mapped_index = Self::map_index(dimensions.column_count, column_ordering, *index);
                    state.column_widths.get(mapped_index).map(|it|it + outer_inner_difference.x + 1.0).unwrap_or(0.0)
                }).sum::<f32>()
            });

            let rows_to_filter = renderer.rows_to_filter();
            let filtered_content_height = rows_to_filter.map_or(0.0,|rows|{
                rows.iter().map(|index| {
                    let mapped_index = Self::map_index(dimensions.column_count, column_ordering, *index);
                    state.row_heights.get(mapped_index).map(|it|it + outer_inner_difference.y + 1.0).unwrap_or(0.0)
                }).sum::<f32>()
            });

            let total_content_size = Vec2::new(
                total_content_width - filtered_content_width,
                total_content_height - filtered_content_height,
            );
            trace!("total_content_size: {:?}, filtered_content_width: {}, filtered_content_height: {}", total_content_size, filtered_content_width, filtered_content_height);

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
                        trace!("max_rect: {:?}, viewport_rect: {:?}", ui.max_rect(), viewport_rect);
                        //ui.painter().debug_rect(ui.max_rect(), Color32::RED, "mr");
                        let translated_viewport_rect = viewport_rect.translate(ui.max_rect().min.to_vec2());
                        let cells_viewport_rect = Rect::from_min_max(viewport_rect.min, viewport_rect.max - outer_cell_size);
                        if false {
                            ui.ctx().debug_painter().debug_rect(translated_viewport_rect, Color32::GREEN, "vr");
                            ui.ctx().debug_painter().debug_rect(cells_viewport_rect.translate(ui.max_rect().min.to_vec2()).translate(outer_cell_size), Color32::RED, "tvr");
                        }

                        ui.set_height(total_content_size.y);
                        ui.set_width(total_content_size.x);

                        fn range_and_index_for_offset(offset: f32, values: &[f32], map: &[usize], filter: &Option<&[usize]>, sizing: f32) -> Result<(Range<f32>, usize, usize, usize), ()> {
                            let mut visible_index = 0;
                            let mut min = 0.0;
                            let mut max = 0.0;
                            let mut filtered = 0;
                            let mut index ;
                            let values_len = values.len();
                            loop {
                                index = *map.get(visible_index).unwrap_or(&visible_index);
                                if index >= values_len {
                                    // handle out-of-range mapping values
                                    index = visible_index;
                                }

                                let Some(value) = values.get(index) else {
                                    if visible_index == 0 {
                                        // no values at all
                                        return Err(())
                                    }
                                    // no more values, use previous loop iteration values
                                    break
                                };

                                // filter applies AFTER mapping
                                if let Some(filter) = filter {
                                    if filter.contains(&index) {
                                        visible_index += 1;
                                        filtered += 1;
                                        continue;
                                    }
                                }

                                let size = value + sizing;
                                max += size;

                                if offset >= min && offset < max {
                                    break
                                }

                                min += size;
                                visible_index += 1;
                            }

                            Ok((min..max, index, visible_index, filtered))
                        }

                        // use the cells_viewport_rect for upper left and origin calculation
                        let (first_column, first_column_index, first_column_visible_index, first_column_filtered_count) = range_and_index_for_offset(cells_viewport_rect.min.x, &state.column_widths, &column_ordering, &columns_to_filter, outer_inner_difference.x + 1.0).unwrap();
                        let (first_row, first_row_index, first_row_visible_index, first_row_filtered_count) = range_and_index_for_offset(cells_viewport_rect.min.y, &state.row_heights, &row_ordering, &rows_to_filter, outer_inner_difference.y + 1.0).unwrap();

                        // use the total viewport (including header area) to find the last column and row
                        let (last_column, _last_column_index, last_column_visible_index, last_column_filtered_count) = range_and_index_for_offset(viewport_rect.max.x, &state.column_widths, &column_ordering, &columns_to_filter, outer_inner_difference.x + 1.0).unwrap();
                        let (last_row, _last_row_index, last_row_visible_index, last_row_filtered_count) = range_and_index_for_offset(viewport_rect.max.y, &state.row_heights, &row_ordering, &rows_to_filter, outer_inner_difference.y + 1.0).unwrap();

                        // note, if the scroll area doesn't line up exactly with the viewport, then we may have to render additional rows/columns that
                        // are outside of this rect
                        let rect = Rect::from_min_max((first_column.start, first_row.start).into(), (last_column.end, last_row.end).into())
                            .translate(ui.max_rect().min.to_vec2());

                        trace!("rect: {:?}", rect);
                        if false {
                            ui.ctx().debug_painter().debug_rect(rect, Color32::CYAN, "rect");
                        }

                        trace!("first_column_index: {}, first_column_index: {}, first_column_visible_index: {}", first_column_index, first_column_index, first_column_visible_index);
                        trace!("first_row_index: {}, first_row_index: {}, first_row_visible_index: {}", first_row_index, first_row_index, first_row_visible_index);

                        let cell_origin = CellIndex {
                            row: first_row_visible_index,
                            column: first_column_visible_index,
                        };
                        trace!("cell_origin: {:?}", cell_origin);
                        temp_state.cell_origin = cell_origin;

                        let visible_row_count = last_row_visible_index - first_row_visible_index + 1 + last_row_filtered_count;
                        let visible_column_count = last_column_visible_index - first_column_visible_index + 1 + last_column_filtered_count;
                        trace!("visible_row_count: {}, visible_column_count: {}", visible_row_count, visible_column_count);
                        trace!("first_column_filtered_count: {}, last_column_filtered_count: {}", first_column_filtered_count, last_column_filtered_count);
                        trace!("first_row_filtered_count: {}, last_row_filtered_count: {}", first_row_filtered_count, last_row_filtered_count);

                        let mut table_width = 0.0;
                        let mut table_height = 0.0;

                        let mut row_counter = cell_origin.row - first_row_filtered_count;

                        trace!("headers");
                        let header_row_bg_color = ui.style().visuals.widgets.inactive.bg_fill.gamma_multiply(0.5);
                        let mut accumulated_row_heights = 0.0;
                        for grid_row_index in 0..=visible_row_count {
                            if grid_row_index + cell_origin.row > dimensions.row_count {
                                trace!("break 1");
                                break
                            }

                            let visible_row_index = cell_origin.row + (grid_row_index.saturating_sub(1));
                            let mapped_row_index = Self::map_index(dimensions.row_count, row_ordering, visible_row_index);

                            let row_kind = Self::build_row_kind(grid_row_index);

                            if matches!(row_kind, RowKind::ValuesRow) {
                                if let Some(rows_to_filter) = &rows_to_filter {
                                    if rows_to_filter.contains(&(mapped_row_index)) {
                                        trace!("filtered row");
                                        continue;
                                    }
                                }
                            }
                            row_counter += 1;

                            let row_bg_color = striped_row_color(row_counter, &ui.style()).unwrap_or(ui.style().visuals.widgets.noninteractive.weak_bg_fill);

                            let inner_row_height = match row_kind {
                                RowKind::ValuesRow => *state.row_heights.get(mapped_row_index).unwrap_or(&inner_cell_size.y),
                                RowKind::HeaderRow => inner_cell_size.y,
                            };
                            let outer_row_height = inner_row_height + outer_inner_difference.y;

                            let mut accumulated_column_widths = 0.0;

                            for grid_column_index in 0..=visible_column_count {
                                if grid_column_index + cell_origin.column > dimensions.column_count {
                                    break
                                }

                                let cell_kind = Self::build_cell_kind(grid_row_index, grid_column_index);

                                if matches!(cell_kind, CellKind::Value) {
                                    // no cell rendering during header rendering
                                    // we're just rendering the top and left headers
                                    break
                                }

                                let visible_column_index = cell_origin.column + (grid_column_index.saturating_sub(1));
                                let mapped_column_index = Self::map_index(dimensions.column_count, column_ordering, visible_column_index);

                                if matches!(cell_kind, CellKind::ColumnHeader) {
                                    if let Some(columns_to_filter) = &columns_to_filter {
                                        if columns_to_filter.contains(&mapped_column_index) {
                                            trace!("filtered column");
                                            continue;
                                        }
                                    }
                                }

                                let start_pos = match cell_kind {
                                    // for smooth scrolling, we position the cell using rect.min, then later we clip the left/top of the partial cell
                                    CellKind::ColumnHeader | CellKind::RowHeader => rect.min,
                                    // for the corner we fix the cell use the top/left
                                    CellKind::Corner => table_max_rect.min,
                                    _ => unreachable!()
                                };

                                let inner_column_width = if matches!(cell_kind, CellKind::ColumnHeader) {
                                    *state.column_widths.get(mapped_column_index).unwrap_or(&inner_cell_size.x)
                                } else {
                                    inner_cell_size.x
                                };
                                let outer_column_width = inner_column_width + outer_inner_difference.x;

                                let mut y = start_pos.y + accumulated_row_heights;
                                let mut x = start_pos.x + accumulated_column_widths;
                                accumulated_column_widths += outer_column_width + 1.0;

                                if matches!(cell_kind, CellKind::Corner | CellKind::ColumnHeader) {
                                    y = table_max_rect.min.y;
                                }
                                if matches!(cell_kind, CellKind::Corner | CellKind::RowHeader) {
                                    x = table_max_rect.min.x;
                                }

                                let cell_rect = Rect::from_min_size(Pos2::new(x, y), (outer_column_width, outer_row_height).into());

                                let mut cell_clip_rect = cell_rect.intersect(translated_viewport_rect);

                                if grid_row_index == 1 {
                                    cell_clip_rect.min.y = table_max_rect.min.y + outer_cell_size.y + 1.0;
                                }
                                if grid_column_index == 1 {
                                    cell_clip_rect.min.x = table_max_rect.min.x + outer_cell_size.x + 1.0;
                                }
                                let cell_clip_rect = cell_clip_rect.intersect(parent_clip_rect);

                                let cell_inner_rect = cell_rect.shrink2(outer_inner_half_difference);
                                let cell_inner_clip_rect = cell_inner_rect.intersect(cell_clip_rect);

                                if false {
                                    ui.painter().debug_rect(cell_clip_rect, Color32::ORANGE, "ccr");
                                }

                                let cell_clip_rect_size = cell_clip_rect.size();
                                let skip = cell_clip_rect_size.x < 0.0 || cell_clip_rect_size.y < 0.0;

                                trace!("grid: i=[{},{}] v=[{},{}], m=[{},{}], cell_rect: {:?}, cell_clip_rect: {:?}, pos: {:?}, size: {:?}, skip: {}",
                                    grid_row_index, grid_column_index,
                                    visible_row_index, visible_column_index,
                                    mapped_row_index, mapped_column_index,
                                    cell_rect, cell_clip_rect, cell_clip_rect.min, cell_clip_rect_size, skip);

                                if skip {
                                    continue;
                                }

                                let bg_color = if grid_row_index == 0 {
                                    header_row_bg_color
                                } else {
                                    row_bg_color
                                };

                                let cell_painter = ui.painter()
                                    .with_clip_rect(cell_clip_rect);

                                cell_painter
                                    .rect_filled(cell_rect, 0.0, bg_color);

                                if SHOW_HEADER_CELL_BORDERS {
                                    cell_painter
                                        .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);
                                }

                                let resize_painter = ui.painter()
                                    .with_clip_rect(parent_clip_rect);

                                let mut drag_tooltip_message = None;

                                if matches!(cell_kind, CellKind::ColumnHeader) {
                                    let column_parameters = self.parameters.column_parameters
                                        .map(|it|it.get(mapped_column_index))
                                        .flatten()
                                        .unwrap_or_else(|| {
                                        &default_column_parameters
                                    });

                                    let column_resize_id = ui.id().with("resize_column").with(mapped_column_index);

                                    let resize_line_points = [cell_rect.right_top(), cell_rect.right_bottom()];
                                    let resize_interact_rect = Rect::from(resize_line_points)
                                        .expand2(Vec2::new(ui.style().interaction.resize_grab_radius_side, 0.0));

                                    if false {
                                        ui.painter().debug_rect(resize_interact_rect, Color32::MAGENTA, "r");
                                    }

                                    let resize_response =
                                        ui.interact(resize_interact_rect, column_resize_id, egui::Sense::click_and_drag());

                                    let mut drag_handle_state = if resize_response.hovered() {
                                        if !column_parameters.resizable {
                                            DragHandleState::Disabled
                                        } else {
                                            DragHandleState::Hovered
                                        }
                                    } else {
                                        DragHandleState::Inactive
                                    };

                                    if column_parameters.resizable {
                                        if resize_response.drag_started_by(PointerButton::Primary) && temp_state.drag_state.is_none() {
                                            temp_state.drag_state = pointer_pos.map(|start_pos| DragState { index: mapped_column_index, start_pos, cell_kind: cell_kind, initial_size: outer_column_width });
                                        }

                                        if resize_response.drag_stopped() {
                                            clear_drag_state = true;
                                        }

                                        match temp_state.drag_state {
                                            Some(DragState { index, start_pos, cell_kind: drag_cell_kind, initial_size }) if index == mapped_column_index && drag_cell_kind == cell_kind => {
                                                // dragging this column
                                                let drag_delta = pointer_pos.map_or(Vec2::ZERO, |current_pos| current_pos - start_pos);
                                                let new_outer_column_width = initial_size + drag_delta.x;
                                                let new_inner_column_width = new_outer_column_width - outer_inner_difference.x;

                                                let sanitized_column_width = column_parameters.dimension_range.clamp(new_inner_column_width);

                                                let new_column_width = sanitized_column_width.at_least(minimum_resize_size);

                                                if new_column_width != inner_column_width {
                                                    // change at the end of the frame to avoid cells being the old size.
                                                    drag_action = Some(DragAction::SetWidth(mapped_column_index, new_column_width));
                                                }
                                                drag_tooltip_message = Some(format!("{}", new_column_width));

                                                drag_handle_state = DragHandleState::Dragged;
                                            }
                                            _ => {}
                                        };
                                    }

                                    Self::paint_resize_handle(ui, resize_line_points, drag_handle_state, &resize_painter, cell_kind);
                                }

                                if matches!(cell_kind, CellKind::RowHeader) {
                                    let row_resize_id = ui.id().with("resize_row").with(grid_row_index);

                                    let resize_line_points = [cell_rect.left_bottom(), cell_rect.right_bottom()];
                                    let resize_interact_rect = Rect::from(resize_line_points)
                                        .expand2(Vec2::new(0.0, ui.style().interaction.resize_grab_radius_side));

                                    let resize_response =
                                        ui.interact(resize_interact_rect, row_resize_id, egui::Sense::click_and_drag());

                                    let mut drag_handle_state = if resize_response.hovered() {
                                        DragHandleState::Hovered
                                    } else {
                                        DragHandleState::Inactive
                                    };

                                    if resize_response.drag_started_by(PointerButton::Primary) && temp_state.drag_state.is_none() {
                                        temp_state.drag_state = pointer_pos.map(|start_pos|DragState { index: mapped_row_index, start_pos, cell_kind: cell_kind, initial_size: outer_row_height });
                                    }

                                    if resize_response.drag_stopped() {
                                        clear_drag_state = true;
                                    }

                                    match temp_state.drag_state {
                                        Some(DragState { index, start_pos, cell_kind: drag_cell_kind, initial_size }) if index == mapped_row_index && drag_cell_kind == cell_kind => {
                                            // dragging this row
                                            let drag_delta = pointer_pos.map_or(Vec2::ZERO, |current_pos| current_pos - start_pos);
                                            let new_outer_row_height = initial_size + drag_delta.y;
                                            let new_inner_row_height = new_outer_row_height - outer_inner_difference.y;
                                            let new_row_height = Rangef::new(minimum_resize_size, f32::INFINITY).clamp(new_inner_row_height);

                                            if new_row_height != inner_row_height {
                                                // change at the end of the frame to avoid cells being the old size.
                                                drag_action = Some(DragAction::SetHeight(mapped_row_index, new_row_height));
                                            }
                                            drag_tooltip_message = Some(format!("{}", new_row_height));

                                            drag_handle_state = DragHandleState::Dragged;
                                        }
                                        _ => { }
                                    }

                                    Self::paint_resize_handle(ui, resize_line_points, drag_handle_state, &resize_painter, cell_kind);
                                }

                                if let Some(message) = drag_tooltip_message {
                                    Tooltip::always_open(ctx.clone(), ui_layer_id, "_egui_deferred_table_resize_".into(), PopupAnchor::Pointer)
                                        .gap(12.0)
                                        .show(|ui|{
                                            ui.horizontal(|ui|{
                                                ui.label(message);
                                            });
                                        });
                                }

                                let response = ui.allocate_rect(cell_clip_rect, Sense::click_and_drag());

                                struct DndPayload {
                                    cell_kind: CellKind,
                                    index: usize,
                                }

                                let payload = match cell_kind {
                                    CellKind::ColumnHeader => {
                                        Some(DndPayload { cell_kind, index: mapped_column_index })
                                    }
                                    CellKind::RowHeader => {
                                        Some(DndPayload { cell_kind, index: mapped_row_index })
                                    }
                                    _ => None
                                };

                                if let Some(payload) = payload {
                                    response.dnd_set_drag_payload(payload);
                                }

                                let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(cell_inner_rect));
                                cell_ui.set_clip_rect(cell_inner_clip_rect);
                                let style = cell_ui.style_mut();
                                style.wrap_mode = Some(egui::TextWrapMode::Extend);

                                let mut monospace = false;
                                let label = match cell_kind {
                                    CellKind::Corner => {
                                        Some(format!("{}*{} ({},{})", dimensions.column_count, dimensions.row_count, cell_origin.column, cell_origin.row))
                                    }
                                    CellKind::ColumnHeader => {
                                        monospace = default_column_parameters.monospace;

                                        if let Some(column_parameters) = self.parameters
                                            .column_parameters
                                            .map(|it| it.get(mapped_column_index))
                                            .flatten()
                                        {
                                            monospace = column_parameters.monospace;
                                            column_parameters.name.clone()
                                        } else if self.parameters.zero_based_headers {
                                            Some(mapped_column_index.to_string())
                                        } else {
                                            let mapped_column_number = mapped_column_index + 1;
                                            Some(mapped_column_number.to_string())
                                        }
                                    }
                                    CellKind::RowHeader => {
                                        monospace = default_row_parameters.monospace;
                                        if let Some(row_parameters) = self.parameters
                                            .row_parameters
                                            .map(|it| it.get(mapped_row_index))
                                            .flatten()
                                        {
                                            monospace = row_parameters.monospace;
                                            row_parameters.name.clone()
                                        } else if self.parameters.zero_based_headers {
                                            Some(mapped_row_index.to_string())
                                        } else {
                                            let mapped_row_number = mapped_row_index + 1;
                                            Some(mapped_row_number.to_string())
                                        }
                                    },
                                    CellKind::Value => {
                                        // already filtered out
                                        unreachable!()
                                    }
                                };

                                if let Some(label) = &label {
                                    cell_ui.add({
                                        let mut text = RichText::new(label);

                                        if monospace {
                                            text = text.monospace();
                                        }
                                        egui::Label::new(text).selectable(false)
                                    });
                                }

                                if !matches!(cell_kind, CellKind::Corner) {
                                    if let Some(label) = label {
                                        if response.dragged() {
                                            Tooltip::always_open(ctx.clone(), ui_layer_id, "_egui_deferred_table_dnd_".into(), PopupAnchor::Pointer)
                                                .gap(12.0)
                                                .show(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(label);
                                                    });
                                                });
                                        }
                                    }

                                    // Highlight drop target
                                    if response.dnd_hover_payload::<DndPayload>().is_some() {
                                        ui.painter().rect_filled(
                                            cell_clip_rect,
                                            CornerRadius::ZERO,
                                            ui.style().visuals.selection.bg_fill.gamma_multiply(0.25),
                                        );
                                    }

                                    // handle dnd release
                                    if let Some(payload) = response.dnd_release_payload::<DndPayload>() {
                                        match (payload.cell_kind, cell_kind) {
                                            // currently only dragging like onto like is supported.
                                            (CellKind::ColumnHeader, CellKind::ColumnHeader) => if payload.index != mapped_column_index {
                                                info!("dnd release: column {} -> column {}", payload.index, mapped_column_index);
                                                actions.push(Action::ColumnReorder{ from: payload.index, to: mapped_column_index })
                                            }
                                            (CellKind::RowHeader, CellKind::RowHeader) => if payload.index != mapped_row_index {
                                                info!("dnd release: row {} -> row {}", payload.index, mapped_row_index);
                                                actions.push(Action::RowReorder{ from: payload.index, to: mapped_row_index })
                                            }
                                            _ => ()
                                        }
                                    }
                                }

                                if grid_row_index == 0 {
                                    table_width += cell_clip_rect.size().x + 1.0;
                                }
                                if grid_column_index == 0 {
                                    table_height += cell_clip_rect.size().y + 1.0;
                                }
                            }
                            accumulated_row_heights += outer_row_height + 1.0;
                        }

                        trace!("cells");

                        let cells_clip_rect = Rect::from_min_max((table_max_rect.min + outer_cell_size) + Vec2::splat(1.0), translated_viewport_rect.max).intersect(parent_clip_rect);
                        if false {
                            ui.painter().debug_rect(cells_clip_rect, Color32::CYAN, "cr");
                        }

                        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                            ui.set_clip_rect(translated_viewport_rect);
                            // ui.skip_ahead_auto_ids(???); // TODO Make sure we get consistent IDs.

                            let table_max_rect = ui.max_rect();

                            //
                            // display the table
                            //

                            let start_pos = table_max_rect.min;

                            // reset the visual row index for the cells, skipping the header row.
                            row_counter = cell_origin.row + 1 - first_row_filtered_count;

                            // start with an offset equal to header height, which is currently using the cell_size
                            let mut accumulated_row_heights = outer_cell_size.y + 1.0;
                            for grid_row_index in 1..=visible_row_count {
                                if grid_row_index + cell_origin.row > dimensions.row_count {
                                    break
                                }

                                let visible_row_index = cell_origin.row + (grid_row_index.saturating_sub(1));
                                let mapped_row_index = Self::map_index(dimensions.row_count, row_ordering, visible_row_index);

                                if let Some(rows_to_filter) = &rows_to_filter {
                                    if rows_to_filter.contains(&mapped_row_index) {
                                        trace!("filtered row");
                                        continue;
                                    }
                                }
                                row_counter += 1;

                                let inner_row_height = state.row_heights[mapped_row_index];
                                let outer_row_height = inner_row_height + outer_inner_difference.y;

                                let row_bg_color = striped_row_color(row_counter, &ui.style()).unwrap_or(ui.style().visuals.panel_fill);

                                let y = start_pos.y + accumulated_row_heights;

                                // start with an offset equal to header width, which is currently using the cell_size
                                let mut accumulated_column_widths = outer_cell_size.x + 1.0;

                                for grid_column_index in 1..=visible_column_count {
                                    if grid_column_index + cell_origin.column > dimensions.column_count {
                                        break
                                    }

                                    let visible_column_index = cell_origin.column + (grid_column_index - 1);
                                    let mapped_column_index = Self::map_index(dimensions.column_count, column_ordering, visible_column_index);

                                    if let Some(columns_to_filter) = &columns_to_filter {
                                        if columns_to_filter.contains(&mapped_column_index) {
                                            trace!("filtered column");
                                            continue;
                                        }
                                    }

                                    let inner_column_width = state.column_widths[visible_column_index];
                                    let outer_column_width = inner_column_width + outer_inner_difference.x;

                                    let cell_index = CellIndex {
                                        row: mapped_row_index,
                                        column: mapped_column_index,
                                    };

                                    let x = start_pos.x + accumulated_column_widths;
                                    accumulated_column_widths += outer_column_width + 1.0;

                                    let cell_rect = Rect::from_min_size(Pos2::new(x, y), (outer_column_width, outer_row_height).into());
                                    let cell_clip_rect = cell_rect.intersect(cells_clip_rect);
                                    let cell_clip_rect_size = cell_clip_rect.size();

                                    let cell_inner_rect = cell_rect.shrink2(outer_inner_half_difference);
                                    let cell_inner_clip_rect = cell_inner_rect.intersect(cell_clip_rect);

                                    let skip = cell_clip_rect_size.x < 0.0 || cell_clip_rect_size.y < 0.0;

                                    trace!("grid: r={}, c={}, rect: {:?}, pos: {:?}, size: {:?}, skip: {}", grid_row_index, grid_column_index, cell_clip_rect, cell_clip_rect.min, cell_clip_rect_size, skip);

                                    if skip {
                                        continue;
                                    }

                                    let response = ui.allocate_rect(cell_clip_rect, Sense::click());

                                    let bg_color = if self.parameters.highlight_hovered_cell && response.contains_pointer() {
                                        ui.style().visuals.widgets.hovered.weak_bg_fill
                                    } else {
                                        row_bg_color
                                    };

                                    ui.painter()
                                        .with_clip_rect(cell_clip_rect)
                                        .rect_filled(cell_rect, 0.0, bg_color);

                                    // note: cannot use 'response.clicked()' here as the the cell 'swallows' the click if the contents are interactive.
                                    if response.contains_pointer() && ui.ctx().input(|i| i.pointer.primary_released()) {
                                        // FIXME this doesn't track if the click location is in the same cell, that is, this will
                                        //       be triggered if you click somewhere, then release in this cell.
                                        //       which is not the intention.
                                        actions.push(Action::CellClicked(cell_index));
                                    }

                                    // TODO track double clicks

                                    if SHOW_CELL_BORDERS {
                                        ui.painter()
                                            .with_clip_rect(cell_clip_rect)
                                            .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);
                                    }

                                    let mut cell_ui = ui.new_child(UiBuilder::new().max_rect(cell_inner_rect));
                                    cell_ui.set_clip_rect(cell_inner_clip_rect);
                                    cell_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                                    renderer.render_cell(&mut cell_ui, cell_index, data_source);
                                }
                                accumulated_row_heights += outer_row_height + 1.0;
                            }
                        });

                        let line_stroke = ui.style().visuals.window_stroke;
                        ui.painter()
                            .with_clip_rect(inner_max_rect)
                            .hline(table_max_rect.min.x..=table_max_rect.min.x + table_width, table_max_rect.min.y + outer_cell_size.y, line_stroke);

                        ui.painter()
                            .with_clip_rect(inner_max_rect)
                            .vline(table_max_rect.min.x + outer_cell_size.x, table_max_rect.min.y..=table_max_rect.min.y + table_height, line_stroke);
                    });
            });
        });

        if clear_drag_state {
            temp_state.drag_state = None;
        }

        let repaint = match drag_action.take() {
            None => false,
            Some(DragAction::SetWidth(index, new_width)) => {
                state.column_widths[index] = new_width;
                true
            }
            Some(DragAction::SetHeight(index, new_height)) => {
                state.row_heights[index] = new_height;
                true
            }
        };

        if repaint {
            ui.ctx().request_repaint();
        }

        DeferredTablePersistentState::store(ui.ctx(), persistent_state_id, state);
        DeferredTableTempState::store(ui.ctx(), temp_state_id, temp_state);

        (ui.response(), actions)
    }

    fn paint_resize_handle(
        ui: &mut Ui,
        points: [Pos2; 2],
        state: DragHandleState,
        cell_painter: &Painter,
        cell_kind: CellKind,
    ) {
        let stroke = match state {
            DragHandleState::Disabled => ui.visuals().widgets.noninteractive.bg_stroke,
            DragHandleState::Inactive => ui.visuals().widgets.open.bg_stroke,
            DragHandleState::Hovered => ui.style().visuals.widgets.hovered.bg_stroke,
            DragHandleState::Dragged => ui.style().visuals.widgets.active.bg_stroke,
        };

        cell_painter.line_segment(points, stroke);

        match state {
            DragHandleState::Disabled => {
                ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
            }
            DragHandleState::Inactive => {}
            DragHandleState::Dragged | DragHandleState::Hovered => match cell_kind {
                CellKind::ColumnHeader => {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                }
                CellKind::RowHeader => {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeRow);
                }
                _ => unreachable!(),
            },
        }
    }

    fn build_cell_kind(grid_row_index: usize, grid_column_index: usize) -> CellKind {
        if grid_row_index == 0 && grid_column_index == 0 {
            CellKind::Corner
        } else if grid_row_index == 0 {
            CellKind::ColumnHeader
        } else if grid_column_index == 0 {
            CellKind::RowHeader
        } else {
            CellKind::Value
        }
    }

    fn build_row_kind(grid_row_index: usize) -> RowKind {
        if grid_row_index == 0 {
            RowKind::HeaderRow
        } else {
            RowKind::ValuesRow
        }
    }

    fn map_index(count: usize, row_ordering: &[usize], visible_row_index: usize) -> usize {
        let mut mapped_row_index = *row_ordering
            .get(visible_row_index)
            .unwrap_or(&visible_row_index);
        if mapped_row_index >= count {
            // handle out-of-range mapping values
            mapped_row_index = visible_row_index;
        }
        mapped_row_index
    }
}

fn striped_row_color(row: usize, style: &Style) -> Option<Color32> {
    if row % 2 == 1 {
        Some(style.visuals.faint_bg_color)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DragHandleState {
    Disabled,
    Inactive,
    Hovered,
    Dragged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CellKind {
    Corner,
    ColumnHeader,
    RowHeader,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RowKind {
    HeaderRow,
    ValuesRow,
}

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
    /// 1. [`DeferredTableDataSource::column_ordering`]
    /// 2. [`apply_reordering`]
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
    /// 1. [`DeferredTableDataSource::row_ordering`]
    /// 2. [`apply_reordering`]
    RowReorder {
        from: usize,
        to: usize,
    },
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

impl TableDimensions {
    pub fn is_empty(&self) -> bool {
        self.row_count == 0 || self.column_count == 0
    }
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

    drag_state: Option<DragState>,
}

#[derive(Clone, Copy)]
struct DragState {
    index: usize,
    start_pos: Pos2,
    cell_kind: CellKind,
    initial_size: f32,
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
    /// called once per frame, before any other methods are used.
    fn prepare(&mut self) {}
    /// called once per frame, after the source has been used.
    fn finalize(&mut self) {}

    fn get_dimensions(&self) -> TableDimensions;
}

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

/// Specifies the axis (row/column) parameters.
///
/// Since min/max/default dimension can all conflict or be specified in a different order they must be sanitized before use
/// in the following order: default -> clamp(min, max)
///
/// debug_asserts are raised if any values are < 0
/// in release builds default/min/max have a minimum of 0 at runtime.
#[derive(Debug, Clone)]
pub struct AxisParameters {
    name: Option<String>,
    /// The row/column height/width, respectively
    default_dimension: Option<f32>,
    /// The row/column height/width range, respectively
    dimension_range: Rangef,
    resizable: bool,
    monospace: bool,
}

impl Default for AxisParameters {
    fn default() -> Self {
        Self {
            name: None,
            default_dimension: None,
            dimension_range: Rangef::new(10.0, f32::INFINITY),
            resizable: true,
            monospace: false,
        }
    }
}

impl AxisParameters {
    pub fn name(mut self, s: impl Into<String>) -> Self {
        self.name = Some(s.into());
        self
    }

    /// The row/column height/width, respectively
    pub fn default_dimension(mut self, value: f32) -> Self {
        debug_assert!(value >= 0.0);
        self.default_dimension = Some(value.at_least(0.0));
        self
    }

    /// default: 10.0
    ///
    /// if the row/column is resizable, then the minimum dimension might be larger the value specified here, or the default,
    /// due to the space required for resize handles and resize handle interaction constraints
    pub fn minimum_dimension(mut self, value: f32) -> Self {
        debug_assert!(value >= 0.0);
        self.dimension_range.min = value.at_least(0.0);
        self
    }

    /// a value f32::INFINITY allows the row/column to be resized to be as large as possible
    ///
    /// default: f32::INFINITY
    pub fn maximum_dimension(mut self, value: f32) -> Self {
        debug_assert!(value >= 0.0);
        self.dimension_range.max = value.at_least(0.0);
        self
    }

    pub fn resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }

    pub fn monospace(mut self, value: bool) -> Self {
        self.monospace = value;
        self
    }
}

/// Helper for rendering tables based on tuple slices
///
/// Implementations of `DeferredTableRender` for tuples with 2 to 16 elements are provided by the
/// `impl_deferred_table_for_tuple` macro.
///
/// See crate examples.
#[derive(Default)]
pub struct SimpleTupleRenderer {}

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

        impl<$($T: std::fmt::Display),*> DeferredTableRenderer<&[($($T),*)]> for SimpleTupleRenderer {
            fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &&[($($T),*)]) {
                if let Some(row_data) = source.get(cell_index.row) {
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

/// Helper method to be used by clients to help with handling column re-ordering during action processing.
///
/// ```text
/// match action {
///     Action::ColumnReorder { from, to } => {
///         egui_deferred_table::apply_column_reordering(&mut column_ordering, from, to);
///     }
///     // ...
/// }
///```
///
/// See also:
/// 1. [`DeferredTableDataSource::column_ordering`]
/// 2. [`DeferredTableDataSource::row_ordering`]
/// 3. [`Action::ColumnReorder`]
///
pub fn apply_reordering(ordering: &mut Option<Vec<usize>>, from: usize, to: usize) {
    if from == to {
        return;
    }

    //
    // Part 1
    //

    // Initialize ordering if it doesn't exist
    if ordering.is_none() {
        *ordering = Some(Vec::new());
    }

    // Get a mutable reference to column_ordering
    let ordering = ordering.as_mut().unwrap();

    // Find the maximum index needed
    let max_index = from.max(to);

    // Expand the vector if needed to include max_index
    while ordering.len() <= max_index {
        ordering.push(ordering.len());
    }

    //
    // Part 2: Perform the actual move
    //

    // Find positions of 'from' and 'to' in the ordering vector
    let from_pos = ordering.iter().position(|&x| x == from).unwrap();
    let to_pos = ordering.iter().position(|&x| x == to).unwrap();

    // Remove 'from' from its current position
    ordering.remove(from_pos);

    // if to_pos was after from_pos, it will be out by one, but this is factored into the code below.
    ordering.insert(to_pos, from);
}

#[cfg(test)]
mod reordering_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    // dragging left
    #[case(0,1,vec![1,0], vec![0,1])]
    #[case(4,0,vec![0,1,2,3,4,5,6], vec![4,0,1,2,3,5,6])]
    #[case(10,0,vec![], vec![10,0,1,2,3,4,5,6,7,8,9])]
    // dragging right
    #[case(0,1,vec![], vec![1,0])]
    #[case(1,0,vec![1,0], vec![0,1])]
    #[case(1,0,vec![1,0,2,3,4], vec![0,1,2,3,4])]
    #[case(4,0,vec![], vec![4,0,1,2,3])]
    #[case(4,3,vec![4,0,1,2,3,5,6], vec![0,1,2,3,4,5,6])]
    #[case(10,9,vec![10,0,1,2,3,4,5,6,7,8,9], vec![0,1,2,3,4,5,6,7,8,9,10])]
    // from/to same
    #[case(0,0,vec![], vec![])]
    #[case(4,4,vec![0,1], vec![0,1])]
    fn test_apply_reordering(
        #[case] from: usize,
        #[case] to: usize,
        #[case] ordering: Vec<usize>,
        #[case] expected: Vec<usize>,
    ) {
        let mut ordering = Some(ordering);
        apply_reordering(&mut ordering, from, to);
        assert_eq!(ordering, Some(expected));
    }
}
