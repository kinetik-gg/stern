use super::{
    Dock, DockDropTarget, DockDropZone, DockNode, DockPathElement, DockPlacement,
    DockRestoreError, DockSnapshot, DockSnapshotNode, DockSplitInsertion, DockSplitPath, Frame,
    FrameId, FrameLayout, Panel, PanelId, frame_tabs, resolve_dock_drop_target,
    resolve_frame_drop_zone, solve_dock_layout, solve_dock_splitters, split_ratio_from_drag,
};
use kinetik_ui_core::{Axis, Point, Rect, SemanticRole, Vec2};

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}
fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

fn dock() -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.25,
        min_first: 100.0,
        min_second: 100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Media")]))),
        second: Box::new(DockNode::Frame(frame(
            2,
            vec![panel(2, "Viewport"), panel(3, "Timeline")],
        ))),
    })
}

#[test]
fn dock_tree_visits_frames_in_order() {
    let area = dock();
    let frames = area.frames();

    assert_eq!(frames[0].id, FrameId::from_raw(1));
    assert_eq!(frames[1].id, FrameId::from_raw(2));
}

#[test]
fn dock_initializes_active_frame_from_tree_order() {
    let area = dock();

    assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
}

#[test]
fn set_active_frame_requires_existing_frame() {
    let mut area = dock();

    assert!(area.set_active_frame(FrameId::from_raw(2)));
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    assert!(!area.set_active_frame(FrameId::from_raw(99)));
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
}

#[test]
fn selects_and_removes_frame_tabs() {
    let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);

    assert!(frame.select_panel(PanelId::from_raw(2)));
    assert_eq!(
        frame.active_panel().expect("active").id,
        PanelId::from_raw(2)
    );
    assert_eq!(
        frame
            .remove_panel(PanelId::from_raw(2))
            .expect("removed")
            .title,
        "B"
    );
    assert_eq!(frame.active, 0);
}

#[test]
fn moves_panels_between_frames() {
    let mut area = dock();

    assert!(area.move_panel(
        FrameId::from_raw(2),
        FrameId::from_raw(1),
        PanelId::from_raw(3)
    ));

    assert_eq!(
        area.frame_mut(FrameId::from_raw(1))
            .expect("frame")
            .panels
            .len(),
        2
    );
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
}

#[test]
fn moving_panels_preserves_frame_owned_dismissal_policy() {
    let mut area = dock();
    area.frame_mut(FrameId::from_raw(2))
        .expect("source")
        .set_panel_dismissible(PanelId::from_raw(3), false);

    assert!(area.move_panel(
        FrameId::from_raw(2),
        FrameId::from_raw(1),
        PanelId::from_raw(3)
    ));

    assert!(
        !area
            .frame(FrameId::from_raw(1))
            .expect("target")
            .panel_dismissible(PanelId::from_raw(3))
    );
}

#[test]
fn moving_panel_to_missing_target_does_not_remove_it() {
    let mut area = dock();

    assert!(!area.move_panel(
        FrameId::from_raw(2),
        FrameId::from_raw(99),
        PanelId::from_raw(3)
    ));

    let source = area.frame(FrameId::from_raw(2)).expect("source");
    assert_eq!(source.panels.len(), 2);
    assert!(
        source
            .panels
            .iter()
            .any(|panel| panel.id == PanelId::from_raw(3))
    );
}

#[test]
fn moving_last_panel_prunes_empty_source_frame() {
    let mut area = dock();
    assert!(area.set_active_frame(FrameId::from_raw(1)));

    assert!(area.move_panel(
        FrameId::from_raw(1),
        FrameId::from_raw(2),
        PanelId::from_raw(1)
    ));

    assert_eq!(area.frames().len(), 1);
    assert!(area.frame(FrameId::from_raw(1)).is_none());
    assert_eq!(
        area.frame(FrameId::from_raw(2))
            .expect("target")
            .panels
            .len(),
        3
    );
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
}

#[test]
fn merges_frames_into_target() {
    let mut area = dock();
    assert!(area.set_active_frame(FrameId::from_raw(1)));

    assert!(area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(2)));

    assert_eq!(
        area.frame_mut(FrameId::from_raw(2))
            .expect("target")
            .panels
            .len(),
        3
    );
    assert_eq!(area.frames().len(), 1);
    assert!(area.frame(FrameId::from_raw(1)).is_none());
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
}

#[test]
fn merging_missing_target_does_not_remove_source_panels() {
    let mut area = dock();

    assert!(!area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(99)));

    assert_eq!(
        area.frame(FrameId::from_raw(1))
            .expect("source")
            .panels
            .len(),
        1
    );
}

