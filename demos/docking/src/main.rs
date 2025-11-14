extern crate core;

use chrono::{DateTime, Local};
use egui::{Ui, ViewportBuilder, WidgetText};
use egui_deferred_table::{
    Action, AxisParameters, CellIndex, DeferredTable, DeferredTableRenderer, SimpleTupleRenderer,
    apply_reordering,
};
use egui_dock::{DockArea, DockState, NodeIndex};
use log::Level;
use shared::data::futurama;
use shared::data::futurama::{RowType, format_value};
use shared::growing::ui::GrowingTableState;
use shared::sparse::ui::SparseTableState;
use shared::spreadsheet::ui::SpreadsheetState;

fn main() -> eframe::Result<()> {
    // run with `RUST_LOG=egui_tool_windows=trace` to see trace logs
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([1027.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_deferred_table - docking windows demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

type LogEntry = (DateTime<Local>, Level, String);

struct MyApp {
    inspection: bool,

    data: Vec<RowType>,
    log_entries: Vec<LogEntry>,

    tree: DockState<Tab>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut log_entries = vec![];

        let mut tree = DockState::new(vec![
            Tab {
                name: "Growing",
                kind: TabKind::GrowingTable {
                    state: GrowingTableState::default(),
                },
            },
            Tab {
                name: "Sparse Table",
                kind: TabKind::SparseTable {
                    state: SparseTableState::default(),
                },
            },
            Tab {
                name: "Spreadsheet",
                kind: TabKind::Spreadsheet {
                    state: SpreadsheetState::default(),
                },
            },
        ]);

        let [a, _b] = tree.main_surface_mut().split_left(
            NodeIndex::root(),
            0.3,
            vec![Tab {
                name: "Tables in a tab",
                kind: TabKind::TableInsideScrollArea {
                    state: InsideScrollAreaState::default(),
                },
            }],
        );
        let [_, _] = tree.main_surface_mut().split_below(
            a,
            0.7,
            vec![Tab {
                name: "Log",
                kind: TabKind::Log {
                    state: LogState::default(),
                },
            }],
        );
        let _ = tree.add_window(vec![Tab {
            name: "Advanced (initially floating)",
            kind: TabKind::SimpleTable {
                state: AdvancedTableState::default(),
            },
        }]);

        example_log(&mut log_entries, Level::Info, "Demo started".into());

        Self {
            inspection: false,
            data: futurama::characters(),
            tree,
            log_entries,
        }
    }
}

fn example_log(entries: &mut Vec<LogEntry>, level: Level, message: String) {
    let entry = (Local::now(), level, message);
    entries.push(entry);
}

impl MyApp {
    fn top_panel_content(&mut self, ui: &mut Ui) {
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.label("Docking windows demo");
            },
            |ui| {
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            },
        );
    }

    fn central_panel_content(&mut self, ui: &mut Ui) {
        let mut tab_context = TabContext {
            data: &mut self.data,
            log_entries: &mut self.log_entries,
        };

        DockArea::new(&mut self.tree).show_inside(
            ui,
            &mut TabViewer {
                context: &mut tab_context,
            },
        );
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.top_panel_content(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.central_panel_content(ui);
        });

        // Inspection window
        egui::Window::new("🔍 Inspection")
            .open(&mut self.inspection)
            .vscroll(true)
            .show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });
    }
}

struct Tab {
    name: &'static str,
    kind: TabKind,
}

enum TabKind {
    TableInsideScrollArea { state: InsideScrollAreaState },
    SimpleTable { state: AdvancedTableState },
    Spreadsheet { state: SpreadsheetState },
    SparseTable { state: SparseTableState },
    GrowingTable { state: GrowingTableState },
    Log { state: LogState },
}

impl TabKind {
    pub fn ui(&mut self, ui: &mut egui::Ui, context: &mut TabContext) {
        match self {
            TabKind::TableInsideScrollArea { state } => {
                contents_inside_scroll_area(ui, context, state);
            }
            TabKind::SimpleTable { state } => {
                contents_advanced_table(ui, context, state);
            }
            TabKind::Spreadsheet { state } => {
                contents_spreadsheet(ui, context, state);
            }
            TabKind::SparseTable { state } => {
                contents_sparse_table(ui, context, state);
            }
            TabKind::GrowingTable { state } => {
                contents_growing_table(ui, context, state);
            }
            TabKind::Log { state } => {
                contents_log(ui, context, state);
            }
        }
    }
}

