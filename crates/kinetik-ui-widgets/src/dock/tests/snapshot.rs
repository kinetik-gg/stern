#[test]
fn dropping_tab_on_same_frame_selects_without_reordering() {
    let mut area = dock();
    assert!(area.set_active_frame(FrameId::from_raw(1)));
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(2))));

    let frame = area.frame(FrameId::from_raw(2)).expect("frame");
    assert_eq!(frame.panels.len(), 2);
    assert_eq!(
        frame
            .panels
            .iter()
            .map(|panel| panel.id)
            .collect::<Vec<_>>(),
        vec![PanelId::from_raw(2), PanelId::from_raw(3)]
    );
    assert_eq!(
        frame.active_panel().expect("active").id,
        PanelId::from_raw(3)
    );
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
}

#[test]
fn dropping_tab_on_frame_merges_and_selects_panel() {
    let mut area = dock();
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(1))));

    let target = area.frame(FrameId::from_raw(1)).expect("target");
    assert_eq!(target.panels.len(), 2);
    assert_eq!(
        target.active_panel().expect("active").id,
        PanelId::from_raw(3)
    );
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
    assert_eq!(
        area.frame(FrameId::from_raw(2))
            .expect("source")
            .panels
            .len(),
        1
    );
}

#[test]
fn dropping_tab_preserves_dismissible_policy_through_snapshot() {
    let mut area = dock();
    area.frame_mut(FrameId::from_raw(2))
        .expect("source")
        .set_panel_dismissible(PanelId::from_raw(3), false);
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(1))));

    let target = area.frame(FrameId::from_raw(1)).expect("target");
    assert!(!target.panel_dismissible(PanelId::from_raw(3)));

    let restored = Dock::restore(area.snapshot()).expect("restore");
    assert!(
        !restored
            .frame(FrameId::from_raw(1))
            .expect("restored target")
            .panel_dismissible(PanelId::from_raw(3))
    );
}

#[test]
fn dropping_tab_on_split_edge_inserts_new_frame_and_round_trips() {
    let mut area = dock();
    area.frame_mut(FrameId::from_raw(2))
        .expect("source")
        .set_panel_dismissible(PanelId::from_raw(3), false);
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");
    let insertion = DockSplitInsertion {
        target_frame: FrameId::from_raw(1),
        placement: DockPlacement::Left,
        new_frame: FrameId::from_raw(9),
        ratio: 0.3,
        min_first: 80.0,
        min_second: 120.0,
    };

    assert!(area.drop_tab(
        drag,
        DockDropTarget::Split {
            frame: insertion.target_frame,
            placement: insertion.placement,
            new_frame: insertion.new_frame,
            ratio: insertion.ratio,
            min_first: insertion.min_first,
            min_second: insertion.min_second,
        }
    ));

    let frames = area.frames();
    assert_eq!(frames.len(), 3);
    assert_eq!(area.active_frame(), Some(FrameId::from_raw(9)));
    assert_eq!(
        area.frame(FrameId::from_raw(9))
            .expect("inserted")
            .active_panel()
            .expect("panel")
            .id,
        PanelId::from_raw(3)
    );
    assert!(
        !area
            .frame(FrameId::from_raw(9))
            .expect("inserted")
            .panel_dismissible(PanelId::from_raw(3))
    );
    let restored = Dock::restore(area.snapshot()).expect("restore");
    assert_eq!(restored.frames().len(), 3);
    assert_eq!(restored.active_frame(), Some(FrameId::from_raw(9)));
    assert_eq!(
        restored
            .frame(FrameId::from_raw(9))
            .expect("inserted")
            .active_panel()
            .expect("panel")
            .id,
        PanelId::from_raw(3)
    );
    assert!(
        !restored
            .frame(FrameId::from_raw(9))
            .expect("restored inserted")
            .panel_dismissible(PanelId::from_raw(3))
    );
}

#[test]
fn invalid_tab_drop_does_not_remove_panel() {
    let mut area = dock();
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(!area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(99))));

    assert!(
        area.frame(FrameId::from_raw(2))
            .expect("source")
            .panels
            .iter()
            .any(|panel| panel.id == PanelId::from_raw(3))
    );
}

