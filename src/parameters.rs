use egui::{NumExt, Rangef, Vec2};

/// Specifies the axis (row/column) parameters.
///
/// Since min/max/default dimension can all conflict or be specified in a different order they must be sanitized before use
/// in the following order: default -> clamp(min, max)
///
/// debug_asserts are raised if any values are < 0
/// in release builds default/min/max have a minimum of 0 at runtime.
#[derive(Debug, Clone)]
pub struct AxisParameters {
    pub name: Option<String>,
    /// The row/column height/width, respectively
    pub default_dimension: Option<f32>,
    /// The row/column height/width range, respectively
    pub dimension_range: Rangef,
    pub resizable: bool,
    pub monospace: bool,
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

pub(crate) struct DeferredTableParameters<'a> {
    pub(crate) default_cell_size: Option<Vec2>,
    pub(crate) zero_based_headers: bool,
    pub(crate) highlight_hovered_cell: bool,
    pub(crate) min_size: Vec2,
    pub(crate) column_parameters: Option<&'a Vec<AxisParameters>>,
    pub(crate) row_parameters: Option<&'a Vec<AxisParameters>>,
    pub(crate) selectable_rows: bool,
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
            selectable_rows: true,
        }
    }
}
