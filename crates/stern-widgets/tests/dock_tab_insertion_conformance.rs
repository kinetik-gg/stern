//! Deterministic tab-strip insertion and local reordering conformance.
//!
//! Covers the pure resolver, the pure current-Dock validation query, and the
//! validated commit path: first/middle/end placements same-frame and
//! cross-frame, priority over generic edge-split targeting, no-op
//! suppression, active-tab preservation, dismissal-policy preservation,
//! stable anchor identity under concurrent edits, invalid-target refusal
//! without snapshot mutation, and snapshot round-tripping.
//!
//! Controller-level cancellation paths (Escape, capture loss, focus loss,
//! disablement, source removal, stale targets) live in
//! `dock_controller_conformance.rs`.

use stern_core::{Axis, Point, Rect};

use stern_widgets::dock::{
    Dock, DockDropTarget, DockNode, DockTabDrag, DockTabSlotGeometry, DockTabStripGeometry,
    DockTabStripTarget, Frame, FrameId, Panel, PanelId, dock_tab_slot_anchor,
    dock_tab_strip_contains_point, resolve_dock_drop_target, resolve_dock_tab_strip_target,
    solve_dock_layout,
};

const STRIP_HEIGHT: f32 = 28.0;
const TAB_WIDTH: f32 = 100.0;
const FRAME_WIDTH: f32 = 300.0;

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}

fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

/// Two frames split horizontally: frame 1 holds tabs 11..13 with 12 active;
/// frame 2 holds tabs 21..22.
fn two_frame_dock() -> Dock {
    let mut dock = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(
            1,
            vec![
                panel(11, "Assets"),
                panel(12, "Inspector"),
                panel(13, "Details"),
            ],
        ))),
        second: Box::new(DockNode::Frame(frame(
            2,
            vec![panel(21, "Viewport"), panel(22, "Timeline")],
        ))),
    });
    assert!(dock.select_panel(FrameId::from_raw(1), PanelId::from_raw(12)));
    dock
}

/// Frame `id` occupies `x..x+300` with one `TAB_WIDTH` tab per panel inside a
/// `STRIP_HEIGHT` strip at the top of the frame.
#[allow(clippy::cast_precision_loss)]
fn strip(id: u64, x: f32, panels: &[u64]) -> DockTabStripGeometry {
    DockTabStripGeometry {
        frame: FrameId::from_raw(id),
        rect: Rect::new(x, 0.0, FRAME_WIDTH, 400.0),
        tab_list_rect: Rect::new(x, 0.0, FRAME_WIDTH, STRIP_HEIGHT),
        tabs: panels
            .iter()
            .enumerate()
            .map(|(index, panel)| DockTabSlotGeometry {
                panel: PanelId::from_raw(*panel),
                rect: Rect::new(x + TAB_WIDTH * index as f32, 0.0, TAB_WIDTH, STRIP_HEIGHT),
            })
            .collect(),
    }
}

fn strips() -> Vec<DockTabStripGeometry> {
    vec![strip(1, 0.0, &[11, 12, 13]), strip(2, 300.0, &[21, 22])]
}

fn drag(source_frame: u64, panel: u64) -> DockTabDrag {
    DockTabDrag {
        source_frame: FrameId::from_raw(source_frame),
        panel: PanelId::from_raw(panel),
    }
}

fn point(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn order(dock: &Dock, frame_id: u64) -> Vec<u64> {
    dock.frame(FrameId::from_raw(frame_id))
        .expect("frame")
        .panels
        .iter()
        .map(|panel| panel.id.raw())
        .collect()
}

fn active_panel(dock: &Dock, frame_id: u64) -> Option<u64> {
    dock.frame(FrameId::from_raw(frame_id))
        .and_then(Frame::active_panel)
        .map(|panel| panel.id.raw())
}

#[test]
fn reorder_resolves_first_middle_end_and_suppresses_no_ops() {
    let layout = strips();
    let move_inspector = drag(1, 12);

    // Left of every remaining center resolves the front slot.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(40.0, 14.0), move_inspector),
        Some(DockTabStripTarget::Reorder {
            frame: FrameId::from_raw(1),
            index: 0,
        })
    );
    // Hovering the dragged tab's own neighborhood is a no-op.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(150.0, 14.0), move_inspector),
        None
    );
    // Right of the last remaining center appends to the end slot.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(280.0, 14.0), move_inspector),
        Some(DockTabStripTarget::Reorder {
            frame: FrameId::from_raw(1),
            index: 2,
        })
    );
}

