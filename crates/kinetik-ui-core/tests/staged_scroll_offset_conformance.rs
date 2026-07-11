//! Next-frame scroll-offset staging conformance.

use kinetik_ui_core::{UiMemory, UiTestHarness, Vec2, WidgetId};

#[test]
fn staged_offset_is_invisible_until_frame_end() {
    let owner = WidgetId::from_key("staged-owner");
    let mut harness = UiTestHarness::new();

    let (during, _) = harness.run_frame(|ui| {
        ui.stage_scroll_offset(owner, Vec2::new(12.0, 24.0));
        ui.memory().scroll_offset(owner)
    });

    assert_eq!(during, Vec2::ZERO);
    assert_eq!(harness.memory().scroll_offset(owner), Vec2::new(12.0, 24.0));
}

#[test]
fn last_staged_write_wins_per_owner() {
    let owner = WidgetId::from_key("last-write-owner");
    let mut harness = UiTestHarness::new();

    let _ = harness.run_frame(|ui| {
        ui.stage_scroll_offset(owner, Vec2::new(1.0, 2.0));
        ui.stage_scroll_offset(owner, Vec2::new(3.0, 4.0));
        ui.stage_scroll_offset(owner, Vec2::new(5.0, 6.0));
        assert_eq!(ui.memory().scroll_offset(owner), Vec2::ZERO);
    });

    assert_eq!(harness.memory().scroll_offset(owner), Vec2::new(5.0, 6.0));
}

#[test]
fn staged_offsets_are_independent_between_owners() {
    let first = WidgetId::from_key("first-owner");
    let second = WidgetId::from_key("second-owner");
    let mut harness = UiTestHarness::new();
    harness
        .memory_mut()
        .set_scroll_offset(first, Vec2::new(2.0, 3.0));
    harness
        .memory_mut()
        .set_scroll_offset(second, Vec2::new(4.0, 5.0));

    let _ = harness.run_frame(|ui| {
        ui.stage_scroll_offset(first, Vec2::new(10.0, 11.0));
        ui.stage_scroll_offset(second, Vec2::new(20.0, 21.0));
        assert_eq!(ui.memory().scroll_offset(first), Vec2::new(2.0, 3.0));
        assert_eq!(ui.memory().scroll_offset(second), Vec2::new(4.0, 5.0));
    });

    assert_eq!(harness.memory().scroll_offset(first), Vec2::new(10.0, 11.0));
    assert_eq!(
        harness.memory().scroll_offset(second),
        Vec2::new(20.0, 21.0)
    );
}

#[test]
fn immediate_set_cancels_only_that_owners_pending_write() {
    let first = WidgetId::from_key("immediate-first");
    let second = WidgetId::from_key("immediate-second");
    let mut memory = UiMemory::new();

    memory.stage_scroll_offset(first, Vec2::new(10.0, 11.0));
    memory.stage_scroll_offset(second, Vec2::new(20.0, 21.0));
    memory.set_scroll_offset(first, Vec2::new(30.0, 31.0));

    assert_eq!(memory.scroll_offset(first), Vec2::new(30.0, 31.0));
    assert_eq!(memory.scroll_offset(second), Vec2::ZERO);

    let mut harness = UiTestHarness::new();
    *harness.memory_mut() = memory;
    let _ = harness.run_frame(|_| {});

    assert_eq!(harness.memory().scroll_offset(first), Vec2::new(30.0, 31.0));
    assert_eq!(
        harness.memory().scroll_offset(second),
        Vec2::new(20.0, 21.0)
    );
}