#[test]
fn selecting_panel_updates_active_frame() {
    let mut area = dock();

    assert!(area.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));

    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    assert_eq!(
        area.frame(FrameId::from_raw(2))
            .expect("frame")
            .active_panel()
            .expect("active")
            .id,
        PanelId::from_raw(3)
    );
}

#[test]
fn solves_horizontal_split_layout() {
    let area = dock();
    let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));

    assert_eq!(layout.len(), 2);
    assert!((layout[0].rect.width - 250.0).abs() < f32::EPSILON);
    assert!((layout[1].rect.x - 250.0).abs() < f32::EPSILON);
}

#[test]
fn split_layout_respects_minimums() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.05,
        min_first: 100.0,
        min_second: 100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
    });
    let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 500.0, 200.0));

    assert!((layout[0].rect.width - 100.0).abs() < f32::EPSILON);
}

#[test]
fn split_layout_never_emits_negative_sizes_when_minimums_exceed_bounds() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 100.0,
        min_second: 100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
    });
    let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

    assert_eq!(layout.len(), 2);
    assert!(layout[0].rect.width >= 0.0);
    assert!(layout[1].rect.width >= 0.0);
    assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
}

#[test]
fn split_layout_sanitizes_direct_non_finite_values() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: f32::NAN,
        min_first: f32::INFINITY,
        min_second: -100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
    });
    let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

    assert_eq!(layout.len(), 2);
    assert!(layout[0].rect.width.is_finite());
    assert!(layout[1].rect.width.is_finite());
    assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
}

#[test]
fn splitter_drag_maps_delta_to_clamped_ratio() {
    let bounds = Rect::new(0.0, 0.0, 500.0, 200.0);

    let ratio = split_ratio_from_drag(
        Axis::Horizontal,
        bounds,
        0.5,
        100.0,
        100.0,
        Vec2::new(75.0, 0.0),
    );
    assert!((ratio - 0.65).abs() < f32::EPSILON);

    let clamped = split_ratio_from_drag(
        Axis::Horizontal,
        bounds,
        0.5,
        100.0,
        100.0,
        Vec2::new(-500.0, 0.0),
    );
    assert!((clamped - 0.2).abs() < f32::EPSILON);

    let vertical = split_ratio_from_drag(
        Axis::Vertical,
        bounds,
        0.5,
        50.0,
        50.0,
        Vec2::new(1000.0, 25.0),
    );
    assert!((vertical - 0.625).abs() < f32::EPSILON);
}

#[test]
fn dock_resizes_root_and_nested_splits() {
    let mut area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 100.0,
        min_second: 100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.5,
            min_first: 50.0,
            min_second: 50.0,
            first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
            second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
        }),
    });
    let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);

    assert!(area.resize_split(&DockSplitPath::root(), bounds, Vec2::new(60.0, 0.0)));
    assert!(area.resize_split(
        &DockSplitPath::new([DockPathElement::Second]),
        bounds,
        Vec2::new(0.0, -75.0)
    ));
    let layout = solve_dock_layout(&area, bounds);

    assert!((layout[0].rect.width - 360.0).abs() < f32::EPSILON);
    assert!((layout[1].rect.height - 125.0).abs() < f32::EPSILON);
}

#[test]
fn dock_splitters_expose_paths_and_hit_rects() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 100.0,
        min_second: 100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.25,
            min_first: 50.0,
            min_second: 50.0,
            first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
            second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
        }),
    });

    let splitters = solve_dock_splitters(&area, Rect::new(0.0, 0.0, 600.0, 400.0), 8.0);

    assert_eq!(splitters.len(), 2);
    assert_eq!(splitters[0].path, DockSplitPath::root());
    assert_eq!(splitters[0].rect, Rect::new(296.0, 0.0, 8.0, 400.0));
    assert_eq!(
        splitters[1].path,
        DockSplitPath::new([DockPathElement::Second])
    );
    assert_eq!(splitters[1].axis, Axis::Vertical);
}

#[test]
fn dock_splitters_sanitize_non_finite_geometry() {
    let area = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: f32::NAN,
        min_first: f32::INFINITY,
        min_second: -100.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
    });

    let splitters = solve_dock_splitters(
        &area,
        Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, f32::NAN),
        f32::NAN,
    );

    assert_eq!(splitters.len(), 1);
    assert!(splitters[0].rect.x.is_finite());
    assert!(splitters[0].rect.y.is_finite());
    assert!(splitters[0].rect.width.is_finite());
    assert!(splitters[0].rect.height.is_finite());
    assert!(splitters[0].ratio.is_finite());
    assert!(splitters[0].min_first.is_finite());
    assert!(splitters[0].min_second.is_finite());
}

