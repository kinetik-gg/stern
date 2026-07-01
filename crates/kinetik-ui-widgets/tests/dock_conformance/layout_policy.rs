#[test]
fn nested_splits_layout_resize_and_snapshot_cycles_are_deterministic() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    assert_close(frame_rect(&dock, 1, bounds).width, 300.0);
    assert_close(frame_rect(&dock, 2, bounds).height, 300.0);
    assert_close(frame_rect(&dock, 3, bounds).height, 200.0);

    let splitters = solve_dock_splitters(&dock, bounds, 8.0);
    assert_eq!(splitters.len(), 2);
    assert_eq!(splitters[0].path, DockSplitPath::root());
    assert_eq!(
        splitters[1].path,
        DockSplitPath::root().child(DockPathElement::Second)
    );

    assert!(dock.resize_split(
        &DockSplitPath::root().child(DockPathElement::Second),
        bounds,
        Vec2::new(0.0, 50.0),
    ));
    assert_close(frame_rect(&dock, 2, bounds).height, 350.0);
    assert_close(frame_rect(&dock, 3, bounds).height, 150.0);

    let first_snapshot = dock.snapshot();
    let restored = Dock::restore(first_snapshot.clone()).expect("restore");
    assert_eq!(restored.snapshot(), first_snapshot);
    let restored_again = Dock::restore(restored.snapshot()).expect("restore again");
    assert_eq!(restored_again.snapshot(), first_snapshot);
}
#[test]
fn invalid_geometry_is_sanitized_for_layout_splitters_and_drag_ratios() {
    let mut dock = Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: f32::NAN,
        min_first: f32::INFINITY,
        min_second: -5.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
    });
    let invalid_bounds = Rect::new(f32::NAN, f32::INFINITY, -100.0, 300.0);

    for layout in solve_dock_layout(&dock, invalid_bounds) {
        assert!(layout.rect.x.is_finite());
        assert!(layout.rect.y.is_finite());
        assert!(layout.rect.width.is_finite());
        assert!(layout.rect.height.is_finite());
        assert!(layout.rect.width >= 0.0);
        assert!(layout.rect.height >= 0.0);
    }

    let splitters = solve_dock_splitters(&dock, invalid_bounds, f32::NAN);
    assert_eq!(splitters.len(), 1);
    assert_close(splitters[0].ratio, 0.5);
    assert!(splitters[0].min_first.is_finite());
    assert!(splitters[0].min_second.is_finite());

    let ratio = split_ratio_from_drag(
        Axis::Horizontal,
        invalid_bounds,
        f32::NAN,
        f32::INFINITY,
        -1.0,
        Vec2::new(f32::INFINITY, 0.0),
    );
    assert_close(ratio, 0.5);

    assert!(dock.resize_split(
        &DockSplitPath::root(),
        invalid_bounds,
        Vec2::new(f32::INFINITY, 0.0)
    ));
    match dock.root {
        DockNode::Split { ratio, .. } => assert_close(ratio, 0.5),
        DockNode::Frame(_) => panic!("root split should remain intact"),
    }
}

