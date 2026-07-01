#[test]
fn drop_targets_distinguish_center_merge_from_edge_split() {
    let dock = nested_dock();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);

    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(650.0, 150.0), new_frame),
        Some(DockDropTarget::tab(FrameId::from_raw(2)))
    );
    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(998.0, 250.0), new_frame),
        Some(DockDropTarget::split(
            FrameId::from_raw(2),
            DockPlacement::Right,
            new_frame,
        ))
    );
    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(650.0, 498.0), new_frame),
        Some(DockDropTarget::split(
            FrameId::from_raw(3),
            DockPlacement::Bottom,
            new_frame,
        ))
    );
}

#[test]
fn frame_split_affordance_requests_resolve_edges_and_corners() {
    let mut dock = nested_dock();
    assert!(dock.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));
    let before = dock.snapshot();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);

    for (point, target_frame, placement) in [
        (
            Point::new(301.0, 150.0),
            FrameId::from_raw(2),
            DockPlacement::Left,
        ),
        (
            Point::new(998.0, 150.0),
            FrameId::from_raw(2),
            DockPlacement::Right,
        ),
        (
            Point::new(650.0, 2.0),
            FrameId::from_raw(2),
            DockPlacement::Top,
        ),
        (
            Point::new(650.0, 298.0),
            FrameId::from_raw(2),
            DockPlacement::Bottom,
        ),
        (
            Point::new(302.0, 2.0),
            FrameId::from_raw(2),
            DockPlacement::Left,
        ),
        (
            Point::new(998.0, 2.0),
            FrameId::from_raw(2),
            DockPlacement::Right,
        ),
    ] {
        assert_eq!(
            resolve_frame_split_affordance_request(
                &dock,
                &layout,
                FrameId::from_raw(2),
                point,
                new_frame,
            ),
            Some(FrameSplitAffordanceRequest {
                source_frame: FrameId::from_raw(2),
                target_frame,
                placement,
                active_panel: Some(PanelInstanceLocation {
                    panel_instance: PanelInstanceId::from_raw(3),
                    panel: PanelId::from_raw(3),
                    frame: FrameId::from_raw(2),
                }),
                new_frame,
            })
        );
    }

    assert_eq!(dock.snapshot(), before);
}

#[test]
fn frame_split_affordance_requests_reject_center_and_invalid_inputs() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);

    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &layout,
            FrameId::from_raw(2),
            Point::new(650.0, 150.0),
            new_frame,
        ),
        None
    );
    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &layout,
            FrameId::from_raw(2),
            Point::new(f32::NAN, 150.0),
            new_frame,
        ),
        None
    );
    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &[FrameLayout {
                frame: FrameId::from_raw(2),
                rect: Rect::new(300.0, 0.0, f32::INFINITY, 300.0),
            }],
            FrameId::from_raw(2),
            Point::new(301.0, 150.0),
            new_frame,
        ),
        None
    );
    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &layout,
            FrameId::from_raw(99),
            Point::new(301.0, 150.0),
            new_frame,
        ),
        None
    );

    assert_eq!(dock.snapshot(), before);
}

#[test]
fn frame_split_affordance_requests_keep_center_distinct_from_overlapping_edge() {
    let dock = Dock::new(DockNode::Frame(frame(1, vec![panel(1, "A")])));
    let before = dock.snapshot();
    let layout = [
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        },
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(40.0, 40.0, 100.0, 100.0),
        },
    ];

    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &layout,
            FrameId::from_raw(1),
            Point::new(50.0, 50.0),
            FrameId::from_raw(9),
        ),
        None
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn frame_split_affordance_request_allows_missing_active_panel_identity() {
    let dock = Dock::new(DockNode::Frame(Frame::new(FrameId::from_raw(1), vec![])));
    let before = dock.snapshot();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(
        resolve_frame_split_affordance_request(
            &dock,
            &layout,
            FrameId::from_raw(1),
            Point::new(1.0, 50.0),
            FrameId::from_raw(9),
        ),
        Some(FrameSplitAffordanceRequest {
            source_frame: FrameId::from_raw(1),
            target_frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            active_panel: None,
            new_frame: FrameId::from_raw(9),
        })
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn tab_merge_split_and_dismissible_policy_round_trip_through_snapshot() {
    let mut dock = nested_dock();
    dock.frame_mut(FrameId::from_raw(2))
        .expect("source frame")
        .set_panel_dismissible(PanelId::from_raw(3), false);
    let drag = dock
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(dock.drop_tab(
        drag,
        DockDropTarget::Split {
            frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(9),
            ratio: 0.4,
            min_first: 70.0,
            min_second: 90.0,
        },
    ));

    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(9)));
    let inserted = dock.frame(FrameId::from_raw(9)).expect("inserted frame");
    assert_eq!(
        inserted.active_panel().expect("active panel").id,
        PanelId::from_raw(3)
    );
    assert!(!inserted.panel_dismissible(PanelId::from_raw(3)));

    let restored = Dock::restore(dock.snapshot()).expect("restore");
    assert_eq!(restored.active_frame(), Some(FrameId::from_raw(9)));
    let restored_inserted = restored
        .frame(FrameId::from_raw(9))
        .expect("restored frame");
    assert_eq!(
        restored_inserted.active_panel().expect("active panel").id,
        PanelId::from_raw(3)
    );
    assert!(!restored_inserted.panel_dismissible(PanelId::from_raw(3)));
    let tabs = frame_tabs(restored_inserted);
    assert_eq!(tabs.len(), 1);
    assert!(!tabs[0].close_visible);
    assert!(tabs[0].draggable);
}

