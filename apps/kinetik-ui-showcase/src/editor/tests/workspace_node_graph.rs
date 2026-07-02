#[test]
fn default_workspace_snapshot_validates_against_showcase_panel_descriptors() {
    let registry = super::editor_panel_registry();
    let snapshot = super::default_workspace_snapshot();
    let diagnostics = snapshot.diagnostics(registry.descriptors());

    assert!(diagnostics.is_valid(), "{diagnostics:?}");
    assert!(diagnostics.dock.diagnostics.is_empty(), "{diagnostics:?}");
    assert!(diagnostics.workspace.is_empty(), "{diagnostics:?}");
    snapshot
        .validate(registry.descriptors())
        .expect("workspace validates");
    assert_eq!(
        snapshot.panel_instances,
        super::editor_panel_instances(),
        "default workspace instances should be deterministic"
    );
}

#[test]
fn default_workspace_snapshot_round_trips_through_workspace_restore() {
    let registry = super::editor_panel_registry();
    let snapshot = super::default_workspace_snapshot();
    let restored =
        super::Dock::restore_workspace(snapshot.clone(), registry.descriptors()).expect("restore");

    assert_eq!(restored.snapshot(), snapshot.dock);
    assert_eq!(
        restored.workspace_snapshot(super::editor_panel_instances()),
        snapshot
    );
}

#[test]
fn editor_panel_registry_builds_unique_showcase_descriptors() {
    let registry = super::editor_panel_registry();

    assert_eq!(registry.descriptors().len(), 7);
    assert_eq!(
        registry.descriptors(),
        super::editor_panel_type_descriptors().as_slice()
    );
    assert_eq!(
        registry
            .descriptor(super::PANEL_TYPE_NODE_GRAPH)
            .expect("node graph descriptor")
            .title,
        "Node Graph"
    );
}

#[test]
fn registry_open_metadata_exposes_editor_vocabulary_in_stable_order() {
    let registry = super::editor_panel_registry();
    let metadata = super::editor_open_panel_metadata();
    let titles = metadata
        .iter()
        .map(|metadata| metadata.title.as_str())
        .collect::<Vec<_>>();
    let action_ids = metadata
        .iter()
        .map(|metadata| {
            metadata
                .default_open_action
                .as_ref()
                .expect("open action")
                .as_str()
        })
        .collect::<Vec<_>>();
    let categories = registry
        .categories()
        .into_iter()
        .map(super::panel_category_label)
        .collect::<Vec<_>>();

    assert_eq!(
        titles,
        [
            "Viewport",
            "Explorer",
            "Properties",
            "Asset Browser",
            "Timeline",
            "Console",
            "Node Graph",
        ]
    );
    assert_eq!(
        action_ids,
        [
            super::ACTION_OPEN_VIEWPORT,
            super::ACTION_OPEN_EXPLORER,
            super::ACTION_OPEN_PROPERTIES,
            super::ACTION_OPEN_ASSET_BROWSER,
            super::ACTION_OPEN_TIMELINE,
            super::ACTION_OPEN_CONSOLE,
            super::ACTION_OPEN_NODE_GRAPH,
        ]
    );
    assert_eq!(
        metadata
            .iter()
            .map(|metadata| metadata.category.clone())
            .collect::<Vec<_>>(),
        [
            PanelTypeCategory::Viewport,
            PanelTypeCategory::Hierarchy,
            PanelTypeCategory::Inspector,
            PanelTypeCategory::Assets,
            PanelTypeCategory::Timeline,
            PanelTypeCategory::Diagnostics,
            PanelTypeCategory::Timeline,
        ]
    );
    assert_eq!(
        categories,
        [
            "Viewport",
            "Hierarchy",
            "Inspector",
            "Assets",
            "Timeline",
            "Diagnostics",
        ]
    );
}

#[test]
fn default_workspace_snapshot_contains_roblox_blender_style_vocabulary() {
    let snapshot = super::default_workspace_snapshot();
    let titles = snapshot
        .panel_instances
        .iter()
        .map(|instance| instance.title.as_str())
        .collect::<Vec<_>>();
    let state_keys = snapshot
        .panel_instances
        .iter()
        .map(|instance| instance.state_key.as_deref().expect("state key"))
        .collect::<Vec<_>>();

    assert_eq!(
        titles,
        [
            "Explorer",
            "Asset Browser",
            "Viewport",
            "Console",
            "Timeline",
            "Properties",
            "Node Graph",
        ]
    );
    assert_eq!(
        state_keys,
        [
            "editor.explorer",
            "editor.asset-browser",
            "editor.viewport",
            "editor.console",
            "editor.timeline",
            "editor.properties",
            "editor.node-graph",
        ]
    );
}

