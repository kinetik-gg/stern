#[test]
fn dock_neighbors_resolve_left_right_up_down_in_nested_splits() {
    let dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    let neighbors = solve_dock_neighbors(&dock, bounds);

    assert_eq!(neighbors.len(), 3);
    assert_eq!(
        neighbors_for(&neighbors, 1),
        FrameNeighbors {
            frame: FrameId::from_raw(1),
            left: None,
            right: Some(FrameId::from_raw(2)),
            up: None,
            down: None,
        }
    );
    assert_eq!(
        neighbors_for(&neighbors, 2),
        FrameNeighbors {
            frame: FrameId::from_raw(2),
            left: Some(FrameId::from_raw(1)),
            right: None,
            up: None,
            down: Some(FrameId::from_raw(3)),
        }
    );
    assert_eq!(
        neighbors_for(&neighbors, 3),
        FrameNeighbors {
            frame: FrameId::from_raw(3),
            left: Some(FrameId::from_raw(1)),
            right: None,
            up: Some(FrameId::from_raw(2)),
            down: None,
        }
    );
}
#[test]
fn dock_neighbor_lookup_never_returns_self() {
    let layout = [
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        },
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(100.0, 0.0, 100.0, 100.0),
        },
    ];

    assert_eq!(
        frame_neighbor(&layout, FrameId::from_raw(1), DockNeighborDirection::Right,),
        None
    );
}

#[test]
fn dock_neighbor_t_junction_ties_use_lowest_frame_id() {
    let dock = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Left")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.5,
            min_first: 0.0,
            min_second: 0.0,
            first: Box::new(DockNode::Frame(frame(3, vec![panel(3, "Top Right")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "Bottom Right")]))),
        }),
    });

    assert_eq!(
        frame_neighbor(
            &solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0)),
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        Some(FrameId::from_raw(2))
    );
}

#[test]
fn dock_neighbor_prefers_nearer_split_column_over_far_full_height_frame() {
    let dock = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.25,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Left")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 0.0,
            min_second: 0.0,
            first: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.5,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockNode::Frame(frame(3, vec![panel(3, "Near Top")]))),
                second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "Near Bottom")]))),
            }),
            second: Box::new(DockNode::Frame(frame(4, vec![panel(4, "Far Full")]))),
        }),
    });

    assert_eq!(
        frame_neighbor(
            &solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0)),
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        Some(FrameId::from_raw(2))
    );
}

#[test]
fn repeated_layout_solves_produce_stable_dock_neighbors() {
    let dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    let first = solve_dock_neighbors(&dock, bounds);
    let second = solve_dock_neighbors(&dock, bounds);

    assert_eq!(first, second);
    assert_eq!(
        frame_neighbor(
            &solve_dock_layout(&dock, bounds),
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        Some(FrameId::from_raw(2))
    );
}

#[test]
fn invalid_and_empty_geometry_returns_no_dock_neighbors() {
    let dock = nested_dock();
    let invalid_neighbors =
        solve_dock_neighbors(&dock, Rect::new(f32::NAN, f32::INFINITY, -100.0, 0.0));

    assert_eq!(invalid_neighbors.len(), 3);
    assert!(
        invalid_neighbors
            .iter()
            .all(|neighbors| neighbors.left.is_none()
                && neighbors.right.is_none()
                && neighbors.up.is_none()
                && neighbors.down.is_none())
    );
    assert_eq!(
        invalid_neighbors,
        solve_dock_neighbors(&dock, Rect::new(f32::NAN, f32::INFINITY, -100.0, 0.0))
    );

    let invalid_layout = [
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, f32::INFINITY, 100.0),
        },
        FrameLayout {
            frame: FrameId::from_raw(2),
            rect: Rect::new(100.0, 0.0, 100.0, 100.0),
        },
    ];
    assert_eq!(
        frame_neighbor(
            &invalid_layout,
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        None
    );
    assert_eq!(
        frame_neighbor(&[], FrameId::from_raw(1), DockNeighborDirection::Right),
        None
    );
}

#[test]
fn dock_join_requests_resolve_left_right_up_down_neighbors() {
    let dock = nested_dock();
    let neighbors = solve_dock_neighbors(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));

    for (source, direction, target) in [
        (1, DockNeighborDirection::Right, 2),
        (2, DockNeighborDirection::Left, 1),
        (2, DockNeighborDirection::Down, 3),
        (3, DockNeighborDirection::Up, 2),
    ] {
        let request = resolve_dock_join_request(&neighbors, FrameId::from_raw(source), direction)
            .expect("join request");

        assert_eq!(request.source_frame(), FrameId::from_raw(source));
        assert_eq!(request.direction(), direction);
        assert_eq!(request.target_frame(), FrameId::from_raw(target));
    }
}