#[test]
fn splitter_context_actions_identify_adjacent_frames_and_requests() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let layout = solve_dock_layout(&dock, bounds);
    let splitters = solve_dock_splitters(&dock, bounds, 8.0);

    let actions = resolve_dock_splitter_context_actions(&dock, &layout, &splitters[0]);

    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0].context.path, DockSplitPath::root());
    assert_eq!(actions[0].context.axis, Axis::Horizontal);
    assert_eq!(actions[0].context.first_frame, Some(FrameId::from_raw(1)));
    assert_eq!(actions[0].context.second_frame, Some(FrameId::from_raw(2)));

    let join_right = splitter_context_action(
        &actions,
        DockSplitterContextActionKind::Join,
        DockSplitterSide::First,
    );
    assert!(join_right.enabled);
    assert_eq!(join_right.target_side, DockSplitterSide::Second);
    assert_eq!(join_right.source_frame, Some(FrameId::from_raw(1)));
    assert_eq!(join_right.target_frame, Some(FrameId::from_raw(2)));
    assert_eq!(join_right.direction, DockNeighborDirection::Right);
    let join_request = join_right.join_request().expect("join request");
    assert_eq!(join_request.source_frame(), FrameId::from_raw(1));
    assert_eq!(join_request.target_frame(), FrameId::from_raw(2));
    assert_eq!(join_request.direction(), DockNeighborDirection::Right);
    assert_eq!(join_right.swap_request(), None);

    let swap_left = splitter_context_action(
        &actions,
        DockSplitterContextActionKind::Swap,
        DockSplitterSide::Second,
    );
    assert!(swap_left.enabled);
    assert_eq!(swap_left.target_side, DockSplitterSide::First);
    assert_eq!(swap_left.source_frame, Some(FrameId::from_raw(2)));
    assert_eq!(swap_left.target_frame, Some(FrameId::from_raw(1)));
    assert_eq!(swap_left.direction, DockNeighborDirection::Left);
    let swap_request = swap_left.swap_request().expect("swap request");
    assert_eq!(swap_request.source_frame(), FrameId::from_raw(2));
    assert_eq!(swap_request.target_frame(), FrameId::from_raw(1));
    assert_eq!(swap_request.direction(), DockNeighborDirection::Left);
    assert_eq!(swap_left.join_request(), None);

    assert_eq!(dock.snapshot(), before);
}

#[test]
fn splitter_context_actions_have_stable_operation_kinds_and_directions() {
    let dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let layout = solve_dock_layout(&dock, bounds);
    let splitters = solve_dock_splitters(&dock, bounds, 8.0);

    let actions = resolve_dock_splitter_context_actions(&dock, &layout, &splitters[1]);
    let summary: Vec<_> = actions
        .iter()
        .map(|action| {
            (
                action.kind,
                action.source_side,
                action.direction,
                action.enabled,
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                DockSplitterContextActionKind::Join,
                DockSplitterSide::First,
                DockNeighborDirection::Down,
                true,
            ),
            (
                DockSplitterContextActionKind::Join,
                DockSplitterSide::Second,
                DockNeighborDirection::Up,
                true,
            ),
            (
                DockSplitterContextActionKind::Swap,
                DockSplitterSide::First,
                DockNeighborDirection::Down,
                true,
            ),
            (
                DockSplitterContextActionKind::Swap,
                DockSplitterSide::Second,
                DockNeighborDirection::Up,
                true,
            ),
        ]
    );
    assert!(actions.iter().all(|action| {
        action.context.path == DockSplitPath::new([DockPathElement::Second])
            && action.context.axis == Axis::Vertical
            && action.context.first_frame == Some(FrameId::from_raw(2))
            && action.context.second_frame == Some(FrameId::from_raw(3))
    }));
}

#[test]
fn splitter_context_actions_disable_invalid_or_missing_adjacent_frames() {
    let dock = Dock::new(DockNode::Frame(frame(1, vec![panel(1, "A")])));
    let before = dock.snapshot();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 100.0, 100.0));
    let stale_splitter = kinetik_ui_widgets::DockSplitter {
        path: DockSplitPath::root(),
        axis: Axis::Horizontal,
        rect: Rect::new(48.0, 0.0, 4.0, 100.0),
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
    };

    let actions = resolve_dock_splitter_context_actions(&dock, &layout, &stale_splitter);

    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| {
        !action.enabled
            && action.source_frame.is_none()
            && action.target_frame.is_none()
            && action.join_request().is_none()
            && action.swap_request().is_none()
    }));
    assert_eq!(dock.snapshot(), before);

    let split_dock = nested_dock();
    let splitters = solve_dock_splitters(&split_dock, Rect::new(0.0, 0.0, 1000.0, 500.0), 8.0);
    let invalid_layout = [
        FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, f32::NAN, 500.0),
        },
        FrameLayout {
            frame: FrameId::from_raw(2),
            rect: Rect::new(300.0, 0.0, 700.0, f32::INFINITY),
        },
    ];

    let invalid_actions =
        resolve_dock_splitter_context_actions(&split_dock, &invalid_layout, &splitters[0]);

    assert!(invalid_actions.iter().all(|action| !action.enabled));
    assert!(
        invalid_actions
            .iter()
            .all(|action| action.source_frame.is_none() || action.target_frame.is_none())
    );
}

