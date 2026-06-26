//! Windowless Dock/Frame/Panel model conformance tests.

use kinetik_ui_core::{ActionId, Axis, IconId, Point, Rect, Size, Vec2};
use kinetik_ui_widgets::{
    Dock, DockDropTarget, DockNeighborDirection, DockNode, DockPathElement, DockPlacement,
    DockRestoreError, DockSnapshot, DockSnapshotDiagnosticCode, DockSnapshotNode,
    DockSnapshotSplitValue, DockSplitInsertion, DockSplitPath, DockSplitterContextAction,
    DockSplitterContextActionKind, DockSplitterSide, Frame, FrameId, FrameLayout, FrameNeighbors,
    FrameSplitAffordanceRequest, Panel, PanelAffordances, PanelClosePolicy, PanelDockHint,
    PanelDuplicatePolicy, PanelFloatPolicy, PanelId, PanelInstanceId, PanelInstanceLocation,
    PanelInstancePolicy, PanelInstanceSnapshot, PanelOpenActionMetadata, PanelOpenDecision,
    PanelPolicyContext, PanelPolicyMetadata, PanelPolicyUnavailableReason, PanelRegistry,
    PanelRegistryError, PanelTypeCategory, PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext,
    SnapshotDiagnosticSeverity, WorkspaceRestoreError, WorkspaceSnapshotDiagnosticCode,
    frame_neighbor, frame_tabs, resolve_dock_drop_target, resolve_dock_join_request,
    resolve_dock_splitter_context_actions, resolve_dock_swap_request,
    resolve_frame_split_affordance_request, resolve_panel_affordances, resolve_panel_close_request,
    resolve_panel_duplicate_request, resolve_panel_float_request, resolve_panel_open_decision,
    resolve_panel_policy_context, solve_dock_layout, solve_dock_neighbors, solve_dock_splitters,
    split_ratio_from_drag,
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

fn neighbors_for(neighbors: &[FrameNeighbors], frame: u64) -> FrameNeighbors {
    neighbors
        .iter()
        .find(|neighbors| neighbors.frame == FrameId::from_raw(frame))
        .copied()
        .expect("frame neighbors")
}

fn panel_ids(frame: &Frame) -> Vec<PanelId> {
    frame.panels.iter().map(|panel| panel.id).collect()
}

fn splitter_context_action(
    actions: &[DockSplitterContextAction],
    kind: DockSplitterContextActionKind,
    source_side: DockSplitterSide,
) -> &DockSplitterContextAction {
    actions
        .iter()
        .find(|action| action.kind == kind && action.source_side == source_side)
        .expect("splitter context action")
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
fn registry_preserves_descriptor_order_and_stable_lookup() {
    let registry =
        PanelRegistry::from_descriptors(workspace_panel_descriptors()).expect("registry");

    assert_eq!(
        registry
            .descriptors()
            .iter()
            .map(|descriptor| descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            PanelTypeId::from_raw(10),
            PanelTypeId::from_raw(20),
            PanelTypeId::from_raw(30),
            PanelTypeId::from_raw(40),
        ]
    );
    assert_eq!(
        registry
            .iter()
            .map(|descriptor| descriptor.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Media", "Viewport", "Inspector", "Timeline"]
    );
    assert_eq!(
        registry
            .descriptor(PanelTypeId::from_raw(30))
            .map(|descriptor| descriptor.title.as_str()),
        Some("Inspector")
    );
    assert_eq!(registry.descriptor(PanelTypeId::from_raw(999)), None);
}

#[test]
fn registry_rejects_duplicate_panel_type_ids_with_deterministic_context() {
    let descriptors = vec![
        PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Media"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Viewport"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Second Media"),
    ];

    assert_eq!(
        PanelRegistry::from_descriptors(descriptors).expect_err("duplicate descriptor"),
        PanelRegistryError::DuplicatePanelTypeDescriptor {
            panel_type: PanelTypeId::from_raw(10),
            first_index: 0,
            duplicate_index: 2,
        }
    );
}

#[test]
fn registry_iterates_categories_and_category_descriptors_in_presentation_order() {
    let descriptors = vec![
        PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Scene")
            .with_category(PanelTypeCategory::Hierarchy),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Inspector")
            .with_category(PanelTypeCategory::Inspector),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Outliner")
            .with_category(PanelTypeCategory::Hierarchy),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(40), "Console")
            .with_category(PanelTypeCategory::Diagnostics),
    ];
    let registry = PanelRegistry::from_descriptors(descriptors).expect("registry");

    assert_eq!(
        registry
            .categories()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            PanelTypeCategory::Hierarchy,
            PanelTypeCategory::Inspector,
            PanelTypeCategory::Diagnostics,
        ]
    );
    assert_eq!(
        registry
            .descriptors_in_category(&PanelTypeCategory::Hierarchy)
            .map(|descriptor| descriptor.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Scene", "Outliner"]
    );
}

#[test]
fn registry_open_decision_focuses_existing_singleton_instance() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(30),
        "Inspector",
    )
    .with_instance_policy(PanelInstancePolicy::Singleton)])
    .expect("registry");

    let decision = registry
        .resolve_open_decision(
            PanelTypeId::from_raw(30),
            &workspace_panel_instances(),
            &dock,
            PanelWorkspaceContext::Docked,
        )
        .expect("open decision");

    assert_eq!(
        decision,
        PanelOpenDecision::FocusExisting(kinetik_ui_widgets::PanelFocusRequest {
            metadata: PanelPolicyMetadata {
                panel_type: PanelTypeId::from_raw(30),
                title: "Inspector".to_owned(),
                default_open_action: None,
            },
            target: PanelInstanceLocation {
                panel_instance: PanelInstanceId::from_raw(3),
                panel: PanelId::from_raw(3),
                frame: FrameId::from_raw(2),
            },
        })
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn registry_open_decision_returns_open_new_metadata_for_multi_instance_panel() {
    let dock = nested_dock();
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(50),
        "Console",
    )
    .with_default_size(Size::new(480.0, 220.0))
    .with_dock_hints([
        PanelDockHint::Split(DockPlacement::Bottom),
        PanelDockHint::Tab,
    ])
    .with_default_open_action(ActionId::new("workspace.open.console"))])
    .expect("registry");

    let decision = registry
        .resolve_open_decision(
            PanelTypeId::from_raw(50),
            &workspace_panel_instances(),
            &dock,
            PanelWorkspaceContext::Docked,
        )
        .expect("open decision");

    let PanelOpenDecision::OpenNew(request) = decision else {
        panic!("multi-instance panel should open a new request");
    };
    assert_eq!(
        request.metadata,
        PanelPolicyMetadata {
            panel_type: PanelTypeId::from_raw(50),
            title: "Console".to_owned(),
            default_open_action: Some(ActionId::new("workspace.open.console")),
        }
    );
    assert_eq!(request.context, PanelWorkspaceContext::Docked);
    assert_eq!(
        request.dock_hint,
        Some(PanelDockHint::Split(DockPlacement::Bottom))
    );
    assert_eq!(request.default_size, Size::new(480.0, 220.0));
}