#[test]
fn insert_resolves_hovered_anchors_and_appends_past_the_last_tab() {
    let layout = strips();
    let move_assets = drag(1, 11);

    // Hovering the first target tab anchors before it.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(350.0, 14.0), move_assets),
        Some(DockTabStripTarget::Insert {
            frame: FrameId::from_raw(2),
            anchor: Some(PanelId::from_raw(21)),
        })
    );
    // Hovering the second tab anchors before it.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(450.0, 14.0), move_assets),
        Some(DockTabStripTarget::Insert {
            frame: FrameId::from_raw(2),
            anchor: Some(PanelId::from_raw(22)),
        })
    );
    // The empty tail of the strip past both tab rects appends.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout, point(580.0, 14.0), move_assets),
        Some(DockTabStripTarget::Insert {
            frame: FrameId::from_raw(2),
            anchor: None,
        })
    );
}

#[test]
fn resolver_scans_strips_in_slice_order_and_rejects_invalid_geometry() {
    // A later overlapping strip never wins: slice order decides ties.
    let mut overlapping = strips();
    overlapping.push(strip(3, 0.0, &[31]));
    assert_eq!(
        resolve_dock_tab_strip_target(&overlapping, point(10.0, 10.0), drag(3, 31)),
        Some(DockTabStripTarget::Insert {
            frame: FrameId::from_raw(1),
            anchor: Some(PanelId::from_raw(11)),
        })
    );

    for pointer in [
        point(f32::NAN, 14.0),
        point(f32::INFINITY, 14.0),
        point(150.0, f32::NEG_INFINITY),
    ] {
        assert_eq!(
            resolve_dock_tab_strip_target(&strips(), pointer, drag(1, 12)),
            None
        );
        assert!(!dock_tab_strip_contains_point(&strips(), pointer));
    }

    // A stale strip that lost the dragged panel never affirms a reorder.
    let mut stale = strips();
    stale[0]
        .tabs
        .retain(|slot| slot.panel != PanelId::from_raw(12));
    assert_eq!(
        resolve_dock_tab_strip_target(&stale, point(20.0, 10.0), drag(1, 12)),
        None
    );
}

#[test]
fn strip_surface_gate_distinguishes_idle_strips_from_outside_points() {
    let layout = strips();

    // Inside any strip surface: fall-through into split targeting is barred
    // even when the placement itself is idle.
    assert!(dock_tab_strip_contains_point(&layout, point(150.0, 14.0)));
    assert!(dock_tab_strip_contains_point(&layout, point(590.0, 27.9)));
    // Below the strip or outside every frame: generic targeting may run.
    assert!(!dock_tab_strip_contains_point(&layout, point(150.0, 200.0)));
    assert!(!dock_tab_strip_contains_point(&layout, point(700.0, 14.0)));
}

#[test]
fn slot_anchor_maps_front_middle_append_and_rejects_out_of_range() {
    let remaining = [
        PanelId::from_raw(11),
        PanelId::from_raw(12),
        PanelId::from_raw(13),
    ];

    assert_eq!(
        dock_tab_slot_anchor(&remaining, 0),
        Some(Some(PanelId::from_raw(11)))
    );
    assert_eq!(
        dock_tab_slot_anchor(&remaining, 1),
        Some(Some(PanelId::from_raw(12)))
    );
    assert_eq!(
        dock_tab_slot_anchor(&remaining, 3),
        Some(None),
        "the final slot appends"
    );
    assert_eq!(
        dock_tab_slot_anchor(&remaining, 4),
        None,
        "out-of-range slots are invalid, not appends"
    );
}

#[test]
fn validation_accepts_moving_placements_and_rejects_no_ops_and_missing_state() {
    let area = two_frame_dock();
    let frame1 = FrameId::from_raw(1);
    let frame2 = FrameId::from_raw(2);
    let inspector = drag(1, 12);

    // Same-frame placements that change the order are current.
    assert!(area.tab_insertion_is_current(inspector, frame1, Some(PanelId::from_raw(11))));
    assert!(area.tab_insertion_is_current(inspector, frame1, None));

    // Same-frame no-ops are rejected: anchoring on the panel right after the
    // dragged tab reproduces the current order, and so does appending when
    // the dragged tab is already last.
    assert!(!area.tab_insertion_is_current(inspector, frame1, Some(PanelId::from_raw(13))));

    // Cross-frame placements require existing frames and anchors only.
    assert!(area.tab_insertion_is_current(inspector, frame2, Some(PanelId::from_raw(21))));
    assert!(area.tab_insertion_is_current(inspector, frame2, Some(PanelId::from_raw(22))));
    assert!(area.tab_insertion_is_current(inspector, frame2, None));
    assert!(!area.tab_insertion_is_current(inspector, frame2, Some(PanelId::from_raw(99))));
    assert!(!area.tab_insertion_is_current(inspector, FrameId::from_raw(99), None));

    // The source frame must own the dragged panel.
    assert!(!area.tab_insertion_is_current(drag(1, 99), frame2, None));
    // The reverse direction is structurally valid.
    assert!(area.tab_insertion_is_current(drag(2, 21), frame1, Some(PanelId::from_raw(11))));
}