struct TabViewer<'a> {
    context: &'a mut TabContext<'a>,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        (&*tab.name).into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.push_id(ui.id().with(tab.name), |ui| {
            tab.kind.ui(ui, self.context);
        });
    }
}

struct TabContext<'a> {
    data: &'a mut Vec<RowType>,
    log_entries: &'a mut Vec<LogEntry>,
}

fn contents_inside_scroll_area(
    ui: &mut Ui,
    context: &mut TabContext,
    state: &mut InsideScrollAreaState,
) {
    ui.label("content above scroll area");
    ui.separator();

    egui::ScrollArea::both().max_height(200.0).show(ui, |ui| {
        // FIXME the table renders on top of this
        ui.label("content above table, inside scroll area");

        let mut data_source = context.data.as_slice();

        let column_params = futurama::fields()
            .iter()
            .map(|field| AxisParameters::default().name(field.to_string()))
            .collect::<Vec<_>>();

        let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
            .column_parameters(&column_params)
            .min_size((400.0, 400.0).into())
            .show(ui, &mut data_source, &mut state.renderer);

        for action in actions {
            match action {
                Action::CellClicked(cell_index) => example_log(
                    context.log_entries,
                    Level::Info,
                    format!("Cell clicked. cell: {:?}", cell_index),
                ),
                _ => {
                    // ignored
                }
            }
        }

        ui.label("content below table, inside scroll area");
    });

    ui.separator();
    ui.label("content below scroll area");
}

#[derive(Default)]
pub struct InsideScrollAreaState {
    // here we could add state for table properties/presentation/etc.
    renderer: SimpleTupleRenderer,
}

fn contents_advanced_table(ui: &mut Ui, context: &mut TabContext, state: &mut AdvancedTableState) {
    ui.label("Try dragging the column and rows headers to reorder them.");
    ui.label("Some columns are expandable, try resizing the window.");
    ui.label("Some columns are resizable, try resizing them.");

    ui.separator();

    struct Params {
        default_width: f32,
        maximum_width: f32,
        minimum_width: f32,
        resizable: bool,
        expandable: bool,
    }

    const FIELD_PARAMS: [Params; 8] = [
        Params {
            default_width: 100.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
            expandable: false,
        },
        Params {
            default_width: 80.0,
            maximum_width: 0.0,
            minimum_width: 0.0,
            resizable: false,
            expandable: false,
        },
        Params {
            default_width: 100.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
            expandable: false,
        },
        Params {
            default_width: 400.0,
            maximum_width: f32::INFINITY,
            minimum_width: 50.0,
            resizable: true,
            expandable: true,
        },
        Params {
            default_width: 125.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
            expandable: false,
        },
        Params {
            default_width: 100.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
            expandable: false,
        },
        Params {
            default_width: 100.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
            expandable: false,
        },
        Params {
            default_width: 80.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
            // NOTE: this specifically goes against the advice regarding multiple expandable columns, so the behaviour can be observed in this demo.
            expandable: true,
        },
    ];
    let mut data_source = context.data.as_slice();

    let column_params = futurama::fields()
        .iter()
        .zip(FIELD_PARAMS)
        .map(|(field_name, field_params)| {
            AxisParameters::default()
                .name(field_name.to_string())
                .resizable(field_params.resizable)
                .default_dimension(field_params.default_width)
                .minimum_dimension(field_params.minimum_width)
                .maximum_dimension(field_params.maximum_width)
                .expandable(field_params.expandable)
        })
        .collect::<Vec<_>>();

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .column_parameters(&column_params)
        .show(ui, &mut data_source, &mut state.renderer);

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => example_log(
                context.log_entries,
                Level::Info,
                format!("Cell clicked. cell: {:?}", cell_index),
            ),
            Action::ColumnReorder { from, to } => {
                apply_reordering(&mut state.renderer.column_ordering, from, to);
            }
            Action::RowReorder { from, to } => {
                apply_reordering(&mut state.renderer.row_ordering, from, to);
            }
            _ => {
                // ignored
            }
        }
    }
}

#[derive(Default)]
pub struct AdvancedTableState {
    renderer: AdvancedTableRenderer,
}

/// Supports row and column reordering, no persistence between application restart.
#[derive(Default)]
struct AdvancedTableRenderer {
    row_ordering: Option<Vec<usize>>,
    column_ordering: Option<Vec<usize>>,
}