#[test]
fn registry_open_decision_returns_none_for_disallowed_or_unknown_panel_context() {
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(60),
        "Modal Only",
    )
    .with_allowed_contexts([PanelWorkspaceContext::Modal])])
    .expect("registry");

    assert_eq!(
        registry.resolve_open_decision(
            PanelTypeId::from_raw(60),
            &workspace_panel_instances(),
            &nested_dock(),
            PanelWorkspaceContext::Docked,
        ),
        None
    );
    assert_eq!(
        registry.resolve_open_decision(
            PanelTypeId::from_raw(999),
            &workspace_panel_instances(),
            &nested_dock(),
            PanelWorkspaceContext::Docked,
        ),
        None
    );
}

#[test]
fn registry_open_actions_are_app_owned_metadata_only() {
    let registry = PanelRegistry::from_descriptors([
        PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Viewport")
            .with_icon(IconId::from_raw(99))
            .with_category(PanelTypeCategory::Viewport)
            .with_default_open_action(ActionId::new("workspace.open.viewport")),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Inspector")
            .with_category(PanelTypeCategory::Inspector),
    ])
    .expect("registry");

    assert_eq!(
        registry.open_actions().collect::<Vec<_>>(),
        vec![
            PanelOpenActionMetadata {
                panel_type: PanelTypeId::from_raw(20),
                title: "Viewport".to_owned(),
                icon: Some(IconId::from_raw(99)),
                category: PanelTypeCategory::Viewport,
                default_open_action: Some(ActionId::new("workspace.open.viewport")),
            },
            PanelOpenActionMetadata {
                panel_type: PanelTypeId::from_raw(30),
                title: "Inspector".to_owned(),
                icon: None,
                category: PanelTypeCategory::Inspector,
                default_open_action: None,
            },
        ]
    );
}

