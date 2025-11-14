use egui::emath::GuiRounding;
use egui::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, Context, CornerRadius, Frame, Id, Margin, NumExt, Painter, PointerButton, PopupAnchor,
    Pos2, Rangef, Rect, Response, RichText, Sense, Shadow, Stroke, StrokeKind, Style, Tooltip, Ui,
    UiBuilder, UiKind, UiStackInfo, Vec2,
};
use log::{info, trace};
use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::ops::{Add, Range, Sub};

mod actions;
mod cells;
mod data_source;
mod dimensions;
mod editing;
mod ordering;
mod parameters;
mod slices;
mod table_renderer;

pub use actions::*;
pub use cells::*;
pub use data_source::*;
pub use dimensions::*;
pub use editing::*;
pub use ordering::*;
pub use parameters::*;
pub use slices::*;
pub use table_renderer::*;

const SHOW_HEADER_CELL_BORDERS: bool = false;
const SHOW_CELL_BORDERS: bool = false;

const EDITOR_FRAME: Frame = Frame {
    inner_margin: Margin::ZERO,
    stroke: Stroke::NONE,
    fill: Color32::TRANSPARENT,
    corner_radius: CornerRadius::ZERO,
    outer_margin: Margin::ZERO,
    shadow: Shadow::NONE,
};

