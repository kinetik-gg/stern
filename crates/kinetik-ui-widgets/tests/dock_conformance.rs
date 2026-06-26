//! Windowless Dock/Frame/Panel model conformance tests.

use kinetik_ui_core::{ActionId, Axis, IconId, Point, Rect, Size, Vec2};
use kinetik_ui_widgets::{
    Dock, DockDropTarget, DockNode, DockPathElement, DockPlacement, DockRestoreError, DockSnapshot,
    DockSnapshotDiagnosticCode, DockSnapshotNode, DockSnapshotSplitValue, DockSplitPath, Frame,
    FrameId, Panel, PanelClosePolicy, PanelDockHint, PanelDuplicatePolicy, PanelFloatPolicy,
    PanelId, PanelInstanceId, PanelInstancePolicy, PanelInstanceSnapshot, PanelTypeCategory,
    PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext, SnapshotDiagnosticSeverity,
    WorkspaceRestoreError, WorkspaceSnapshotDiagnosticCode, frame_tabs, resolve_dock_drop_target,
    solve_dock_layout, solve_dock_splitters, split_ratio_from_drag,
};

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}

fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

fn nested_dock() -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.3,
        min_first: 80.0,
        min_second: 120.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Media")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.6,
            min_first: 90.0,
            min_second: 110.0,
            first: Box::new(DockNode::Frame(frame(
                2,
                vec![panel(2, "Viewport"), panel(3, "Inspector")],
            ))),
            second: Box::new(DockNode::Frame(frame(3, vec![panel(4, "Timeline")]))),
        }),
    })
}

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 0.001,
        "expected {left} to be close to {right}"
    );
}

fn frame_rect(dock: &Dock, frame: u64, bounds: Rect) -> Rect {
    solve_dock_layout(dock, bounds)
        .into_iter()
        .find(|layout| layout.frame == FrameId::from_raw(frame))
        .expect("frame layout")
        .rect
}

fn workspace_panel_descriptors() -> Vec<PanelTypeDescriptor> {
    vec![
        PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Media"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Viewport"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Inspector"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(40), "Timeline"),
    ]
}

fn workspace_panel_instances() -> Vec<PanelInstanceSnapshot> {
    vec![
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(1),
            PanelTypeId::from_raw(10),
            "Media",
        )
        .with_state_key("media-state"),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(2),
            PanelTypeId::from_raw(20),
            "Viewport",
        ),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(3),
            PanelTypeId::from_raw(30),
            "Inspector",
        ),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(4),
            PanelTypeId::from_raw(40),
            "Timeline",
        ),
    ]
}

#[test]
fn panel_type_id_raw_bits_are_stable() {
    let id = PanelTypeId::from_raw(42);

    assert_eq!(id.raw(), 42);
    assert_eq!(PanelTypeId::from_raw(id.raw()), id);
}

#[test]
fn panel_type_descriptor_defaults_are_deterministic_and_editor_appropriate() {
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(7), "Inspector");

    assert_eq!(descriptor.id, PanelTypeId::from_raw(7));
    assert_eq!(descriptor.title, "Inspector");
    assert_eq!(descriptor.icon, None);
    assert_eq!(descriptor.category, PanelTypeCategory::General);
    assert_eq!(
        descriptor.instance_policy,
        PanelInstancePolicy::MultiInstance
    );
    assert_eq!(descriptor.default_size, Size::new(320.0, 240.0));
    assert_eq!(
        descriptor.allowed_contexts,
        vec![PanelWorkspaceContext::Docked]
    );
    assert_eq!(descriptor.dock_hints, vec![PanelDockHint::Tab]);
    assert_eq!(descriptor.close_policy, PanelClosePolicy::Closable);
    assert_eq!(descriptor.duplicate_policy, PanelDuplicatePolicy::Allowed);
    assert_eq!(descriptor.float_policy, PanelFloatPolicy::Unavailable);
    assert_eq!(descriptor.default_open_action, None);
}