#[test]
fn panel_policy_non_closeable_descriptor_suppresses_close_affordance() {
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Media")
        .with_close_policy(PanelClosePolicy::Required);
    let frame = frame(
        1,
        vec![Panel::from_instance_id(
            PanelInstanceId::from_raw(1),
            "Media",
        )],
    );

    let affordances = resolve_panel_affordances(&descriptor, PanelInstanceId::from_raw(1), &frame);

    assert!(frame.panel_dismissible(PanelId::from_raw(1)));
    assert!(!affordances.close_visible);
    assert_eq!(
        resolve_panel_close_request(&descriptor, PanelInstanceId::from_raw(1), &frame),
        None
    );
}

#[test]
fn panel_policy_frame_non_dismissible_tab_suppresses_close_affordance() {
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Media")
        .with_close_policy(PanelClosePolicy::Closable);
    let mut frame = frame(
        1,
        vec![Panel::from_instance_id(
            PanelInstanceId::from_raw(1),
            "Media",
        )],
    );
    assert!(frame.set_panel_dismissible(PanelId::from_raw(1), false));

    let affordances = resolve_panel_affordances(&descriptor, PanelInstanceId::from_raw(1), &frame);

    assert!(!affordances.close_visible);
    assert_eq!(
        resolve_panel_close_request(&descriptor, PanelInstanceId::from_raw(1), &frame),
        None
    );
}