#[test]
fn validation_rejects_empty_target_frames() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(11, "Assets")]))),
        second: Box::new(DockNode::Frame(frame(2, Vec::new()))),
    });

    assert!(!area.tab_insertion_is_current(drag(1, 11), FrameId::from_raw(2), None,));
    assert_eq!(order(&area, 1), vec![11]);
}

#[test]
fn reorder_commits_first_middle_and_end_slots_without_disturbing_active_tabs() {
    let mut area = two_frame_dock();
    let frame1 = FrameId::from_raw(1);

    // Front slot: the dragged tab keeps the selection it had when active.
    assert!(area.apply_tab_insertion(drag(1, 12), frame1, Some(PanelId::from_raw(11))));
    assert_eq!(order(&area, 1), vec![12, 11, 13]);
    assert_eq!(active_panel(&area, 1), Some(12));

    // End slot from a fresh fixture: active identity survives the shift.
    let mut area = two_frame_dock();
    assert!(area.apply_tab_insertion(drag(1, 12), frame1, None));
    assert_eq!(order(&area, 1), vec![11, 13, 12]);
    assert_eq!(active_panel(&area, 1), Some(12));

    // Moving a background tab leaves the active tab exactly where it was.
    let mut area = two_frame_dock();
    assert!(area.select_panel(frame1, PanelId::from_raw(13)));
    assert!(area.apply_tab_insertion(drag(1, 11), frame1, Some(PanelId::from_raw(13))));
    assert_eq!(order(&area, 1), vec![12, 11, 13]);
    assert_eq!(active_panel(&area, 1), Some(13));
}

#[test]
fn cross_frame_insert_commits_exact_anchor_positions_and_prunes_empty_sources() {
    let mut area = two_frame_dock();

    // Insert before the second target tab.
    assert!(area.apply_tab_insertion(
        drag(1, 12),
        FrameId::from_raw(2),
        Some(PanelId::from_raw(22))
    ));
    assert_eq!(order(&area, 1), vec![11, 13]);
    assert_eq!(order(&area, 2), vec![21, 12, 22]);
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    assert_eq!(active_panel(&area, 2), Some(12));
    // The source frame's selection clamps deterministically.
    assert_eq!(active_panel(&area, 1), Some(13));

    // Append from a fresh fixture.
    let mut area = two_frame_dock();
    assert!(area.apply_tab_insertion(drag(1, 13), FrameId::from_raw(2), None));
    assert_eq!(order(&area, 2), vec![21, 22, 13]);

    // Emptied source frames are pruned like every other docking move.
    let mut single = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(11, "Only")]))),
        second: Box::new(DockNode::Frame(frame(
            2,
            vec![panel(21, "Viewport"), panel(22, "Timeline")],
        ))),
    });
    assert!(single.apply_tab_insertion(
        drag(1, 11),
        FrameId::from_raw(2),
        Some(PanelId::from_raw(22))
    ));
    assert!(single.frame(FrameId::from_raw(1)).is_none());
    assert_eq!(single.frames().len(), 1);
    assert_eq!(order(&single, 2), vec![21, 11, 22]);
}

#[test]
fn dismissal_policy_survives_reorders_and_cross_frame_insertions() {
    let mut area = two_frame_dock();
    assert!(
        area.frame_mut(FrameId::from_raw(1))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(12), false)
    );
    let before_dismissible = area
        .frame(FrameId::from_raw(1))
        .expect("frame")
        .panel_dismissible(PanelId::from_raw(12));

    // Local reorder keeps the per-panel policy.
    assert!(area.apply_tab_insertion(
        drag(1, 12),
        FrameId::from_raw(1),
        Some(PanelId::from_raw(11))
    ));
    assert_eq!(
        area.frame(FrameId::from_raw(1))
            .expect("frame")
            .panel_dismissible(PanelId::from_raw(12)),
        before_dismissible
    );

    // Cross-frame insertion carries the policy into the target frame.
    let mut area = two_frame_dock();
    assert!(
        area.frame_mut(FrameId::from_raw(1))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(12), false)
    );
    assert!(area.apply_tab_insertion(
        drag(1, 12),
        FrameId::from_raw(2),
        Some(PanelId::from_raw(22))
    ));
    assert!(
        !area
            .frame(FrameId::from_raw(2))
            .expect("target")
            .panel_dismissible(PanelId::from_raw(12))
    );
}