#[test]
fn panel_type_descriptor_represents_workspace_metadata() {
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(8), "Timeline")
        .with_icon(IconId::from_raw(99))
        .with_category(PanelTypeCategory::Timeline)
        .with_default_size(Size::new(640.0, 180.0))
        .with_allowed_contexts([
            PanelWorkspaceContext::Docked,
            PanelWorkspaceContext::Floating,
        ])
        .with_dock_hints([
            PanelDockHint::Split(DockPlacement::Bottom),
            PanelDockHint::Tab,
        ])
        .with_close_policy(PanelClosePolicy::Required)
        .with_float_policy(PanelFloatPolicy::Allowed)
        .with_default_open_action(ActionId::new("workspace.open.timeline"));

    assert_eq!(descriptor.icon, Some(IconId::from_raw(99)));
    assert_eq!(descriptor.category, PanelTypeCategory::Timeline);
    assert_eq!(descriptor.default_size, Size::new(640.0, 180.0));
    assert_eq!(
        descriptor.allowed_contexts,
        vec![
            PanelWorkspaceContext::Docked,
            PanelWorkspaceContext::Floating
        ]
    );
    assert_eq!(
        descriptor.dock_hints,
        vec![
            PanelDockHint::Split(DockPlacement::Bottom),
            PanelDockHint::Tab,
        ]
    );
    assert_eq!(descriptor.close_policy, PanelClosePolicy::Required);
    assert_eq!(descriptor.float_policy, PanelFloatPolicy::Allowed);
    assert_eq!(
        descriptor
            .default_open_action
            .as_ref()
            .map(ActionId::as_str),
        Some("workspace.open.timeline")
    );
}

#[test]
fn panel_type_descriptor_represents_singleton_and_multi_instance_policy() {
    let singleton = PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Scene")
        .with_instance_policy(PanelInstancePolicy::Singleton)
        .with_duplicate_policy(PanelDuplicatePolicy::Denied);
    let multi = PanelTypeDescriptor::new(PanelTypeId::from_raw(11), "Viewport")
        .with_instance_policy(PanelInstancePolicy::MultiInstance)
        .with_duplicate_policy(PanelDuplicatePolicy::Allowed);

    assert_eq!(singleton.instance_policy, PanelInstancePolicy::Singleton);
    assert_eq!(singleton.duplicate_policy, PanelDuplicatePolicy::Denied);
    assert_eq!(multi.instance_policy, PanelInstancePolicy::MultiInstance);
    assert_eq!(multi.duplicate_policy, PanelDuplicatePolicy::Allowed);
}

#[test]
fn panel_id_and_panel_instance_id_convert_without_changing_existing_panel_usage() {
    let legacy = PanelId::from_raw(123);
    let instance = PanelInstanceId::from_raw(123);

    assert_eq!(legacy.instance_id(), instance);
    assert_eq!(PanelInstanceId::from(legacy), instance);
    assert_eq!(PanelId::from(instance), legacy);
    assert_eq!(PanelId::from_instance_id(instance), legacy);

    let panel = Panel::from_instance_id(instance, "Graph");
    assert_eq!(panel.id, legacy);
    assert_eq!(panel.instance_id(), instance);
    assert_eq!(Panel::new(legacy, "Graph"), panel);
}

#[test]
fn workspace_snapshot_panel_instance_references_survive_validation_and_restore() {
    let descriptors = workspace_panel_descriptors();
    let instances = workspace_panel_instances();
    let snapshot = nested_dock().workspace_snapshot(instances);

    snapshot
        .validate(&descriptors)
        .expect("workspace validates");
    assert_eq!(
        snapshot.panel_instances[0].state_key.as_deref(),
        Some("media-state")
    );

    let restored = Dock::restore_workspace(snapshot.clone(), &descriptors).expect("restore");
    assert_eq!(restored.snapshot(), snapshot.dock);

    let restored_workspace = restored.workspace_snapshot(snapshot.panel_instances.clone());
    assert_eq!(restored_workspace, snapshot);
    restored_workspace
        .validate(&descriptors)
        .expect("restored workspace validates");
}

#[test]
fn workspace_snapshot_rejects_missing_panel_instance_record() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances.retain(|instance| instance.id != PanelInstanceId::from_raw(3));
    let snapshot = nested_dock().workspace_snapshot(instances);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("missing instance"),
        WorkspaceRestoreError::MissingPanelInstance {
            panel_instance: PanelInstanceId::from_raw(3),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_unknown_panel_type() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[1].panel_type = PanelTypeId::from_raw(999);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("unknown panel type"),
        WorkspaceRestoreError::UnknownPanelType {
            panel_instance: PanelInstanceId::from_raw(2),
            panel_type: PanelTypeId::from_raw(999),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_duplicate_panel_instance_ids() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[2].id = PanelInstanceId::from_raw(2);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("duplicate panel instance"),
        WorkspaceRestoreError::DuplicatePanelInstanceId {
            panel_instance: PanelInstanceId::from_raw(2),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_duplicate_panel_type_descriptors_deterministically() {
    let mut descriptors = workspace_panel_descriptors();
    descriptors.push(PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Second Viewport",
    ));
    let snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("duplicate descriptor"),
        WorkspaceRestoreError::DuplicatePanelTypeDescriptor {
            panel_type: PanelTypeId::from_raw(20),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_stale_panel_instance_records_deterministically() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));

    assert_eq!(
        snapshot.validate(&descriptors).expect_err("stale instance"),
        WorkspaceRestoreError::StalePanelInstance {
            panel_instance: PanelInstanceId::from_raw(99),
        }
    );
}

fn invalid_dock_diagnostic_snapshot() -> DockSnapshot {
    DockSnapshot {
        active_frame: Some(FrameId::from_raw(99)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 1.25,
            min_first: -1.0,
            min_second: f32::INFINITY,
            first: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![],
                active: 1,
                dismissible_panels: vec![],
            }),
            second: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(2, "A"), panel(2, "Duplicate")],
                active: 3,
                dismissible_panels: vec![PanelId::from_raw(9), PanelId::from_raw(9)],
            }),
        },
    }
}