#[test]
fn dock_join_requests_reject_invalid_topology_without_mutation() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let before = dock.snapshot();
    let neighbors = solve_dock_neighbors(&dock, bounds);

    assert_eq!(
        resolve_dock_join_request(
            &neighbors,
            FrameId::from_raw(99),
            DockNeighborDirection::Right
        ),
        None
    );
    assert!(!dock.join_neighbor(bounds, FrameId::from_raw(99), DockNeighborDirection::Right));
    assert_eq!(dock.snapshot(), before);

    assert_eq!(
        resolve_dock_join_request(
            &neighbors,
            FrameId::from_raw(1),
            DockNeighborDirection::Left
        ),
        None
    );
    assert!(!dock.join_neighbor(bounds, FrameId::from_raw(1), DockNeighborDirection::Left));
    assert_eq!(dock.snapshot(), before);

    assert_eq!(
        resolve_dock_join_request(
            &neighbors,
            FrameId::from_raw(1),
            DockNeighborDirection::Down
        ),
        None
    );
    assert!(!dock.join_neighbor(bounds, FrameId::from_raw(1), DockNeighborDirection::Down));
    assert_eq!(dock.snapshot(), before);

    let self_join = [FrameNeighbors {
        frame: FrameId::from_raw(1),
        left: Some(FrameId::from_raw(1)),
        right: None,
        up: None,
        down: None,
    }];
    assert_eq!(
        resolve_dock_join_request(
            &self_join,
            FrameId::from_raw(1),
            DockNeighborDirection::Left
        ),
        None
    );
    assert_eq!(dock.snapshot(), before);

    let missing_target = [FrameNeighbors {
        frame: FrameId::from_raw(1),
        left: None,
        right: Some(FrameId::from_raw(99)),
        up: None,
        down: None,
    }];
    assert_eq!(
        resolve_dock_join_request(
            &missing_target,
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        None
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn dock_join_moves_source_tabs_into_neighbor_and_round_trips() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    assert!(dock.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));
    dock.frame_mut(FrameId::from_raw(2))
        .expect("source frame")
        .set_panel_dismissible(PanelId::from_raw(3), false);
    let neighbors = solve_dock_neighbors(&dock, bounds);
    let request = resolve_dock_join_request(
        &neighbors,
        FrameId::from_raw(2),
        DockNeighborDirection::Left,
    )
    .expect("join request");

    assert!(dock.apply_join_request(bounds, request));

    assert!(dock.frame(FrameId::from_raw(2)).is_none());
    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(1)));
    let target = dock.frame(FrameId::from_raw(1)).expect("target frame");
    assert_eq!(
        target
            .panels
            .iter()
            .map(|panel| panel.id)
            .collect::<Vec<_>>(),
        vec![
            PanelId::from_raw(1),
            PanelId::from_raw(2),
            PanelId::from_raw(3),
        ]
    );
    assert_eq!(
        target.active_panel().expect("active panel").id,
        PanelId::from_raw(3)
    );
    assert!(target.panel_dismissible(PanelId::from_raw(1)));
    assert!(target.panel_dismissible(PanelId::from_raw(2)));
    assert!(!target.panel_dismissible(PanelId::from_raw(3)));

    let snapshot = dock.snapshot();
    let restored = Dock::restore(snapshot.clone()).expect("restore");
    assert_eq!(restored.snapshot(), snapshot);
    let restored_target = restored
        .frame(FrameId::from_raw(1))
        .expect("restored target");
    assert_eq!(
        restored_target
            .active_panel()
            .expect("restored active panel")
            .id,
        PanelId::from_raw(3)
    );
    assert!(!restored_target.panel_dismissible(PanelId::from_raw(3)));
}

