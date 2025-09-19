extern crate core;

use chrono::{DateTime, Local};
use egui::{Ui, ViewportBuilder, WidgetText};
use egui_deferred_table::{Action, AxisParameters, DeferredTable};
use egui_dock::{DockArea, DockState, NodeIndex};
use log::Level;
use shared::data::futurama;
use shared::data::futurama::RowType;
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
            name: "Simple (initially floating)",
            kind: TabKind::SimpleTable {
                state: SimpleTableState::default(),
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

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut tab_context = TabContext {
            data: &mut self.data,
            log_entries: &mut self.log_entries,
        };

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Fixed data demo");
                ui.checkbox(&mut self.inspection, "🔍 Inspection");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            DockArea::new(&mut self.tree).show_inside(
                ui,
                &mut TabViewer {
                    context: &mut tab_context,
                },
            );
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
    SimpleTable { state: SimpleTableState },
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
                contents_simple_table(ui, context, state);
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
    _state: &mut InsideScrollAreaState,
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
            .show(ui, &mut data_source);

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
}

fn contents_simple_table(ui: &mut Ui, context: &mut TabContext, _state: &mut SimpleTableState) {
    struct Params {
        default_width: f32,
        maximum_width: f32,
        minimum_width: f32,
        resizable: bool,
    }

    const FIELD_PARAMS: [Params; 8] = [
        Params {
            default_width: 100.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
        },
        Params {
            default_width: 80.0,
            maximum_width: 0.0,
            minimum_width: 0.0,
            resizable: false,
        },
        Params {
            default_width: 100.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
        },
        Params {
            default_width: 400.0,
            maximum_width: f32::INFINITY,
            minimum_width: 50.0,
            resizable: true,
        },
        Params {
            default_width: 125.0,
            maximum_width: 400.0,
            minimum_width: 50.0,
            resizable: true,
        },
        Params {
            default_width: 100.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
        },
        Params {
            default_width: 100.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
        },
        Params {
            default_width: 80.0,
            maximum_width: 200.0,
            minimum_width: 25.0,
            resizable: true,
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
                .default_width(field_params.default_width)
                .minimum_dimension(field_params.minimum_width)
                .maximum_dimension(field_params.maximum_width)
        })
        .collect::<Vec<_>>();

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .column_parameters(&column_params)
        .show(ui, &mut data_source);

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
pub struct SimpleTableState {
    // here we could add state for table properties/presentation/etc.
}

fn contents_log(ui: &mut Ui, context: &mut TabContext, _state: &mut LogState) {
    let mut data_source = context.log_entries.as_slice();

    let column_params = vec![
        AxisParameters::default()
            .name("Time".to_string())
            .default_width(200.0),
        AxisParameters::default()
            .name("Level".to_string())
            .default_width(100.0),
        AxisParameters::default()
            .name("Message".to_string())
            .default_width(400.0),
    ];

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .column_parameters(&column_params)
        .show(ui, &mut data_source);

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
            _ => {
                // ignored
            }
        }
    }
}