#[test]
fn frame_tabs_expose_presentation_state() {
    let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);
    frame.select_panel(PanelId::from_raw(2));
    assert!(frame.set_panel_dismissible(PanelId::from_raw(1), false));

    let tabs = frame_tabs(&frame);

    assert!(!tabs[0].active);
    assert!(!tabs[0].close_visible);
    assert!(tabs[1].active);
    assert!(tabs[1].close_visible);
    assert!(tabs[1].draggable);
}

#[test]
fn tab_drag_starts_only_for_panels_owned_by_the_frame() {
    let area = dock();

    assert_eq!(
        area.begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3)),
        Some(super::DockTabDrag {
            source_frame: FrameId::from_raw(2),
            panel: PanelId::from_raw(3),
        })
    );
    assert_eq!(
        area.begin_tab_drag(FrameId::from_raw(1), PanelId::from_raw(3)),
        None
    );
}

#[test]
fn drop_zone_resolution_prefers_edges_over_center() {
    let rect = Rect::new(10.0, 20.0, 200.0, 100.0);

    assert_eq!(
        resolve_frame_drop_zone(rect, Point::new(12.0, 70.0)),
        Some(DockDropZone::Left)
    );
    assert_eq!(
        resolve_frame_drop_zone(rect, Point::new(205.0, 70.0)),
        Some(DockDropZone::Right)
    );
    assert_eq!(
        resolve_frame_drop_zone(rect, Point::new(100.0, 23.0)),
        Some(DockDropZone::Top)
    );
    assert_eq!(
        resolve_frame_drop_zone(rect, Point::new(100.0, 116.0)),
        Some(DockDropZone::Bottom)
    );
    assert_eq!(
        resolve_frame_drop_zone(rect, Point::new(100.0, 70.0)),
        Some(DockDropZone::Center)
    );
    assert_eq!(resolve_frame_drop_zone(rect, Point::new(0.0, 0.0)), None);
}

#[test]
fn drop_zone_resolution_rejects_invalid_geometry() {
    let rect = Rect::new(10.0, 20.0, 200.0, 100.0);
    for point in [
        Point::new(f32::NAN, 70.0),
        Point::new(f32::INFINITY, 70.0),
        Point::new(100.0, f32::NEG_INFINITY),
    ] {
        assert_eq!(resolve_frame_drop_zone(rect, point), None);
    }

    for rect in [
        Rect::new(f32::NAN, 20.0, 200.0, 100.0),
        Rect::new(10.0, f32::INFINITY, 200.0, 100.0),
        Rect::new(10.0, 20.0, f32::INFINITY, 100.0),
        Rect::new(10.0, 20.0, 200.0, f32::NAN),
        Rect::new(10.0, 20.0, 0.0, 100.0),
        Rect::new(10.0, 20.0, 200.0, -1.0),
    ] {
        assert_eq!(resolve_frame_drop_zone(rect, Point::new(100.0, 70.0)), None);
    }
}

#[test]
fn dock_drop_target_resolution_returns_merge_or_split_targets() {
    let area = dock();
    let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);

    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(500.0, 250.0), new_frame),
        Some(DockDropTarget::tab(FrameId::from_raw(2)))
    );
    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(995.0, 250.0), new_frame),
        Some(DockDropTarget::split(
            FrameId::from_raw(2),
            DockPlacement::Right,
            new_frame
        ))
    );
}

#[test]
fn dock_drop_target_resolution_rejects_invalid_geometry() {
    let new_frame = FrameId::from_raw(9);
    let invalid_layout = [FrameLayout {
        frame: FrameId::from_raw(1),
        rect: Rect::new(0.0, 0.0, f32::INFINITY, 100.0),
    }];

    assert_eq!(
        resolve_dock_drop_target(&invalid_layout, Point::new(1.0, 50.0), new_frame),
        None
    );
    assert_eq!(
        resolve_dock_drop_target(
            &[FrameLayout {
                frame: FrameId::from_raw(1),
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
            }],
            Point::new(f32::NAN, 50.0),
            new_frame
        ),
        None
    );

    let mixed_layout = [
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, f32::INFINITY, 100.0),
        },
        FrameLayout {
            frame: FrameId::from_raw(2),
            rect: Rect::new(100.0, 0.0, 100.0, 100.0),
        },
    ];

    let target = resolve_dock_drop_target(&mixed_layout, Point::new(198.0, 50.0), new_frame)
        .expect("target");
    match target {
        DockDropTarget::Split {
            frame,
            placement,
            new_frame: inserted_frame,
            ratio,
            min_first,
            min_second,
        } => {
            assert_eq!(frame, FrameId::from_raw(2));
            assert_eq!(placement, DockPlacement::Right);
            assert_eq!(inserted_frame, new_frame);
            assert!(ratio.is_finite());
            assert!(min_first.is_finite());
            assert!(min_second.is_finite());
        }
        DockDropTarget::Tab { .. } => panic!("expected split target"),
    }
}
