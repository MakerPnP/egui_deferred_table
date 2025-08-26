extern crate core;

use std::fmt::Display;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Local};
use egui::{Ui, ViewportBuilder, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex};
use log::Level;
use egui_deferred_table::{Action, DeferredTable, DeferredTableBuilder};
use shared::sparse::ui::SparseTableState;
use shared::spreadsheet::ui::SpreadsheetState;
use crate::futurama::{Kind, RowType};

mod futurama;

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

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
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
            Tab { name: "Sparse Table", kind: TabKind::SparseTable { state: Arc::new(Mutex::new(SparseTableState::default())) } },
            Tab { name: "Spreadsheet", kind: TabKind::Spreadsheet { state: Arc::new(Mutex::new(SpreadsheetState::default()))}   },
        ]);

        // You can modify the tree before constructing the dock
        let [a, _b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec![
                    Tab { name: "Tables in a tab", kind: TabKind::TableInsideScrollArea { state: Arc::new(Mutex::new(InsideScrollAreaState::default()))} },
                ]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(a, 0.7, vec![
                Tab { name: "Log", kind: TabKind::Log { state: Arc::new(Mutex::new(LogState::default()))} },
            ]);
        let _ = tree
            .add_window( vec![
                Tab { name: "Simple (initially floating)", kind: TabKind::SimpleTable { state: Arc::new(Mutex::new(SimpleTableState::default()))} },
            ]);

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
            DockArea::new(&mut self.tree)
                .show_inside(ui, &mut TabViewer { context: &mut tab_context});
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
    TableInsideScrollArea { state: Arc<Mutex<InsideScrollAreaState>> },
    SimpleTable { state: Arc<Mutex<SimpleTableState>> },
    Spreadsheet { state: Arc<Mutex<SpreadsheetState>> },
    SparseTable { state: Arc<Mutex<SparseTableState>> },
    Log { state: Arc<Mutex<LogState>> },
}

impl TabKind {
    pub fn ui(&mut self, ui: &mut egui::Ui, context: &mut TabContext) {
        match self {
            TabKind::TableInsideScrollArea { state } => {
                contents_inside_scroll_area(ui, context, state.lock().as_mut().unwrap());
            }
            TabKind::SimpleTable { state } => {
                contents_simple_table(ui, context, state.lock().as_mut().unwrap());
            }
            TabKind::Spreadsheet { state } => {
                contents_spreadsheet(ui, context, state.lock().as_mut().unwrap());
            }
            TabKind::SparseTable { state } => {
                contents_sparse_table(ui, context, state.lock().as_mut().unwrap());
            }
            TabKind::Log { state } => {
                contents_log(ui, context, state.lock().as_mut().unwrap());
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
        ui.push_id(ui.id().with(tab.name), |ui|{
            tab.kind.ui(ui, self.context);
        });
    }
}

struct TabContext<'a> {
    data: &'a mut Vec<RowType>,
    log_entries: &'a mut Vec<LogEntry>,
}

fn contents_inside_scroll_area(ui: &mut Ui, context: &mut TabContext, _state: &mut InsideScrollAreaState) {

    ui.label("content above scroll area");
    ui.separator();

    egui::ScrollArea::both()
        .max_height(200.0)
        .show(ui, |ui| {
            // FIXME the table renders on top of this
            ui.label("content above table, inside scroll area");

            let data_source = context.data.as_slice();

            let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
                .min_size((400.0, 400.0).into())
                .show(ui, &data_source, |builder: &mut DeferredTableBuilder<'_, &[RowType]>| {

                    builder.header(|header_builder| {

                        for (index, field) in futurama::fields().iter().enumerate() {
                            header_builder
                                .column(index, field.to_string());
                        }
                    })
                });

            for action in actions {
                match action {
                    Action::CellClicked(cell_index) => {
                        example_log(context.log_entries, Level::Info, format!("Cell clicked. cell: {:?}", cell_index))
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

    let data_source = context.data.as_slice();

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .show(ui, &data_source, |builder: &mut DeferredTableBuilder<'_, &[RowType]>| {
            builder.header(|header_builder| {
                for (index, field) in futurama::fields().iter().enumerate() {
                    header_builder
                        .column(index, field.to_string());
                }
            })
        });

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => {
                example_log(context.log_entries, Level::Info, format!("Cell clicked. cell: {:?}", cell_index))
            }
        }
    }
}

#[derive(Default)]
pub struct SimpleTableState {
    // here we could add state for table properties/presentation/etc.
}

fn contents_log(ui: &mut Ui, context: &mut TabContext, _state: &mut LogState) {

    let data_source = context.log_entries.as_slice();

    let (_response, actions) = DeferredTable::new(ui.make_persistent_id("table_1"))
        .show(ui, &data_source, |builder: &mut DeferredTableBuilder<'_, &[LogEntry]>| {
            builder.header(|header_builder| {
                for (index, field) in ["Time", "Level", "Message"].iter().enumerate() {
                    header_builder
                        .column(index, field.to_string());
                }
            })
        });

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => {
                example_log(context.log_entries, Level::Info, format!("Cell clicked. cell: {:?}", cell_index))
            }
        }
    }
}

#[derive(Default)]
pub struct LogState {
    // here would could add a filter, etc.
}

fn contents_spreadsheet(ui: &mut Ui, context: &mut TabContext, state: &mut SpreadsheetState) {

    let (_response, actions) = shared::spreadsheet::ui::show_table(ui, state);

    for action in actions {
        match action {
            Action::CellClicked(cell_index) => {
                example_log(context.log_entries, Level::Info, format!("Cell clicked. cell: {:?}", cell_index))
            }
        }
    }
}

fn contents_sparse_table(ui: &mut Ui, _context: &mut TabContext, state: &mut SparseTableState) {

    shared::sparse::ui::show_controls(ui, state);

    let (_response, actions) = shared::sparse::ui::show_table(ui, state);

    shared::sparse::ui::handle_actions(actions, state);
}