#[test]
fn splitter_context_actions_are_pure_and_stable_across_solves() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    let first_layout = solve_dock_layout(&dock, bounds);
    let first_splitters = solve_dock_splitters(&dock, bounds, 8.0);
    let first_actions =
        resolve_dock_splitter_context_actions(&dock, &first_layout, &first_splitters[0]);

    let second_layout = solve_dock_layout(&dock, bounds);
    let second_splitters = solve_dock_splitters(&dock, bounds, 8.0);
    let second_actions =
        resolve_dock_splitter_context_actions(&dock, &second_layout, &second_splitters[0]);

    assert_eq!(first_actions, second_actions);
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn default_dock_policy_and_style_match_existing_splitter_behavior() {
    let dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let layout = solve_dock_layout(&dock, bounds);
    let new_frame = FrameId::from_raw(9);

    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(650.0, 150.0), new_frame),
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(650.0, 150.0),
            new_frame,
            DockInteractionPolicy::default(),
        )
    );
    assert_eq!(
        resolve_dock_drop_target(&layout, Point::new(998.0, 250.0), new_frame),
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(998.0, 250.0),
            new_frame,
            DockInteractionPolicy::default(),
        )
    );
    assert_eq!(
        solve_dock_splitters(&dock, bounds, 8.0),
        solve_dock_splitters_with_style(
            &dock,
            bounds,
            DockChromeStyle::default().with_splitter_hit_thickness(8.0),
        )
    );

    let splitters = solve_dock_splitters(&dock, bounds, 8.0);
    assert_eq!(
        resolve_dock_splitter_context_actions(&dock, &layout, &splitters[0]),
        resolve_dock_splitter_context_actions_with_policy(
            &dock,
            &layout,
            &splitters[0],
            DockInteractionPolicy::default(),
        )
    );
}

#[test]
fn custom_dock_policy_changes_drop_edge_resolution() {
    let dock = nested_dock();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);
    let narrow_edge = DockInteractionPolicy::default().with_drop_edge_fraction(0.10);
    let wide_edge = DockInteractionPolicy::default().with_drop_edge_fraction(0.40);

    assert_eq!(
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(390.0, 150.0),
            new_frame,
            narrow_edge,
        ),
        Some(DockDropTarget::tab(FrameId::from_raw(2)))
    );
    assert_eq!(
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(390.0, 150.0),
            new_frame,
            wide_edge
        ),
        Some(DockDropTarget::split(
            FrameId::from_raw(2),
            DockPlacement::Left,
            new_frame,
        ))
    );
}

#[test]
fn dock_policy_can_disable_split_or_tab_drop_targets() {
    let dock = nested_dock();
    let layout = solve_dock_layout(&dock, Rect::new(0.0, 0.0, 1000.0, 500.0));
    let new_frame = FrameId::from_raw(9);
    let no_splits = DockInteractionPolicy::default().with_split_insertion(false);
    let no_tabs = DockInteractionPolicy::default().with_tab_merge(false);

    assert_eq!(
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(998.0, 250.0),
            new_frame,
            no_splits,
        ),
        None
    );
    assert_eq!(
        resolve_frame_split_affordance_request_with_policy(
            &dock,
            &layout,
            FrameId::from_raw(2),
            Point::new(998.0, 250.0),
            new_frame,
            no_splits,
        ),
        None
    );
    assert_eq!(
        resolve_dock_drop_target_with_policy(&layout, Point::new(650.0, 150.0), new_frame, no_tabs,),
        None
    );
    assert_eq!(
        resolve_dock_drop_target_with_policy(&layout, Point::new(998.0, 250.0), new_frame, no_tabs,),
        Some(DockDropTarget::split(
            FrameId::from_raw(2),
            DockPlacement::Right,
            new_frame,
        ))
    );
}

