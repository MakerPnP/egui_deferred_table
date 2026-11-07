mod common;

use common::{ROW_COUNT, TableState, WINDOW_HEIGHT, WINDOW_WIDTH, cell_label, table_harness};
use egui::Id;
use egui::accesskit::Role;
use egui_deferred_table::DeferredTable;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};

/// A table with text above it ends within its parent, the window, so its last row can be scrolled to and shown
/// whole, above the horizontal scroll bar.  It used to take the parent's whole height, from below the text, so it
/// overflowed the window by the text's height, and its last rows could never be shown.
#[test]
fn a_table_below_other_content_ends_within_its_parent() {
    // given
    let mut harness = table_harness(TableState {
        lines_above: 5,
        ..TableState::default()
    });
    let text_bottom = harness
        .get_by_role_and_label(Role::Label, "above the table 4")
        .rect()
        .bottom();

    // when
    harness.state_mut().scroll_to_row = Some((ROW_COUNT - 1, None));
    harness.run();

    // then
    let last_row = harness
        .get_by_role_and_label(Role::Label, &cell_label(ROW_COUNT - 1, 0))
        .rect();
    // the table's scroll bars are always shown, and solid.
    let scroll = egui::style::ScrollStyle::solid();
    let scroll_bar_size = scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin;
    assert!(
        last_row.bottom() <= WINDOW_HEIGHT - scroll_bar_size,
        "the last row ends at {}, below the window's bottom, {}, less the scroll bar's size, {}",
        last_row.bottom(),
        WINDOW_HEIGHT,
        scroll_bar_size,
    );

    // and the table is still below the text.
    let rows_top = harness
        .query_all(By::new().role(Role::Label))
        .filter(|label| {
            label
                .accesskit_node()
                .label()
                .is_some_and(|label| label.starts_with("row "))
        })
        .map(|label| label.rect().top())
        .fold(f32::MAX, f32::min);
    assert!(rows_top > text_bottom);
}

/// A table in a scroll area, below some text, as in the 'docking' demo's "Inside scroll area" tab, is shown below the
/// text, and the text below it is shown below it.
#[test]
fn a_table_in_a_scroll_area_is_between_the_content_above_and_below_it() {
    // given
    let mut harness = Harness::builder()
        .with_size((WINDOW_WIDTH, WINDOW_HEIGHT))
        .build_ui_state(
            |ui, state: &mut TableState| {
                egui::ScrollArea::both().max_height(200.0).show(ui, |ui| {
                    ui.label("above the table");
                    DeferredTable::new(Id::new("table"))
                        .min_size((400.0, 400.0).into())
                        .show(ui, &mut state.rows, &mut state.renderer);
                    ui.label("below the table");
                });
            },
            TableState::default(),
        );

    // when
    harness.run();

    // then
    let above = harness
        .get_by_role_and_label(Role::Label, "above the table")
        .rect();
    let first_row = harness
        .get_by_role_and_label(Role::Label, &cell_label(0, 0))
        .rect();
    assert!(
        first_row.top() > above.bottom(),
        "the first row's top, {}, is above the text above the table, which ends at {}",
        first_row.top(),
        above.bottom(),
    );
    let below = harness
        .get_by_role_and_label(Role::Label, "below the table")
        .rect();
    assert!(below.top() > first_row.bottom());
}