#[test]
fn panel_policy_singleton_open_decision_focuses_existing_instance() {
    let dock = nested_dock();
    let before = dock.snapshot();
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Inspector")
        .with_instance_policy(PanelInstancePolicy::Singleton);

    let decision = resolve_panel_open_decision(
        &descriptor,
        &workspace_panel_instances(),
        &dock,
        PanelWorkspaceContext::Docked,
    )
    .expect("open decision");

    assert_eq!(
        decision,
        PanelOpenDecision::FocusExisting(kinetik_ui_widgets::PanelFocusRequest {
            metadata: PanelPolicyMetadata {
                panel_type: PanelTypeId::from_raw(30),
                title: "Inspector".to_owned(),
                default_open_action: None,
            },
            target: PanelInstanceLocation {
                panel_instance: PanelInstanceId::from_raw(3),
                panel: PanelId::from_raw(3),
                frame: FrameId::from_raw(2),
            },
        })
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn panel_policy_multi_instance_open_decision_allows_new_request() {
    let dock = nested_dock();
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(50), "Console")
        .with_default_size(Size::new(480.0, 220.0))
        .with_dock_hints([
            PanelDockHint::Split(DockPlacement::Bottom),
            PanelDockHint::Tab,
        ])
        .with_default_open_action(ActionId::new("workspace.open.console"));

    let decision = resolve_panel_open_decision(
        &descriptor,
        &workspace_panel_instances(),
        &dock,
        PanelWorkspaceContext::Docked,
    )
    .expect("open decision");

    let PanelOpenDecision::OpenNew(request) = decision else {
        panic!("multi-instance panel should open a new request");
    };
    assert_eq!(
        request.metadata,
        PanelPolicyMetadata {
            panel_type: PanelTypeId::from_raw(50),
            title: "Console".to_owned(),
            default_open_action: Some(ActionId::new("workspace.open.console")),
        }
    );
    assert_eq!(request.context, PanelWorkspaceContext::Docked);
    assert_eq!(
        request.dock_hint,
        Some(PanelDockHint::Split(DockPlacement::Bottom))
    );
    assert_eq!(request.default_size, Size::new(480.0, 220.0));
}

#[test]
fn panel_policy_duplicate_request_respects_descriptor_and_is_app_owned() {
    let descriptor = PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Viewport")
        .with_default_size(Size::new(640.0, 360.0))
        .with_default_open_action(ActionId::new("workspace.open.viewport"));
    let singleton = descriptor
        .clone()
        .with_instance_policy(PanelInstancePolicy::Singleton);
    let denied = descriptor
        .clone()
        .with_duplicate_policy(PanelDuplicatePolicy::Denied);
    let dock = nested_dock();
    let before = dock.snapshot();
    let frame = dock.frame(FrameId::from_raw(2)).expect("frame");

    assert!(
        !resolve_panel_affordances(&singleton, PanelInstanceId::from_raw(2), frame)
            .duplicate_available
    );
    assert_eq!(
        resolve_panel_duplicate_request(
            &singleton,
            PanelInstanceId::from_raw(2),
            frame,
            PanelWorkspaceContext::Docked,
        ),
        None
    );
    assert_eq!(
        resolve_panel_duplicate_request(
            &denied,
            PanelInstanceId::from_raw(2),
            frame,
            PanelWorkspaceContext::Docked,
        ),
        None
    );

    let request = resolve_panel_duplicate_request(
        &descriptor,
        PanelInstanceId::from_raw(2),
        frame,
        PanelWorkspaceContext::Docked,
    )
    .expect("duplicate request");

    assert_eq!(
        request.metadata,
        PanelPolicyMetadata {
            panel_type: PanelTypeId::from_raw(20),
            title: "Viewport".to_owned(),
            default_open_action: Some(ActionId::new("workspace.open.viewport")),
        }
    );
    assert_eq!(
        request.source,
        PanelInstanceLocation {
            panel_instance: PanelInstanceId::from_raw(2),
            panel: PanelId::from_raw(2),
            frame: FrameId::from_raw(2),
        }
    );
    assert_eq!(request.context, PanelWorkspaceContext::Docked);
    assert_eq!(request.dock_hint, Some(PanelDockHint::Tab));
    assert_eq!(request.default_size, Size::new(640.0, 360.0));
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn panel_policy_future_float_request_is_metadata_only() {
    let unavailable = PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Inspector")
        .with_float_policy(PanelFloatPolicy::Unavailable);
    let allowed = unavailable
        .clone()
        .with_float_policy(PanelFloatPolicy::Allowed);
    let dock = nested_dock();
    let before = dock.snapshot();
    let frame = dock.frame(FrameId::from_raw(2)).expect("frame");

    assert!(
        !resolve_panel_affordances(&unavailable, PanelInstanceId::from_raw(3), frame)
            .float_available
    );
    assert_eq!(
        resolve_panel_float_request(&unavailable, PanelInstanceId::from_raw(3), frame),
        None
    );

    let request =
        resolve_panel_float_request(&allowed, PanelInstanceId::from_raw(3), frame).expect("float");

    assert!(
        resolve_panel_affordances(&allowed, PanelInstanceId::from_raw(3), frame).float_available
    );
    assert_eq!(
        request.source,
        PanelInstanceLocation {
            panel_instance: PanelInstanceId::from_raw(3),
            panel: PanelId::from_raw(3),
            frame: FrameId::from_raw(2),
        }
    );
    assert_eq!(request.metadata.panel_type, PanelTypeId::from_raw(30));
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn panel_policy_context_resolves_registry_instance_frame_and_requests() {
    let mut dock = nested_dock();
    assert!(
        dock.frame_mut(FrameId::from_raw(2))
            .expect("frame")
            .set_panel_dismissible(PanelId::from_raw(2), false)
    );
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Viewport",
    )
    .with_default_size(Size::new(640.0, 360.0))
    .with_default_open_action(ActionId::new("workspace.open.viewport"))
    .with_float_policy(PanelFloatPolicy::Allowed)])
    .expect("registry");

    let resolution = registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );

    assert!(resolution.is_available());
    assert_eq!(resolution.unavailable, None);
    assert_eq!(resolution.panel_type, Some(PanelTypeId::from_raw(20)));
    assert_eq!(
        resolution.location,
        Some(PanelInstanceLocation {
            panel_instance: PanelInstanceId::from_raw(2),
            panel: PanelId::from_raw(2),
            frame: FrameId::from_raw(2),
        })
    );
    assert_eq!(
        resolution.affordances,
        Some(PanelAffordances {
            panel_type: PanelTypeId::from_raw(20),
            panel_instance: PanelInstanceId::from_raw(2),
            close_visible: false,
            duplicate_available: true,
            float_available: true,
        })
    );
    assert!(resolution.close_request.is_none());
    assert!(matches!(
        resolution.open_decision,
        Some(PanelOpenDecision::OpenNew(_))
    ));
    assert_eq!(
        resolution
            .duplicate_request
            .as_ref()
            .expect("duplicate")
            .source
            .panel_instance,
        PanelInstanceId::from_raw(2)
    );
    assert_eq!(
        resolution
            .float_request
            .as_ref()
            .expect("float")
            .source
            .panel_instance,
        PanelInstanceId::from_raw(2)
    );
}

#[test]
fn panel_policy_context_reports_missing_descriptor_with_location_context() {
    let dock = nested_dock();
    let registry = PanelRegistry::new();

    let resolution = registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );

    assert_eq!(
        resolution.unavailable,
        Some(PanelPolicyUnavailableReason::MissingDescriptor)
    );
    assert_eq!(resolution.panel_type, Some(PanelTypeId::from_raw(20)));
    assert_eq!(
        resolution.location.expect("location").frame,
        FrameId::from_raw(2)
    );
    assert!(resolution.affordances.is_none());
    assert!(resolution.open_decision.is_none());
    assert!(resolution.duplicate_request.is_none());
}