#[test]
fn dock_snapshot_diagnostics_report_stable_codes() {
    let snapshot = invalid_dock_diagnostic_snapshot();

    let diagnostics = snapshot.diagnostics();
    let codes = diagnostics
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(diagnostics.has_errors());
    assert_eq!(
        codes,
        vec![
            "dock.invalid_split_ratio",
            "dock.invalid_split_minimum",
            "dock.invalid_split_minimum",
            "dock.empty_frame",
            "dock.invalid_active_panel_index",
            "dock.duplicate_frame_id",
            "dock.invalid_active_panel_index",
            "dock.duplicate_panel_id",
            "dock.invalid_dismissible_panel",
            "dock.duplicate_dismissible_policy",
            "dock.invalid_dismissible_panel",
            "dock.invalid_active_frame",
        ]
    );
}

#[test]
fn dock_snapshot_diagnostics_report_context() {
    let snapshot = invalid_dock_diagnostic_snapshot();

    let diagnostics = snapshot.diagnostics();

    assert_eq!(diagnostics.diagnostics[0].path, DockSplitPath::root());
    assert_eq!(
        diagnostics.diagnostics[0].split_value,
        Some(DockSnapshotSplitValue::Ratio)
    );
    assert_eq!(
        diagnostics.diagnostics[1].split_value,
        Some(DockSnapshotSplitValue::MinFirst)
    );
    assert_eq!(
        diagnostics.diagnostics[2].split_value,
        Some(DockSnapshotSplitValue::MinSecond)
    );
    assert_eq!(diagnostics.diagnostics[3].stable_code(), "dock.empty_frame");
    assert_eq!(diagnostics.diagnostics[3].frame, Some(FrameId::from_raw(1)));
    assert_eq!(
        diagnostics.diagnostics[3].path,
        DockSplitPath::root().child(DockPathElement::First)
    );

    let duplicate_frame = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::DuplicateFrameId)
        .expect("duplicate frame diagnostic");
    assert_eq!(duplicate_frame.frame, Some(FrameId::from_raw(1)));
    assert_eq!(
        duplicate_frame.path,
        DockSplitPath::root().child(DockPathElement::Second)
    );

    let duplicate_panel = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::DuplicatePanelId)
        .expect("duplicate panel diagnostic");
    assert_eq!(duplicate_panel.frame, Some(FrameId::from_raw(1)));
    assert_eq!(duplicate_panel.panel, Some(PanelId::from_raw(2)));

    let invalid_active_panel = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DockSnapshotDiagnosticCode::InvalidActivePanelIndex
                && diagnostic.panel_count == Some(2)
        })
        .expect("invalid active panel diagnostic");
    assert_eq!(invalid_active_panel.active_index, Some(3));

    let invalid_dismissible = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::InvalidDismissiblePanel)
        .expect("invalid dismissible diagnostic");
    assert_eq!(invalid_dismissible.panel, Some(PanelId::from_raw(9)));

    let duplicate_dismissible = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DockSnapshotDiagnosticCode::DuplicateDismissiblePolicy
        })
        .expect("duplicate dismissible diagnostic");
    assert_eq!(duplicate_dismissible.panel, Some(PanelId::from_raw(9)));

    let invalid_active_frame = diagnostics
        .diagnostics
        .last()
        .expect("invalid active frame diagnostic");
    assert_eq!(
        invalid_active_frame.code,
        DockSnapshotDiagnosticCode::InvalidActiveFrame
    );
    assert_eq!(invalid_active_frame.frame, Some(FrameId::from_raw(99)));
}