#[test]
fn invalid_split_drop_does_not_remove_panel() {
    let mut area = dock();
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(!area.drop_tab(
        drag,
        DockDropTarget::split(
            FrameId::from_raw(99),
            DockPlacement::Left,
            FrameId::from_raw(9)
        )
    ));

    assert!(
        area.frame(FrameId::from_raw(2))
            .expect("source")
            .panels
            .iter()
            .any(|panel| panel.id == PanelId::from_raw(3))
    );
}

#[test]
fn invalid_split_numbers_do_not_remove_panel() {
    let mut area = dock();
    let drag = area
        .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
        .expect("drag");

    assert!(!area.drop_tab(
        drag,
        DockDropTarget::Split {
            frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(9),
            ratio: f32::NAN,
            min_first: 0.0,
            min_second: 0.0,
        }
    ));

    assert!(
        area.frame(FrameId::from_raw(2))
            .expect("source")
            .panels
            .iter()
            .any(|panel| panel.id == PanelId::from_raw(3))
    );
}

#[test]
fn dock_semantic_role_uses_current_name() {
    assert_eq!(format!("{:?}", SemanticRole::Dock), "Dock");
}

#[test]
fn snapshots_round_trip() {
    let mut area = dock();
    assert!(area.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));
    assert!(area.set_active_frame(FrameId::from_raw(1)));
    let snapshot = area.snapshot();
    let restored = Dock::restore(snapshot).expect("restore");

    assert_eq!(restored.frames().len(), 2);
    assert_eq!(restored.active_frame(), Some(FrameId::from_raw(1)));
    assert_eq!(
        restored
            .frame(FrameId::from_raw(2))
            .expect("frame")
            .active_panel()
            .expect("active")
            .id,
        PanelId::from_raw(3)
    );
}

#[test]
fn restore_defaults_missing_active_frame_to_first_valid_frame() {
    let mut snapshot = dock().snapshot();
    snapshot.active_frame = None;

    let restored = Dock::restore(snapshot).expect("restore");

    assert_eq!(restored.active_frame(), Some(FrameId::from_raw(1)));
}

#[test]
fn invalid_snapshots_are_rejected() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![],
            active: 0,
            dismissible_panels: vec![],
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::EmptyFrame
    );
}

#[test]
fn invalid_snapshot_rejects_invalid_active_panel() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A")],
            active: 1,
            dismissible_panels: vec![PanelId::from_raw(1)],
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::InvalidActiveIndex
    );
}

#[test]
fn invalid_snapshot_rejects_unknown_dismissible_panel() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A")],
            active: 0,
            dismissible_panels: vec![PanelId::from_raw(2)],
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::InvalidDismissiblePanel
    );
}

#[test]
fn invalid_snapshot_rejects_unknown_active_frame() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(99)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A")],
            active: 0,
            dismissible_panels: vec![PanelId::from_raw(1)],
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::InvalidActiveFrame
    );
}

#[test]
fn invalid_snapshot_rejects_duplicate_frame_ids() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 0.0,
            min_second: 0.0,
            first: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1)],
            }),
            second: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(2, "B")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(2)],
            }),
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::DuplicateFrameId
    );
}

#[test]
fn invalid_snapshot_rejects_duplicate_panel_ids() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
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
                panels: vec![panel(1, "B")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1)],
            }),
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::DuplicatePanelId
    );
}

#[test]
fn invalid_snapshot_rejects_duplicate_dismissible_policy_entries() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![panel(1, "A")],
            active: 0,
            dismissible_panels: vec![PanelId::from_raw(1), PanelId::from_raw(1)],
        },
    };

    assert_eq!(
        Dock::restore(snapshot).expect_err("error"),
        DockRestoreError::DuplicateDismissiblePanel
    );
}

#[test]
fn invalid_snapshot_rejects_invalid_split_numbers() {
    let invalid_ratio = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: f32::NAN,
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
        Dock::restore(invalid_ratio).expect_err("error"),
        DockRestoreError::InvalidSplitRatio
    );

    let invalid_minimum = DockSnapshot {
        active_frame: Some(FrameId::from_raw(1)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: -1.0,
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
        Dock::restore(invalid_minimum).expect_err("error"),
        DockRestoreError::InvalidSplitMinimum
    );
}