#[test]
fn invalid_tab_and_split_drops_leave_the_tree_unchanged() {
    let mut dock = nested_dock();
    let before = dock.snapshot();
    let drag = dock
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(!dock.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(99))));
    assert_eq!(dock.snapshot(), before);

    assert!(!dock.drop_tab(
        drag,
        DockDropTarget::split(
            FrameId::from_raw(99),
            DockPlacement::Left,
            FrameId::from_raw(9),
        )
    ));
    assert_eq!(dock.snapshot(), before);

    assert!(!dock.drop_tab(
        drag,
        DockDropTarget::Split {
            frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(2),
            ratio: 0.4,
            min_first: 0.0,
            min_second: 0.0,
        },
    ));
    assert_eq!(dock.snapshot(), before);

    assert!(!dock.drop_tab(
        drag,
        DockDropTarget::Split {
            frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(9),
            ratio: f32::NAN,
            min_first: 0.0,
            min_second: 0.0,
        },
    ));
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn active_frame_refreshes_deterministically_when_frames_move_or_close() {
    let mut dock = nested_dock();
    assert!(dock.set_active_frame(FrameId::from_raw(3)));

    assert!(dock.move_panel(
        FrameId::from_raw(3),
        FrameId::from_raw(1),
        PanelId::from_raw(4),
    ));

    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(1)));
    assert!(dock.frame(FrameId::from_raw(3)).is_none());
    assert_eq!(
        dock.frame(FrameId::from_raw(1))
            .expect("target")
            .active_panel()
            .expect("active")
            .id,
        PanelId::from_raw(4)
    );

    assert!(dock.set_active_frame(FrameId::from_raw(2)));
    assert!(dock.merge_frames(FrameId::from_raw(2), FrameId::from_raw(1)));
    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(1)));
    assert!(dock.frame(FrameId::from_raw(2)).is_none());
}

#[test]
fn snapshot_restore_rejects_invalid_identity_policy_and_split_data() {
    let duplicate_panels = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A"), panel(1, "Duplicate")],
            active: 0,
            dismissible_panels: vec![PanelId::from_raw(1)],
        },
    };
    assert_eq!(
        Dock::restore(duplicate_panels).expect_err("duplicate panels"),
        DockRestoreError::DuplicatePanelId
    );

    let unknown_policy_panel = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A")],
            active: 0,
            dismissible_panels: vec![PanelId::from_raw(2)],
        },
    };
    assert_eq!(
        Dock::restore(unknown_policy_panel).expect_err("unknown policy panel"),
        DockRestoreError::InvalidDismissiblePanel
    );

    let invalid_split = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 1.25,
            min_first: 0.0,
            min_second: 0.0,
            first: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1)],
            }),
            second: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(2),
                panels: vec![panel(2, "B")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(2)],
            }),
        },
    };
    assert_eq!(
        Dock::restore(invalid_split).expect_err("invalid split"),
        DockRestoreError::InvalidSplitRatio
    );
}

#[test]
fn panel_remains_passive_metadata_when_frame_and_dock_policy_changes() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let original_panel = dock
        .frame(FrameId::from_raw(2))
        .expect("frame")
        .panels
        .iter()
        .find(|panel| panel.id == PanelId::from_raw(3))
        .expect("panel")
        .clone();

    assert!(
        dock.frame_mut(FrameId::from_raw(2))
            .expect("frame")
            .set_panel_dismissible(original_panel.id, false)
    );
    let drag = dock
        .begin_tab_drag(FrameId::from_raw(2), original_panel.id)
        .expect("drag");
    let styled_splitters = solve_dock_splitters_with_style(
        &dock,
        bounds,
        DockChromeStyle::default().with_splitter_hit_thickness(20.0),
    );
    assert_eq!(styled_splitters.len(), 2);
    let target = resolve_dock_drop_target_with_policy(
        &solve_dock_layout(&dock, bounds),
        Point::new(150.0, 498.0),
        FrameId::from_raw(9),
        DockInteractionPolicy::default().with_drop_edge_fraction(0.4),
    )
    .expect("policy split target");
    match target {
        DockDropTarget::Split {
            frame, placement, ..
        } => {
            assert_eq!(frame, FrameId::from_raw(1));
            assert_eq!(placement, DockPlacement::Bottom);
        }
        DockDropTarget::Tab { .. } => panic!("expected split target"),
    }
    assert!(dock.drop_tab(drag, target));

    let moved_panel = dock
        .frame(FrameId::from_raw(9))
        .expect("inserted frame")
        .active_panel()
        .expect("active panel");
    assert_eq!(moved_panel, &original_panel);
    assert!(
        !dock
            .frame(FrameId::from_raw(9))
            .expect("inserted frame")
            .panel_dismissible(original_panel.id)
    );
}