#[test]
fn dock_join_rejects_forged_non_adjacent_topology_without_mutation() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let before = dock.snapshot();
    let forged_neighbors = [
        FrameNeighbors {
            frame: FrameId::from_raw(1),
            left: None,
            right: None,
            up: None,
            down: Some(FrameId::from_raw(3)),
        },
        FrameNeighbors::empty(FrameId::from_raw(3)),
    ];
    let request = resolve_dock_join_request(
        &forged_neighbors,
        FrameId::from_raw(1),
        DockNeighborDirection::Down,
    )
    .expect("forged request still resolves as pure metadata");

    assert!(!dock.apply_join_request(bounds, request));
    assert!(!dock.join_neighbor(bounds, FrameId::from_raw(1), DockNeighborDirection::Down));
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn dock_join_rejects_stale_resolved_requests_without_mutation() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let original_neighbors = solve_dock_neighbors(&dock, bounds);
    let stale_request = resolve_dock_join_request(
        &original_neighbors,
        FrameId::from_raw(2),
        DockNeighborDirection::Left,
    )
    .expect("original join request");

    assert!(dock.split_panel(
        FrameId::from_raw(2),
        PanelId::from_raw(3),
        DockSplitInsertion::new(
            FrameId::from_raw(2),
            DockPlacement::Left,
            FrameId::from_raw(9),
        ),
    ));
    assert_eq!(
        frame_neighbor(
            &solve_dock_layout(&dock, bounds),
            FrameId::from_raw(2),
            DockNeighborDirection::Left,
        ),
        Some(FrameId::from_raw(9))
    );
    let before_stale_apply = dock.snapshot();

    assert!(!dock.apply_join_request(bounds, stale_request));
    assert_eq!(dock.snapshot(), before_stale_apply);
}

#[test]
fn dock_join_requests_follow_neighbor_t_junction_ties() {
    let dock = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Left")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.5,
            min_first: 0.0,
            min_second: 0.0,
            first: Box::new(DockNode::Frame(frame(3, vec![panel(3, "Top Right")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "Bottom Right")]))),
        }),
    });
    let neighbors = solve_dock_neighbors(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));

    let request = resolve_dock_join_request(
        &neighbors,
        FrameId::from_raw(1),
        DockNeighborDirection::Right,
    )
    .expect("join request");

    assert_eq!(request.target_frame(), FrameId::from_raw(2));
}

#[test]
fn dock_swap_requests_resolve_left_right_up_down_neighbors() {
    let dock = nested_dock();
    let neighbors = solve_dock_neighbors(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));

    for (source, direction, target) in [
        (1, DockNeighborDirection::Right, 2),
        (2, DockNeighborDirection::Left, 1),
        (2, DockNeighborDirection::Down, 3),
        (3, DockNeighborDirection::Up, 2),
    ] {
        let request = resolve_dock_swap_request(&neighbors, FrameId::from_raw(source), direction)
            .expect("swap request");

        assert_eq!(request.source_frame(), FrameId::from_raw(source));
        assert_eq!(request.direction(), direction);
        assert_eq!(request.target_frame(), FrameId::from_raw(target));
    }
}

#[test]
fn dock_swap_exchanges_frame_leaves_for_each_neighbor_direction() {
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    for (source, direction, target) in [
        (1, DockNeighborDirection::Right, 2),
        (2, DockNeighborDirection::Left, 1),
        (2, DockNeighborDirection::Down, 3),
        (3, DockNeighborDirection::Up, 2),
    ] {
        let mut dock = nested_dock();
        let source_id = FrameId::from_raw(source);
        let target_id = FrameId::from_raw(target);
        let source_rect = frame_rect(&dock, source, bounds);
        let target_rect = frame_rect(&dock, target, bounds);
        let source_panels = panel_ids(dock.frame(source_id).expect("source before"));
        let target_panels = panel_ids(dock.frame(target_id).expect("target before"));
        let neighbors = solve_dock_neighbors(&dock, bounds);
        let request =
            resolve_dock_swap_request(&neighbors, source_id, direction).expect("swap request");

        assert!(dock.apply_swap_request(bounds, request));

        assert_eq!(frame_rect(&dock, source, bounds), target_rect);
        assert_eq!(frame_rect(&dock, target, bounds), source_rect);
        assert_eq!(
            panel_ids(dock.frame(source_id).expect("source after")),
            source_panels
        );
        assert_eq!(
            panel_ids(dock.frame(target_id).expect("target after")),
            target_panels
        );
    }
}