impl DeferredTableRenderer<&[RowType]> for AdvancedTableRenderer {
    fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &&[RowType]) {
        ui.label(format_value(&source[cell_index.row], cell_index.column));
    }

    fn row_ordering(&self) -> Option<&[usize]> {
        self.row_ordering.as_ref().map(|v| v.as_slice())
    }

    fn column_ordering(&self) -> Option<&[usize]> {
        self.column_ordering.as_ref().map(|v| v.as_slice())
    }
}

fn contents_log(ui: &mut Ui, context: &mut TabContext, _state: &mut LogState) {
    let mut data_source = context.log_entries.as_slice();

    let column_params = vec![
        AxisParameters::default()
            .name("Time".to_string())
            .default_dimension(200.0),
        AxisParameters::default()
            .name("Level".to_string())
            .default_dimension(100.0),
        AxisParameters::default()
            .name("Message".to_string())
            .expandable(true)
            .default_dimension(400.0),
    ];

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .column_parameters(&column_params)
        .show(ui, &mut data_source, &mut _state.renderer);

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => example_log(
                context.log_entries,
                Level::Info,
                format!("Cell clicked. cell: {:?}", cell_index),
            ),
            _ => {
                // ignored
            }
        }
    }
}

#[derive(Default)]
pub struct LogState {
    // here would could add a filter, etc.
    renderer: SimpleTupleRenderer,
}

fn contents_spreadsheet(ui: &mut Ui, context: &mut TabContext, state: &mut SpreadsheetState) {
    shared::spreadsheet::ui::show_controls(ui, state);

    let (_response, mut actions) = shared::spreadsheet::ui::show_table(ui, state);

    // pre-process the actions
    actions.retain(|action| match action {
        Action::CellClicked(cell_index) => {
            example_log(
                context.log_entries,
                Level::Info,
                format!("Cell clicked. cell: {:?}", cell_index),
            );
            true
        }
        _ => true,
    });

    // use the default processing for remaining actions
    shared::spreadsheet::ui::handle_actions(actions, state);

    if state.is_automatic_recalculation_enabled() && state.needs_recalculation() {
        state.recalculate();
        ui.ctx().request_repaint();
    }
}

fn contents_sparse_table(ui: &mut Ui, _context: &mut TabContext, state: &mut SparseTableState) {
    shared::sparse::ui::show_controls(ui, state);

    let (_response, actions) = shared::sparse::ui::show_table(ui, state);

    shared::sparse::ui::handle_actions(actions, state);
}

fn contents_growing_table(ui: &mut Ui, context: &mut TabContext, state: &mut GrowingTableState) {
    shared::growing::ui::show_controls(ui, state);
    let (_response, actions) = shared::growing::ui::show_table(ui, state);

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => example_log(
                context.log_entries,
                Level::Info,
                format!("Cell clicked. cell: {:?}", cell_index),
            ),
            Action::RowSelectionChanged { selection } => {
                state.update_row_selection(selection);
            }
            _ => {
                // ignored
            }
        }
    }
}
//
// #[derive(Default)]
// struct SimpleTuple8Renderer {}
//
// impl DeferredTableRenderer<&[RowType]> for SimpleTuple8Renderer {
//     fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &&[RowType]) {
//         if let Some(row_data) = source.get(cell_index.row) {
//             match cell_index.column {
//                 0 => ui.label(row_data.0.to_string()),
//                 1 => ui.label(row_data.1.to_string()),
//                 2 => ui.label(row_data.2.to_string()),
//                 3 => ui.label(row_data.3.to_string()),
//                 4 => ui.label(row_data.4.to_string()),
//                 5 => ui.label(row_data.5.to_string()),
//                 6 => ui.label(row_data.6.to_string()),
//                 7 => ui.label(row_data.7.to_string()),
//                 _ => panic!("cell_index out of bounds. {:?}", cell_index),
//             };
//         }
//     }
// }
//
// #[derive(Default)]
// struct SimpleTuple3Renderer {}
//
// impl DeferredTableRenderer<&[LogEntry]> for SimpleTuple3Renderer {
//     fn render_cell(&self, ui: &mut Ui, cell_index: CellIndex, source: &&[LogEntry]) {
//         if let Some(row_data) = source.get(cell_index.row) {
//             match cell_index.column {
//                 0 => ui.label(row_data.0.to_string()),
//                 1 => ui.label(row_data.1.to_string()),
//                 2 => ui.label(row_data.2.to_string()),
//                 _ => panic!("cell_index out of bounds. {:?}", cell_index),
//             };
//         }
//     }
// }