#[test]
fn anchors_stay_stable_when_the_target_order_shifts_between_preview_and_commit() {
    let mut area = two_frame_dock();
    let target = (FrameId::from_raw(2), Some(PanelId::from_raw(22)));

    // Preview time: the anchor-keyed target validates against the live tree.
    assert!(area.tab_insertion_is_current(drag(1, 12), target.0, target.1));

    // A concurrent edit inserts another panel before the anchor.
    area.frame_mut(FrameId::from_raw(2))
        .expect("target")
        .panels
        .insert(1, panel(15, "Late arrival"));

    // The commit still lands immediately before the same anchor panel.
    assert!(area.apply_tab_insertion(drag(1, 12), target.0, target.1));
    assert_eq!(order(&area, 2), vec![21, 15, 12, 22]);
}

#[test]
fn invalid_and_stale_commit_attempts_never_mutate_the_snapshot() {
    let mut area = two_frame_dock();
    let before = area.snapshot();

    // No-op same-frame placement.
    assert!(!area.apply_tab_insertion(
        drag(1, 12),
        FrameId::from_raw(1),
        Some(PanelId::from_raw(13))
    ));
    assert_eq!(area.snapshot(), before);

    // Missing target frame.
    assert!(!area.apply_tab_insertion(drag(1, 12), FrameId::from_raw(99), None));
    assert_eq!(area.snapshot(), before);

    // Source removal between preview and release.
    area.frame_mut(FrameId::from_raw(1))
        .expect("source")
        .remove_panel(PanelId::from_raw(12));
    let removed = area.snapshot();
    assert!(!area.apply_tab_insertion(drag(1, 12), FrameId::from_raw(2), None));
    assert_eq!(area.snapshot(), removed);
}

#[test]
fn drop_tab_routes_insert_targets_through_the_same_validation() {
    let mut area = two_frame_dock();

    assert!(area.drop_tab(
        drag(1, 12),
        DockDropTarget::Insert {
            frame: FrameId::from_raw(2),
            anchor: Some(PanelId::from_raw(21)),
        },
    ));
    assert_eq!(order(&area, 2), vec![12, 21, 22]);

    // A drag whose source frame does not own the panel is refused.
    let committed = area.snapshot();
    assert!(!area.drop_tab(
        drag(1, 12),
        DockDropTarget::Insert {
            frame: FrameId::from_raw(2),
            anchor: None,
        },
    ));
    assert_eq!(area.snapshot(), committed);
}

#[test]
fn tab_strip_targeting_wins_over_generic_edge_split_resolution() {
    let area = two_frame_dock();
    let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);
    let layout = solve_dock_layout(&area, bounds);
    let layout_strips = strips();
    let corner = point(2.0, 14.0);

    // The generic resolver alone would read the corner as a left-edge split.
    let generic = resolve_dock_drop_target(&layout, corner, FrameId::from_raw(90))
        .expect("generic edge target");
    assert!(matches!(generic, DockDropTarget::Split { .. }));

    // The strip resolver reads the same point as a precise reorder, and the
    // controller consults it first, so the split never fires there.
    assert_eq!(
        resolve_dock_tab_strip_target(&layout_strips, corner, drag(1, 12)),
        Some(DockTabStripTarget::Reorder {
            frame: FrameId::from_raw(1),
            index: 0,
        })
    );
}

#[test]
fn inserted_orders_round_trip_through_snapshots() {
    let mut area = two_frame_dock();
    assert!(area.apply_tab_insertion(
        drag(1, 12),
        FrameId::from_raw(1),
        Some(PanelId::from_raw(11))
    ));
    assert!(area.apply_tab_insertion(
        drag(1, 13),
        FrameId::from_raw(2),
        Some(PanelId::from_raw(21))
    ));
    let snapshot = area.snapshot();

    let restored = Dock::restore(snapshot.clone()).expect("valid snapshot");
    assert_eq!(restored.snapshot(), snapshot);
    assert_eq!(restored.active_frame(), area.active_frame());
    assert_eq!(order(&restored, 1), order(&area, 1));
    assert_eq!(order(&restored, 2), order(&area, 2));
}
