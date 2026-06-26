//! Windowless tab strip contract conformance tests.

use kinetik_ui_widgets::{FrameTab, PanelId, TabStrip, TabStripMove, TabStripTarget};

fn panel(raw: u64) -> PanelId {
    PanelId::from_raw(raw)
}

fn target(raw: u64, index: usize) -> TabStripTarget {
    TabStripTarget::new(panel(raw), index)
}

fn tab(raw: u64, title: &str, active: bool, close_visible: bool, draggable: bool) -> FrameTab {
    FrameTab {
        panel: panel(raw),
        title: title.to_owned(),
        active,
        close_visible,
        draggable,
    }
}

#[test]
fn tab_strip_preserves_frame_tab_order_and_active_identity() {
    let tabs = vec![
        tab(1, "Viewport", false, true, true),
        tab(2, "Inspector", true, false, true),
        tab(3, "Timeline", false, true, false),
    ];

    let strip = TabStrip::from_frame_tabs(tabs.clone());

    assert_eq!(strip.tabs(), tabs.as_slice());
    assert_eq!(strip.len(), 3);
    assert!(!strip.is_empty());
    assert_eq!(strip.active_index(), Some(1));
    assert_eq!(strip.active_panel(), Some(panel(2)));
    assert_eq!(
        strip.active_tab().map(|tab| tab.title.as_str()),
        Some("Inspector")
    );
}

#[test]
fn tab_strip_activation_targets_use_index_and_stable_panel_id() {
    let strip = TabStrip::from_tabs([
        tab(10, "Graph", false, true, true),
        tab(20, "Outliner", true, true, true),
        tab(30, "Console", false, true, true),
    ]);

    assert_eq!(strip.activation_target_by_index(1), Some(target(20, 1)));
    assert_eq!(
        strip.activation_target_by_panel(panel(30)),
        Some(target(30, 2))
    );
    assert_eq!(strip.focus_target_by_index(0), Some(target(10, 0)));
    assert_eq!(strip.focus_target_by_panel(panel(20)), Some(target(20, 1)));
    assert_eq!(strip.activation_target_by_index(99), None);
    assert_eq!(strip.activation_target_by_panel(panel(99)), None);
}

#[test]
fn tab_strip_movement_wraps_and_handles_empty_or_single_tab_strips() {
    let empty = TabStrip::new();
    assert_eq!(empty.previous_target(), None);
    assert_eq!(empty.next_target(), None);
    assert_eq!(empty.movement_target(TabStripMove::Next), None);

    let single = TabStrip::from_tabs([tab(7, "Only", true, true, true)]);
    assert_eq!(single.previous_target(), Some(target(7, 0)));
    assert_eq!(single.next_target(), Some(target(7, 0)));

    let first_active = TabStrip::from_tabs([
        tab(1, "First", true, true, true),
        tab(2, "Second", false, true, true),
        tab(3, "Third", false, true, true),
    ]);
    assert_eq!(first_active.previous_target(), Some(target(3, 2)));
    assert_eq!(first_active.next_target(), Some(target(2, 1)));

    let last_active = TabStrip::from_tabs([
        tab(1, "First", false, true, true),
        tab(2, "Second", false, true, true),
        tab(3, "Third", true, true, true),
    ]);
    assert_eq!(last_active.next_target(), Some(target(1, 0)));

    let no_active = TabStrip::from_tabs([
        tab(1, "First", false, true, true),
        tab(2, "Second", false, true, true),
    ]);
    assert_eq!(no_active.next_target(), Some(target(1, 0)));
    assert_eq!(no_active.previous_target(), Some(target(2, 1)));
}

#[test]
fn tab_strip_preserves_close_and_drag_affordance_metadata() {
    let strip = TabStrip::from_tabs([
        tab(1, "Locked", true, false, false),
        tab(2, "Editable", false, true, true),
    ]);

    assert_eq!(strip.tab(0).map(|tab| tab.close_visible), Some(false));
    assert_eq!(strip.tab(0).map(|tab| tab.draggable), Some(false));
    assert_eq!(
        strip.tab_by_panel(panel(2)).map(|tab| tab.title.as_str()),
        Some("Editable")
    );

    assert_eq!(strip.close_target_by_index(0), None);
    assert_eq!(strip.close_target_by_panel(panel(1)), None);
    assert_eq!(strip.close_target_by_index(1), Some(target(2, 1)));
    assert_eq!(strip.close_target_by_panel(panel(2)), Some(target(2, 1)));

    assert_eq!(strip.drag_target_by_index(0), None);
    assert_eq!(strip.drag_target_by_panel(panel(1)), None);
    assert_eq!(strip.drag_target_by_index(1), Some(target(2, 1)));
    assert_eq!(strip.drag_target_by_panel(panel(2)), Some(target(2, 1)));
}