#[test]
fn dock_policy_disables_splitter_resize_join_and_swap_without_mutating_topology() {
    let mut dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let before = dock.snapshot();
    let no_resize = DockInteractionPolicy::default().with_splitter_resize(false);

    assert!(!dock.resize_split_with_policy(
        &DockSplitPath::root().child(DockPathElement::Second),
        bounds,
        Vec2::new(0.0, 50.0),
        no_resize,
    ));
    assert_eq!(dock.snapshot(), before);

    let layout = solve_dock_layout(&dock, bounds);
    let splitters = solve_dock_splitters(&dock, bounds, 8.0);
    let no_join = DockInteractionPolicy::default().with_splitter_join(false);
    let join_disabled =
        resolve_dock_splitter_context_actions_with_policy(&dock, &layout, &splitters[0], no_join);
    assert!(
        join_disabled
            .iter()
            .filter(|action| action.kind == DockSplitterContextActionKind::Join)
            .all(|action| !action.enabled
                && action.join_request().is_none()
                && action.source_frame.is_some()
                && action.target_frame.is_some())
    );
    assert!(
        join_disabled
            .iter()
            .filter(|action| action.kind == DockSplitterContextActionKind::Swap)
            .all(|action| action.enabled && action.swap_request().is_some())
    );

    let no_swap = DockInteractionPolicy::default().with_splitter_swap(false);
    let swap_disabled =
        resolve_dock_splitter_context_actions_with_policy(&dock, &layout, &splitters[0], no_swap);
    assert!(
        swap_disabled
            .iter()
            .filter(|action| action.kind == DockSplitterContextActionKind::Swap)
            .all(|action| !action.enabled
                && action.swap_request().is_none()
                && action.source_frame.is_some()
                && action.target_frame.is_some())
    );
    assert!(
        swap_disabled
            .iter()
            .filter(|action| action.kind == DockSplitterContextActionKind::Join)
            .all(|action| action.enabled && action.join_request().is_some())
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn invalid_dock_policy_and_style_values_sanitize_deterministically() {
    let dock = nested_dock();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);
    let layout = solve_dock_layout(&dock, bounds);
    let new_frame = FrameId::from_raw(9);

    let invalid_policy = DockInteractionPolicy::default().with_drop_edge_fraction(f32::NAN);
    assert_close(
        invalid_policy.sanitized().drop_targets.edge_fraction,
        DockInteractionPolicy::default().drop_targets.edge_fraction,
    );
    assert_eq!(
        resolve_dock_drop_target_with_policy(
            &layout,
            Point::new(998.0, 250.0),
            new_frame,
            invalid_policy,
        ),
        resolve_dock_drop_target(&layout, Point::new(998.0, 250.0), new_frame)
    );

    let clamped_policy = DockInteractionPolicy::default().with_drop_edge_fraction(2.0);
    assert_close(clamped_policy.sanitized().drop_targets.edge_fraction, 0.5);
    assert_eq!(
        resolve_frame_drop_zone_with_policy(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Point::new(49.0, 50.0),
            clamped_policy,
        ),
        Some(kinetik_ui_widgets::DockDropZone::Left)
    );

    let invalid_style = DockChromeStyle::default().with_splitter_hit_thickness(f32::NEG_INFINITY);
    assert_eq!(invalid_style.sanitized(), DockChromeStyle::default());
    assert_eq!(
        solve_dock_splitters_with_style(&dock, bounds, invalid_style),
        solve_dock_splitters_with_style(&dock, bounds, DockChromeStyle::default())
    );
}

#[test]
fn dock_chrome_style_changes_splitter_hit_metadata_without_changing_topology() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let bounds = Rect::new(0.0, 0.0, 1000.0, 500.0);

    let thin = solve_dock_splitters_with_style(
        &dock,
        bounds,
        DockChromeStyle::default().with_splitter_hit_thickness(4.0),
    );
    let thick = solve_dock_splitters_with_style(
        &dock,
        bounds,
        DockChromeStyle::default().with_splitter_hit_thickness(20.0),
    );

    assert_eq!(thin.len(), thick.len());
    assert_eq!(thin[0].path, thick[0].path);
    assert_eq!(thin[0].axis, thick[0].axis);
    assert_close(thin[0].rect.width, 4.0);
    assert_close(thick[0].rect.width, 20.0);
    assert_close(thin[0].ratio, thick[0].ratio);
    assert_close(thin[0].min_first, thick[0].min_first);
    assert_close(thin[0].min_second, thick[0].min_second);
    assert_eq!(dock.snapshot(), before);
}