#[test]
fn registry_open_or_focus_workflow_is_app_owned_and_deterministic() {
    let mut editor = EditorShowcase::new();
    let registry = super::editor_panel_registry();
    let instances = super::editor_panel_instances();
    let decision = registry
        .resolve_open_decision(
            super::PANEL_TYPE_NODE_GRAPH,
            &instances,
            &editor.dock,
            super::PanelWorkspaceContext::Docked,
        )
        .expect("open decision");

    assert!(matches!(decision, PanelOpenDecision::FocusExisting(_)));
    assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));
    assert_eq!(editor.status, "Focused Node Graph");
    assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
    assert_eq!(
        editor
            .dock
            .frame(FRAME_BOTTOM)
            .and_then(|frame| frame.active_panel())
            .map(|panel| panel.id),
        Some(super::PANEL_NODE_GRAPH)
    );

    assert!(editor.apply_action(super::ACTION_OPEN_PROPERTIES));
    assert_eq!(editor.status, "Focused Properties");
    assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
}

#[test]
fn editor_node_graph_panel_exercises_stage9_contracts() {
    let mut editor = EditorShowcase::new();
    assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));

    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(editor_test_context(UiInput::default()), &mut memory, &theme);
    editor.render(&mut ui, 0);
    let frame = ui.finish_output();

    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Custom("node-graph".to_owned())
            && node.label.as_deref() == Some("Node graph")
    }));

    let body = Rect::new(20.0, 40.0, 480.0, 180.0);
    let viewport = super::EditorShowcase::showcase_node_graph_viewport(body);
    let graph = super::EditorShowcase::showcase_node_graph_descriptor();
    graph.validate().expect("showcase graph validates");

    let output = super::EditorShowcase::showcase_node_graph_output(
        WidgetId::from_key("showcase-node-graph"),
        viewport,
    )
    .expect("showcase graph emits static output");
    assert!(matches!(
        output.primitives.first(),
        Some(Primitive::ClipBegin { .. })
    ));
    assert!(matches!(
        output.primitives.last(),
        Some(Primitive::ClipEnd { .. })
    ));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("node".to_owned())
            && node.label.as_deref() == Some("Color Grade")
            && node.state.selected
    }));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("edge".to_owned())
            && node.label.as_deref() == Some("Edge 51: Color Grade Out to Output Surface")
            && node.state.selected
    }));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("port".to_owned())
            && node.label.as_deref() == Some("Input Mask")
            && node.description.as_deref() == Some("Incompatible port")
    }));

    let color_grade_center = viewport.graph_rect_to_screen(graph.nodes[1].rect).center();
    assert_eq!(
        graph
            .hit_test(viewport, color_grade_center)
            .expect("node hit target"),
        NodeGraphHitTarget::NodeBody(NodeId::from_raw(2))
    );

    let selection =
        NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(2)));
    let context_actions = graph.context_actions(
        NodeGraphContextTarget::Node(NodeId::from_raw(2)),
        &selection,
    );
    assert!(
        context_actions
            .iter()
            .any(|action| { action.kind == NodeGraphContextActionKind::Delete && action.enabled })
    );
    assert!(context_actions.iter().any(|action| {
        action.kind == NodeGraphContextActionKind::FrameSelection && action.enabled
    }));

    let link_request = graph
        .create_link_request(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
        )
        .expect("link request metadata");
    assert!(matches!(
        link_request,
        NodeGraphLinkEditRequest::CreateLink(_)
    ));

    let move_request = graph
        .move_frame_request(
            viewport,
            NodeFrameId::from_raw(1),
            GraphVector::new(20.0, -10.0),
        )
        .expect("frame move metadata");
    assert_eq!(move_request.children.len(), 2);
    assert_eq!(move_request.graph_delta, GraphVector::new(20.0, -10.0));
}