#[test]
fn panel_policy_context_reports_missing_instance_location_and_frame_membership() {
    let dock = nested_dock();
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Viewport",
    )])
    .expect("registry");

    let missing_instance = registry.resolve_policy_context(
        &workspace_panel_instances()
            .into_iter()
            .filter(|instance| instance.id != PanelInstanceId::from_raw(2))
            .collect::<Vec<_>>(),
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );
    assert_eq!(
        missing_instance.unavailable,
        Some(PanelPolicyUnavailableReason::MissingPanelInstance)
    );
    assert_eq!(missing_instance.panel_type, None);
    assert_eq!(missing_instance.location, None);

    let missing_location_instances = [PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(20),
        "Detached Viewport",
    )];
    let missing_location = registry.resolve_policy_context(
        &missing_location_instances,
        &dock,
        PanelInstanceId::from_raw(99),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );
    assert_eq!(
        missing_location.unavailable,
        Some(PanelPolicyUnavailableReason::MissingPanelLocation)
    );
    assert_eq!(missing_location.panel_type, Some(PanelTypeId::from_raw(20)));
    assert_eq!(missing_location.location, None);

    let missing_membership = registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(1),
        PanelWorkspaceContext::Docked,
    );
    assert_eq!(
        missing_membership.unavailable,
        Some(PanelPolicyUnavailableReason::MissingFrameMembership)
    );
    assert_eq!(
        missing_membership.location.expect("actual location").frame,
        FrameId::from_raw(2)
    );
    assert!(missing_membership.affordances.is_none());
}

#[test]
fn panel_policy_context_denies_singleton_duplicate_and_disallowed_context_requests() {
    let dock = nested_dock();
    let singleton_registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(30),
        "Inspector",
    )
    .with_instance_policy(PanelInstancePolicy::Singleton)])
    .expect("registry");

    let singleton = singleton_registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(3),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );

    assert!(singleton.is_available());
    assert!(
        !singleton
            .affordances
            .expect("singleton affordances")
            .duplicate_available
    );
    assert!(singleton.duplicate_request.is_none());
    assert!(matches!(
        singleton.open_decision,
        Some(PanelOpenDecision::FocusExisting(_))
    ));

    let modal_only_registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Viewport",
    )
    .with_allowed_contexts([PanelWorkspaceContext::Modal])])
    .expect("registry");

    let disallowed = modal_only_registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );

    assert_eq!(
        disallowed.unavailable,
        Some(PanelPolicyUnavailableReason::DisallowedContext)
    );
    assert!(disallowed.affordances.is_some());
    assert!(disallowed.open_decision.is_none());
    assert!(disallowed.close_request.is_none());
    assert!(disallowed.duplicate_request.is_none());
    assert!(disallowed.float_request.is_none());
}

#[test]
fn panel_policy_context_float_request_is_metadata_only_when_allowed() {
    let dock = nested_dock();
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(30),
        "Inspector",
    )
    .with_float_policy(PanelFloatPolicy::Allowed)])
    .expect("registry");
    let before = dock.snapshot();

    let resolution = registry.resolve_policy_context(
        &workspace_panel_instances(),
        &dock,
        PanelInstanceId::from_raw(3),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    );

    assert!(resolution.is_available());
    assert_eq!(
        resolution.float_request.expect("float").source,
        PanelInstanceLocation {
            panel_instance: PanelInstanceId::from_raw(3),
            panel: PanelId::from_raw(3),
            frame: FrameId::from_raw(2),
        }
    );
    assert_eq!(dock.snapshot(), before);
}

#[test]
fn panel_policy_context_resolver_is_pure_metadata() {
    let dock = nested_dock();
    let dock_before = dock.snapshot();
    let registry = PanelRegistry::from_descriptors([PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Viewport",
    )
    .with_float_policy(PanelFloatPolicy::Allowed)])
    .expect("registry");
    let registry_before = registry.clone();
    let instances = workspace_panel_instances();
    let instances_before = instances.clone();

    let resolution = resolve_panel_policy_context(PanelPolicyContext::new(
        &registry,
        &instances,
        &dock,
        PanelInstanceId::from_raw(2),
        FrameId::from_raw(2),
        PanelWorkspaceContext::Docked,
    ));

    assert!(resolution.is_available());
    assert!(resolution.open_decision.is_some());
    assert_eq!(dock.snapshot(), dock_before);
    assert_eq!(registry, registry_before);
    assert_eq!(instances, instances_before);
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