#[test]
fn dock_swap_preserves_frame_state_and_round_trips() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    assert!(dock.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));
    dock.frame_mut(FrameId::from_raw(2))
        .expect("source frame")
        .set_panel_dismissible(PanelId::from_raw(3), false);
    assert!(dock.set_active_frame(FrameId::from_raw(2)));
    let prior = dock.snapshot();

    assert!(dock.swap_neighbor(bounds, FrameId::from_raw(2), DockNeighborDirection::Left));

    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(2)));
    assert_eq!(
        frame_rect(&dock, 2, bounds),
        frame_rect(&nested_dock(), 1, bounds)
    );
    assert_eq!(
        panel_ids(dock.frame(FrameId::from_raw(2)).expect("source after")),
        vec![PanelId::from_raw(2), PanelId::from_raw(3)]
    );
    let source = dock.frame(FrameId::from_raw(2)).expect("source after");
    assert_eq!(
        source.active_panel().expect("active panel").id,
        PanelId::from_raw(3)
    );
    assert!(source.panel_dismissible(PanelId::from_raw(2)));
    assert!(!source.panel_dismissible(PanelId::from_raw(3)));
    assert_eq!(
        panel_ids(dock.frame(FrameId::from_raw(1)).expect("target after")),
        vec![PanelId::from_raw(1)]
    );

    let snapshot = dock.snapshot();
    let restored = Dock::restore(snapshot.clone()).expect("restore");
    assert_eq!(restored.snapshot(), snapshot);
    assert_eq!(restored.active_frame(), Some(FrameId::from_raw(2)));
    let restored_source = restored
        .frame(FrameId::from_raw(2))
        .expect("restored source");
    assert_eq!(
        restored_source
            .active_panel()
            .expect("restored active panel")
            .id,
        PanelId::from_raw(3)
    );
    assert!(!restored_source.panel_dismissible(PanelId::from_raw(3)));

    assert!(dock.swap_neighbor(bounds, FrameId::from_raw(2), DockNeighborDirection::Right));
    assert_eq!(dock.snapshot(), prior);
}

#[test]
fn dock_swap_rejects_invalid_topology_without_mutation() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let before = dock.snapshot();
    let neighbors = solve_dock_neighbors(&dock, bounds);

    assert_eq!(
        resolve_dock_swap_request(
            &neighbors,
            FrameId::from_raw(99),
            DockNeighborDirection::Right
        ),
        None
    );
    assert!(!dock.swap_neighbor(bounds, FrameId::from_raw(99), DockNeighborDirection::Right));
    assert_eq!(dock.snapshot(), before);

    assert_eq!(
        resolve_dock_swap_request(
            &neighbors,
            FrameId::from_raw(1),
            DockNeighborDirection::Left
        ),
        None
    );
    assert!(!dock.swap_neighbor(bounds, FrameId::from_raw(1), DockNeighborDirection::Left));
    assert_eq!(dock.snapshot(), before);

    let self_swap = [FrameNeighbors {
        frame: FrameId::from_raw(1),
        left: Some(FrameId::from_raw(1)),
        right: None,
        up: None,
        down: None,
    }];
    assert_eq!(
        resolve_dock_swap_request(
            &self_swap,
            FrameId::from_raw(1),
            DockNeighborDirection::Left
        ),
        None
    );
    assert_eq!(dock.snapshot(), before);

    let missing_target = [FrameNeighbors {
        frame: FrameId::from_raw(1),
        left: None,
        right: Some(FrameId::from_raw(99)),
        up: None,
        down: None,
    }];
    assert_eq!(
        resolve_dock_swap_request(
            &missing_target,
            FrameId::from_raw(1),
            DockNeighborDirection::Right,
        ),
        None
    );
    assert_eq!(dock.snapshot(), before);

    let forged_neighbors = [
        FrameNeighbors {
            frame: FrameId::from_raw(1),
            left: None,
            right: None,
            up: None,
            down: Some(FrameId::from_raw(3)),
        },
        FrameNeighbors::empty(FrameId::from_raw(3)),
    ];
    let request = resolve_dock_swap_request(
        &forged_neighbors,
        FrameId::from_raw(1),
        DockNeighborDirection::Down,
    )
    .expect("forged request still resolves as pure metadata");

    assert!(!dock.apply_swap_request(bounds, request));
    assert!(!dock.swap_neighbor(bounds, FrameId::from_raw(1), DockNeighborDirection::Down));
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn dock_swap_rejects_stale_resolved_requests_without_mutation() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let original_neighbors = solve_dock_neighbors(&dock, bounds);
    let stale_request = resolve_dock_swap_request(
        &original_neighbors,
        FrameId::from_raw(2),
        DockNeighborDirection::Left,
    )
    .expect("original swap request");

    assert!(dock.split_panel(
        FrameId::from_raw(2),
        PanelId::from_raw(3),
        DockSplitInsertion::new(
            FrameId::from_raw(2),
            DockPlacement::Left,
            FrameId::from_raw(9),
        ),
    ));
    assert_eq!(
        frame_neighbor(
            &solve_dock_layout(&dock, bounds),
            FrameId::from_raw(2),
            DockNeighborDirection::Left,
        ),
        Some(FrameId::from_raw(9))
    );
    let before_stale_apply = dock.snapshot();

    assert!(!dock.apply_swap_request(bounds, stale_request));
    assert_eq!(dock.snapshot(), before_stale_apply);
}