#[test]
fn dock_snapshot_diagnostics_are_repeatable() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(99)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![],
            active: 0,
            dismissible_panels: vec![],
        },
    };

    assert_eq!(snapshot.diagnostics(), snapshot.diagnostics());
}

#[test]
fn workspace_snapshot_diagnostics_report_stable_codes_and_context() {
    let mut descriptors = workspace_panel_descriptors();
    descriptors.push(PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Second Viewport",
    ));

    let mut instances = workspace_panel_instances();
    instances[0].title = "Renamed Media".to_owned();
    instances[1].panel_type = PanelTypeId::from_raw(999);
    instances[2].id = PanelInstanceId::from_raw(2);
    instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));
    let snapshot = nested_dock().workspace_snapshot(instances);

    let diagnostics = snapshot.diagnostics(&descriptors);
    let codes = diagnostics
        .workspace
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(diagnostics.dock.is_valid());
    assert!(diagnostics.has_errors());
    assert_eq!(
        codes,
        vec![
            "workspace.duplicate_panel_type_descriptor",
            "workspace.duplicate_panel_instance_id",
            "workspace.unknown_panel_type",
            "workspace.missing_panel_instance",
            "workspace.stale_panel_instance",
            "workspace.panel_title_drift",
        ]
    );

    let duplicate_type = &diagnostics.workspace[0];
    assert_eq!(duplicate_type.panel_type, Some(PanelTypeId::from_raw(20)));

    let duplicate_instance = &diagnostics.workspace[1];
    assert_eq!(
        duplicate_instance.panel_instance,
        Some(PanelInstanceId::from_raw(2))
    );

    let unknown_type = &diagnostics.workspace[2];
    assert_eq!(
        unknown_type.panel_instance,
        Some(PanelInstanceId::from_raw(2))
    );
    assert_eq!(unknown_type.panel_type, Some(PanelTypeId::from_raw(999)));

    let missing_instance = &diagnostics.workspace[3];
    assert_eq!(
        missing_instance.panel_instance,
        Some(PanelInstanceId::from_raw(3))
    );
    assert_eq!(missing_instance.frame, Some(FrameId::from_raw(2)));
    assert_eq!(missing_instance.panel, Some(PanelId::from_raw(3)));
    assert_eq!(missing_instance.dock_title.as_deref(), Some("Inspector"));

    let stale_instance = &diagnostics.workspace[4];
    assert_eq!(
        stale_instance.panel_instance,
        Some(PanelInstanceId::from_raw(99))
    );
    assert_eq!(stale_instance.panel_type, Some(PanelTypeId::from_raw(10)));
    assert_eq!(
        stale_instance.instance_title.as_deref(),
        Some("Stale Media")
    );

    let title_drift = &diagnostics.workspace[5];
    assert_eq!(title_drift.severity, SnapshotDiagnosticSeverity::Warning);
    assert_eq!(title_drift.stable_code(), "workspace.panel_title_drift");
    assert_eq!(
        title_drift.panel_instance,
        Some(PanelInstanceId::from_raw(1))
    );
    assert_eq!(title_drift.frame, Some(FrameId::from_raw(1)));
    assert_eq!(title_drift.panel, Some(PanelId::from_raw(1)));
    assert_eq!(title_drift.dock_title.as_deref(), Some("Media"));
    assert_eq!(title_drift.instance_title.as_deref(), Some("Renamed Media"));
}

#[test]
fn workspace_snapshot_title_drift_is_a_warning_not_a_restore_error() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances[0].title = "Renamed Media".to_owned();
    let snapshot = nested_dock().workspace_snapshot(instances);

    let diagnostics = snapshot.diagnostics(&descriptors);

    assert!(diagnostics.is_valid());
    assert_eq!(diagnostics.workspace.len(), 1);
    assert_eq!(
        diagnostics.workspace[0].code,
        WorkspaceSnapshotDiagnosticCode::PanelTitleDrift
    );
    assert_eq!(
        diagnostics.workspace[0].severity,
        SnapshotDiagnosticSeverity::Warning
    );
    snapshot
        .validate(&descriptors)
        .expect("title drift allowed");
}

#[test]
fn workspace_snapshot_diagnostics_are_repeatable() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));

    assert_eq!(
        snapshot.diagnostics(&descriptors),
        snapshot.diagnostics(&descriptors)
    );
}

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
    assert!(dock.drop_tab(
        drag,
        DockDropTarget::split(
            FrameId::from_raw(1),
            DockPlacement::Bottom,
            FrameId::from_raw(9),
        )
    ));

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
