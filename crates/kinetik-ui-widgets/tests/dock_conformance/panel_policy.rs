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
