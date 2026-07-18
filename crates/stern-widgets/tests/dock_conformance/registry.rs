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
        .with_icon(stern_icons_phosphor::regular::SIDEBAR)
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

    assert_eq!(
        descriptor.icon,
        Some(stern_icons_phosphor::regular::SIDEBAR.icon())
    );
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
        PanelOpenDecision::FocusExisting(stern_widgets::PanelFocusRequest {
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
            .with_icon(stern_icons_phosphor::regular::SIDEBAR)
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
                icon: Some(stern_icons_phosphor::regular::SIDEBAR.icon()),
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