pub struct DeferredTable<'a, DataSource> {
    id: Id,
    parameters: DeferredTableParameters<'a>,
    phantom_data: PhantomData<DataSource>,
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

    /// default: one-based headers
    pub fn zero_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = true;
        self
    }

    /// default: enabled
    pub fn one_based_headers(mut self) -> Self {
        self.parameters.zero_based_headers = false;
        self
    }

    /// default: enabled,
    pub fn selectable_rows(mut self) -> Self {
        self.parameters.selectable_rows = true;
        self
    }

    /// default: selectable-rows enabled
    pub fn selectable_rows_disabled(mut self) -> Self {
        self.parameters.selectable_rows = false;
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
        self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
    {
        let editor: Option<&mut NullEditor> = None;
        let edit_state: Option<&mut EditorState<(), ()>> = None;
        self.show_outer::<Renderer, NullEditor, _, _>(ui, data_source, renderer, editor, edit_state)
    }

    pub fn show_and_edit<Renderer, Editor, IS, V>(
        self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
        editor: &mut Editor,
        edit_state: &mut EditorState<IS, V>,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
        Editor: EditableTableRenderer<DataSource, ItemState = IS, Value = V>,
    {
        self.show_outer(ui, data_source, renderer, Some(editor), Some(edit_state))
    }

    fn show_outer<Renderer, Editor, IS, V>(
        self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
        editor: Option<&mut Editor>,
        edit_state: Option<&mut EditorState<IS, V>>,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
        Editor: EditableTableRenderer<DataSource, ItemState = IS, Value = V>,
    {
        data_source.prepare();
        // cache the dimensions now, to remain consistent, since the data_source could return different dimensions
        // each time it's called.

        let dimensions = data_source.get_dimensions();

        let result = if !dimensions.is_empty() {
            self.show_inner(ui, data_source, renderer, dimensions, edit_state, editor)
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
    fn show_inner<Renderer, Editor, IS, V>(
        mut self,
        ui: &mut Ui,
        data_source: &mut DataSource,
        renderer: &mut Renderer,
        dimensions: TableDimensions,
        mut edit_state: Option<&mut EditorState<IS, V>>,
        mut editor: Option<&mut Editor>,
    ) -> (Response, Vec<Action>)
    where
        DataSource: DeferredTableDataSource,
        Renderer: DeferredTableRenderer<DataSource>,
        Editor: EditableTableRenderer<DataSource, ItemState = IS, Value = V>,
    {
        let ctx = ui.ctx().clone();
        let style = ui.style();
        let pixels_per_point = ctx.pixels_per_point();

        // We need an OPAQUE 'color' so that when it's used to fill the background of the editor popup window the
        // contents behind the window are obscured.
        let opaque_faint_bg_color = if style.visuals.faint_bg_color.is_opaque() {
            style.visuals.faint_bg_color
        } else {
            style.visuals.panel_fill.add(style.visuals.faint_bg_color)
        };

        let opaque_faint_selected_bg_color = style.visuals.selection.bg_fill.gamma_multiply(0.8);

        // we need to use `any_down`, since DRAGGING doesn't count as a click in `Response::clicked_elsewhere()`
        let (pointer_interact_pos, any_down) = ctx.input(|i| {
            (
                i.pointer.latest_pos().unwrap_or_default(),
                i.pointer.any_down(),
            )
        });

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
        let mut request_row_selection_changed_action = false;

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

        let dimensions_changed = temp_state
            .dimensions
            .map_or(true, |previous_frame_dimensions| {
                previous_frame_dimensions != dimensions
            });

        if dimensions_changed {
            temp_state.dimensions = Some(dimensions);

            // remove non-visible selections
            temp_state.row_selections.retain(|&mapped_row_id| {
                let visible = mapped_row_id < dimensions.row_count;

                if !visible {
                    request_row_selection_changed_action = true;
                }
                visible
            });
        }

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

            #[derive(Debug, Copy, Clone, Hash)]
            enum CellId {
                Corner,
                MappedColumn(usize),
                MappedRow(usize),
                Cell(CellIndex),
            }

            //
            // container for the table and the scroll bars.
            //

            let column_ordering = renderer.column_ordering().unwrap_or_default();
            let row_ordering = renderer.row_ordering().unwrap_or_default();

            let outer_inner_difference = outer_cell_size - inner_cell_size;
            // pre-calculate to avoid doing the divide for every cell.
            let outer_inner_half_difference = outer_inner_difference / 2.0;

            // add the width/height of the column/row headers to the sum of the column widths/row heights, respectively,
            // while ignoring widths/heights that don't apply to the current dimensions.
            let total_content_width = state.column_widths
                .iter()
                .take(dimensions.column_count)
                .sum::<f32>() + ((outer_inner_difference.x + 1.0) * dimensions.column_count as f32) + outer_cell_size.x;
            let total_content_height = state.row_heights
                .iter()
                .take(dimensions.row_count)
                .sum::<f32>() + ((outer_inner_difference.y + 1.0) * dimensions.row_count as f32) + outer_cell_size.y;

            let columns_to_filter = renderer.columns_to_filter();
            let filtered_content_width = columns_to_filter.map_or(0.0,|columns|{
                columns.iter().take(dimensions.column_count).map(|index| {
                    let mapped_index = Self::map_index(dimensions.column_count, column_ordering, *index);
                    state.column_widths.get(mapped_index).map(|it|it + outer_inner_difference.x + 1.0).unwrap_or(0.0)
                }).sum::<f32>()
            });

            let rows_to_filter = renderer.rows_to_filter();
            let filtered_content_height = rows_to_filter.map_or(0.0,|rows|{
                rows.iter().take(dimensions.row_count).map(|index| {
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
                        let viewport_changed = temp_state.last_viewport_rect.map_or(false, |last_viewport_rect| {
                            last_viewport_rect != viewport_rect
                        });

                        temp_state.last_viewport_rect = Some(viewport_rect);

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
                            .translate(ui.max_rect().min.to_vec2())
                            .round_to_pixels(pixels_per_point);

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

                            let row_was_selected = if matches!(row_kind, RowKind::ValuesRow) && self.parameters.selectable_rows {
                                temp_state.row_selections.contains(&mapped_row_index)
                            } else {
                                false
                            };

                            let row_bg_color = Self::pick_row_bg_color(opaque_faint_bg_color, opaque_faint_selected_bg_color, ui, row_counter, row_was_selected);

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
                                    state.column_widths[mapped_column_index]
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

                                let cell_id = match cell_kind {
                                    CellKind::Corner => CellId::Corner,
                                    CellKind::ColumnHeader => CellId::MappedColumn(mapped_column_index),
                                    CellKind::RowHeader => CellId::MappedRow(mapped_row_index),
                                    CellKind::Value => unreachable!(),
                                };

                                if matches!(cell_kind, CellKind::ColumnHeader | CellKind::RowHeader) {
                                    response.dnd_set_drag_payload(cell_id);
                                }

                                let mut cell_ui = ui.new_child(UiBuilder::new()
                                    .id_salt(cell_id)
                                    .max_rect(cell_inner_rect));
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
                                    //cell_ui.label(format!("{:?}", cell_ui.id()));
                                    let mut text = RichText::new(label);

                                    if monospace {
                                        text = text.monospace();
                                    }

                                    cell_ui.add({
                                        egui::Label::new(text).selectable(false)
                                    });
                                }

                                if response.clicked() {
                                    match cell_kind {
                                        CellKind::RowHeader => {
                                            if self.parameters.selectable_rows {
                                                match row_was_selected {
                                                    true => { temp_state.row_selections.remove(&mapped_row_index); },
                                                    false => { temp_state.row_selections.insert(mapped_row_index); },
                                                }
                                                request_row_selection_changed_action = true;
                                            }
                                        }
                                        // TODO selectable columns?
                                        _ => {}
                                    }
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
                                    if response.dnd_hover_payload::<CellId>().is_some() {
                                        ui.painter().rect_filled(
                                            cell_clip_rect,
                                            CornerRadius::ZERO,
                                            ui.style().visuals.selection.bg_fill.gamma_multiply(0.25),
                                        );
                                    }

                                    // handle dnd release
                                    if let Some(payload) = response.dnd_release_payload::<CellId>() {
                                        match (*payload, cell_id) {
                                            // currently only dragging like onto like is supported.
                                            (CellId::MappedColumn(payload_index), CellId::MappedColumn(current_index)) => if payload_index != current_index {
                                                info!("dnd release: column {} -> column {}", payload_index, current_index);
                                                actions.push(Action::ColumnReorder{ from: payload_index, to: mapped_column_index })
                                            }
                                            (CellId::MappedRow(payload_index), CellId::MappedRow(current_index)) => if payload_index != current_index {
                                                info!("dnd release: row {} -> row {}", payload_index, current_index);
                                                actions.push(Action::RowReorder{ from: payload_index, to: current_index })
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

                                let row_was_selected = if self.parameters.selectable_rows {
                                    temp_state.row_selections.contains(&mapped_row_index)
                                } else {
                                    false
                                };

                                let row_bg_color = Self::pick_row_bg_color(opaque_faint_bg_color, opaque_faint_selected_bg_color, ui, row_counter, row_was_selected);

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

                                    let inner_column_width = state.column_widths[mapped_column_index];
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

                                        if let (Some(editor), Some(edit_state)) = (editor.as_mut(), edit_state.as_mut()) {
                                            self.handle_editable_cell_click(data_source, cell_index, *editor, *edit_state);
                                        }
                                    }

                                    // TODO track double clicks

                                    if SHOW_CELL_BORDERS {
                                        ui.painter()
                                            .with_clip_rect(cell_clip_rect)
                                            .rect_stroke(cell_rect, CornerRadius::ZERO, ui.style().visuals.widgets.noninteractive.bg_stroke, StrokeKind::Inside);
                                    }

                                    let cell_id = CellId::Cell(cell_index);

                                    let mut cell_ui = ui.new_child(UiBuilder::new()
                                        .id_salt(cell_id)
                                        .ui_stack_info(UiStackInfo::new(UiKind::TableCell))
                                        .max_rect(cell_inner_rect));
                                    cell_ui.set_clip_rect(cell_inner_clip_rect);
                                    cell_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                                    //cell_ui.label(format!("{:?}", cell_ui.id()));

                                    let mut handled = false;

                                    editor.as_mut().zip(edit_state.as_mut()).map(|(editor, edit_state)| {
                                        match &mut edit_state.state {
                                            Some(CellEditState::Editing(editing_cell_index, item_state, value)) if cell_index.eq(editing_cell_index) => {

                                                let mut editor_frame: Frame = EDITOR_FRAME;
                                                editor_frame.fill = row_bg_color;
                                                editor_frame.stroke = Stroke::new(1.0, ui.style().visuals.window_stroke.color);
                                                editor_frame.inner_margin = Margin::same(1);

                                                egui::Window::new("")
                                                    .title_bar(false)
                                                    .id(cell_ui.id().with("cell_editor"))
                                                    .frame(editor_frame)
                                                    .constrain_to(cells_clip_rect)
                                                    .fixed_pos(cell_rect.min.sub(Vec2::splat(2.0)))
                                                    .auto_sized()
                                                    .default_rect(cell_rect)
                                                    .min_size(cell_rect.size())
                                                    .max_width(cell_rect.width())
                                                    .show(&ctx, |ui|{
                                                        //ui.set_min_width(cell_rect.width());
                                                        ui.set_min_size(cell_rect.size());
                                                        editor.render_cell_editor(ui, &cell_index, item_state, value, data_source);
                                                    })
                                                    .map(|window|
                                                    {
                                                        let window_response = window.response;

                                                        // cannot use `resp.contains_pointer()` here, since it doesn't take into account popups generated by clicking on a drop-down which appear
                                                        // above the window and may appear outside of it. (e.g. small cell and a larger drop-down)
                                                        let window_contains_pointer = window_response.contains_pointer();
                                                        // cannot use `resp.clicked_elsewhere()` here, since it doesn't it doesn't take into account a DRAG started or stopped outside the reponse area.
                                                        // (e.g. try creating a text editor outside of table and selecting text in it while an editor is visible), this will be false.
                                                        let window_clicked_elsewhere = window_response.clicked_elsewhere();

                                                        let cells_clip_rect_contains_pointer = cells_clip_rect.contains(pointer_interact_pos);

                                                        trace!("clicked_elsewhere: {}, response_rect_contains_pointer: {}, any_down: {}, contains_pointer: {}, viewport_changed: {}", window_clicked_elsewhere, cells_clip_rect_contains_pointer, any_down, window_contains_pointer, viewport_changed);

                                                        // given the above issues and scenarious, we use `any_down` (see above) and since the editor window is constrained to the cells_clip_rect can see if the pointer
                                                        // is still within the table area too.

                                                        let apply_edit = if any_down && !cells_clip_rect_contains_pointer {
                                                            trace!("applying edit due a button down and cells_clip_rect does not contain pointer");
                                                            true
                                                        } else if viewport_changed {
                                                            trace!("applying edit due to viewport change");
                                                            true
                                                        } else {
                                                            false
                                                        };

                                                        if apply_edit {
                                                            Self::apply_edit(data_source, cell_index, *editor, edit_state);
                                                        }
                                                    });

                                                handled = true;
                                            }
                                            _ => {}
                                        }
                                    });

                                    if !handled {
                                        renderer.render_cell(&mut cell_ui, cell_index, data_source);
                                    }
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

                        ui.response()
                    });
            });
        });

        if clear_drag_state {
            temp_state.drag_state = None;
        }

        if request_row_selection_changed_action {
            actions.push(Action::RowSelectionChanged {
                selection: temp_state.row_selections.clone(),
            });
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

    fn pick_row_bg_color(
        opaque_faint_bg_color: Color32,
        opaque_faint_selected_bg_color: Color32,
        ui: &mut Ui,
        row_counter: usize,
        row_was_selected: bool,
    ) -> Color32 {
        if row_was_selected {
            striped_row_color(row_counter, opaque_faint_selected_bg_color)
                .unwrap_or(ui.style().visuals.selection.bg_fill)
        } else {
            striped_row_color(row_counter, opaque_faint_bg_color)
                .unwrap_or(ui.style().visuals.widgets.noninteractive.weak_bg_fill)
        }
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

    /// call this function from a cell action handler
    pub fn handle_editable_cell_click<IS, V>(
        &mut self,
        source: &mut DataSource,
        cell_index: CellIndex,
        editor: &mut dyn EditableTableRenderer<DataSource, ItemState = IS, Value = V>,
        edit_state: &mut EditorState<IS, V>,
    ) {
        match &edit_state.state {
            None => {
                // change selection
                edit_state.state.replace(CellEditState::Pivot(cell_index));
            }
            Some(CellEditState::Pivot(pivot_cell_index)) if *pivot_cell_index == cell_index => {
                trace!("clicked in selected cell");

                // change mode to edit
                let item_state = editor.build_item_state(cell_index, source);
                if let Some((edit, original_item)) = item_state {
                    edit_state.state.replace(CellEditState::Editing(
                        cell_index,
                        edit,
                        original_item,
                    ));
                }
            }
            Some(CellEditState::Pivot(_)) => {
                trace!("clicked in different cell");

                // change selection
                edit_state.state.replace(CellEditState::Pivot(cell_index));
            }
            Some(CellEditState::Editing(editing_cell_index, _cell_edit_state, _original_item))
                if *editing_cell_index == cell_index =>
            {
                trace!("clicked in cell while editing");

                // nothing to do
            }
            Some(CellEditState::Editing(_editing_cell_index, _cell_edit_state, _original_item)) => {
                trace!("clicked in a different cell while editing");

                Self::apply_edit(source, cell_index, editor, edit_state);
            }
        }
    }

    fn apply_edit<IS, V>(
        source: &mut DataSource,
        cell_index: CellIndex,
        editor: &mut dyn EditableTableRenderer<DataSource, Value = V, ItemState = IS>,
        edit_state: &mut EditorState<IS, V>,
    ) {
        trace!("applying edit");
        // apply edited value
        let Some(CellEditState::Editing(index, state, original_item)) = edit_state.state.take()
        else {
            unreachable!();
        };
        editor.on_edit_complete(index, state, original_item, source);

        // change selection
        edit_state.state.replace(CellEditState::Pivot(cell_index));
    }
}

fn striped_row_color(row: usize, striped_color: Color32) -> Option<Color32> {
    if row % 2 == 1 {
        Some(striped_color)
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

/// State that could be stored between application restarts
#[derive(Default, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct DeferredTablePersistentState {
    // FUTURE We *could* add row/column ordering/filtering here too
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
    last_viewport_rect: Option<Rect>,

    // the collection here needs to a have a fast lookup, slow insertion/removal is fine.
    // this is because we render frames often and insert/remove infrequently.
    row_selections: BTreeSet<usize>,
    /// holds the dimensions used on the last render.
    dimensions: Option<TableDimensions>,
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
