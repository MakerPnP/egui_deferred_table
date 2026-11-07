mod common;

use common::{TableState, cell_position, click_at, drag, row_header_position, table_harness};
use egui::Modifiers;
use std::collections::BTreeSet;

fn rows(rows: &[usize]) -> BTreeSet<usize> {
    rows.iter().copied().collect()
}

fn selecting_on_cell_click() -> TableState {
    TableState {
        select_rows_on_cell_click: true,
        ..TableState::default()
    }
}

#[test]
fn clicking_a_cell_selects_its_row() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());

    // when
    let position = cell_position(&harness, 2, 1);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(harness.state().last_selection_changed(), Some(&rows(&[2])));
}

#[test]
fn clicking_another_rows_cell_selects_only_that_row() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());
    let position = cell_position(&harness, 2, 0);
    click_at(&mut harness, position, Modifiers::NONE);

    // when
    let position = cell_position(&harness, 4, 1);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(harness.state().last_selection_changed(), Some(&rows(&[4])));
}

#[test]
fn clicking_a_cell_of_the_only_row_selected_changes_nothing() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());
    let position = cell_position(&harness, 2, 0);
    click_at(&mut harness, position, Modifiers::NONE);
    assert_eq!(harness.state().selection_changes(), 1);

    // when
    let position = cell_position(&harness, 2, 1);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(harness.state().selection_changes(), 1);
}

#[test]
fn clicking_a_cell_with_command_selects_or_deselects_its_row_keeping_the_others() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());
    let position = cell_position(&harness, 2, 0);
    click_at(&mut harness, position, Modifiers::NONE);

    // when
    let position = cell_position(&harness, 4, 0);
    click_at(&mut harness, position, Modifiers::COMMAND);

    // then
    assert_eq!(
        harness.state().last_selection_changed(),
        Some(&rows(&[2, 4]))
    );

    // and when
    let position = cell_position(&harness, 2, 1);
    click_at(&mut harness, position, Modifiers::COMMAND);

    // then
    assert_eq!(harness.state().last_selection_changed(), Some(&rows(&[4])));
}

#[test]
fn clicking_a_cell_selects_no_row_by_default() {
    // given
    let mut harness = table_harness(TableState::default());

    // when
    let position = cell_position(&harness, 2, 1);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(harness.state().selection_changes(), 0);
}

#[test]
fn clicking_a_cell_selects_no_row_when_rows_are_not_selectable() {
    // given
    let mut harness = table_harness(TableState {
        selectable_rows: false,
        ..selecting_on_cell_click()
    });

    // when
    let position = cell_position(&harness, 2, 1);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(harness.state().selection_changes(), 0);
}

#[test]
fn a_drag_that_ends_on_a_cell_selects_no_row() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());

    // when
    let from = cell_position(&harness, 2, 0);
    let to = cell_position(&harness, 5, 1);
    drag(&mut harness, from, to);

    // then
    assert_eq!(harness.state().selection_changes(), 0);
}

#[test]
fn clicking_a_rows_header_still_selects_or_deselects_it_keeping_the_others() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());
    let position = cell_position(&harness, 2, 0);
    click_at(&mut harness, position, Modifiers::NONE);

    // when
    let position = row_header_position(&harness, 4);
    click_at(&mut harness, position, Modifiers::NONE);

    // then
    assert_eq!(
        harness.state().last_selection_changed(),
        Some(&rows(&[2, 4]))
    );
}

#[test]
fn the_rows_selected_by_the_app_are_the_tables_selection() {
    // given
    let mut harness = table_harness(TableState {
        row_selection: Some(rows(&[3])),
        ..selecting_on_cell_click()
    });

    // when
    // the user adds to the rows the app selected.
    let position = cell_position(&harness, 5, 0);
    click_at(&mut harness, position, Modifiers::COMMAND);

    // then
    assert_eq!(
        harness.state().last_selection_changed(),
        Some(&rows(&[3, 5]))
    );
    assert_eq!(harness.state().row_selection, Some(rows(&[3, 5])));
}

#[test]
fn the_app_changing_its_rows_selected_is_not_a_selection_change() {
    // given
    let mut harness = table_harness(TableState {
        row_selection: Some(rows(&[3])),
        ..selecting_on_cell_click()
    });

    // when
    harness.state_mut().row_selection = Some(rows(&[1, 7]));
    harness.run();

    // then
    assert_eq!(harness.state().selection_changes(), 0);

    // and the table uses the rows the app changed them to.
    let position = cell_position(&harness, 7, 0);
    click_at(&mut harness, position, Modifiers::COMMAND);
    assert_eq!(harness.state().last_selection_changed(), Some(&rows(&[1])));
}

#[test]
fn the_rows_the_app_selected_that_are_not_in_the_data_source_are_removed() {
    // given
    let mut harness = table_harness(TableState {
        row_selection: Some(rows(&[3])),
        ..TableState::default()
    });

    // when
    harness.state_mut().row_selection = Some(rows(&[3, 99, 100, 150]));
    harness.run();

    // then
    assert_eq!(
        harness.state().last_selection_changed(),
        Some(&rows(&[3, 99]))
    );
    assert_eq!(harness.state().row_selection, Some(rows(&[3, 99])));
}

#[test]
fn the_rows_the_table_selected_that_are_no_longer_in_the_data_source_are_removed() {
    // given
    let mut harness = table_harness(selecting_on_cell_click());
    let position = cell_position(&harness, 5, 0);
    click_at(&mut harness, position, Modifiers::NONE);

    // when
    harness.state_mut().rows.row_count = 5;
    harness.run();

    // then
    assert_eq!(harness.state().last_selection_changed(), Some(&rows(&[])));
}
