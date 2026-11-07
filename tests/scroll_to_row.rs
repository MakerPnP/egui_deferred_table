mod common;

use common::{ROW_COUNT, TableState, is_shown, table_harness};
use egui::Align;

#[test]
fn a_row_below_the_table_is_scrolled_to_the_bottom() {
    // given
    let mut harness = table_harness(TableState::default());
    assert!(is_shown(&harness, 0));
    assert!(!is_shown(&harness, 80));

    // when
    harness.state_mut().scroll_to_row = Some((80, None));
    harness.run();

    // then
    assert!(is_shown(&harness, 80));
    // shown at the bottom, by scrolling as little as it can, so the row after it isn't shown.
    assert!(!is_shown(&harness, 81));
    assert!(!is_shown(&harness, 0));
}

#[test]
fn a_row_above_the_table_is_scrolled_to_the_top() {
    // given
    let mut harness = table_harness(TableState::default());
    harness.state_mut().scroll_to_row = Some((ROW_COUNT - 1, None));
    harness.run();
    assert!(!is_shown(&harness, 20));

    // when
    harness.state_mut().scroll_to_row = Some((20, None));
    harness.run();

    // then
    assert!(is_shown(&harness, 20));
    assert!(!is_shown(&harness, 19));
    assert!(!is_shown(&harness, ROW_COUNT - 1));
}

#[test]
fn a_row_shown_already_is_not_scrolled_to() {
    // given
    let mut harness = table_harness(TableState::default());
    assert!(is_shown(&harness, 3));

    // when
    harness.state_mut().scroll_to_row = Some((3, None));
    harness.run();

    // then
    assert!(is_shown(&harness, 0));
    assert!(is_shown(&harness, 3));
}

#[test]
fn a_row_is_scrolled_to_the_top_when_aligned_to_the_top() {
    // given
    let mut harness = table_harness(TableState::default());

    // when
    harness.state_mut().scroll_to_row = Some((50, Some(Align::Min)));
    harness.run();

    // then
    assert!(is_shown(&harness, 50));
    assert!(!is_shown(&harness, 49));
    assert!(is_shown(&harness, 51));
}

#[test]
fn a_row_is_found_by_its_data_index_when_the_rows_are_reordered() {
    // given
    // the rows in reverse, so row 0 is shown last.
    let mut state = TableState::default();
    state.renderer.row_ordering = Some((0..ROW_COUNT).rev().collect());
    let mut harness = table_harness(state);
    assert!(is_shown(&harness, ROW_COUNT - 1));
    assert!(!is_shown(&harness, 0));

    // when
    harness.state_mut().scroll_to_row = Some((0, None));
    harness.run();

    // then
    assert!(is_shown(&harness, 0));
    assert!(!is_shown(&harness, ROW_COUNT - 1));
}

#[test]
fn the_table_can_be_scrolled_away_from_the_row_afterwards() {
    // given
    let mut harness = table_harness(TableState::default());
    harness.state_mut().scroll_to_row = Some((80, None));
    harness.run();
    assert!(is_shown(&harness, 80));

    // when
    // the wheel scrolls up, a page at a time.
    harness.event(egui::Event::PointerMoved(egui::pos2(200.0, 150.0)));
    harness.step();
    for _ in 0..10 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, 100.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
    }
    harness.run_steps(10);

    // then
    assert!(is_shown(&harness, 0));
    assert!(!is_shown(&harness, 80));
}
