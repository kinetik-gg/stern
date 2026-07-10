use super::{
    ActionId, ComponentCategory, ComponentConformanceStatus, EdgeDescriptor, GraphRect, JobList,
    JobPhase, JobProgress, JobRow, JobRowId, NodeDescriptor, NodeGraphDescriptor, NodeGraphPanZoom,
    NodeGraphSelection, NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphViewport, NodeId,
    PanZoom, PanelId, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId, Rect,
    SemanticRole, Size, TabStrip, TextureId, TimelineDescriptor, TimelineFrameRate, TimelineId,
    TimelineItemDescriptor, TimelineItemId, TimelineLaneDescriptor, TimelineLaneId, TimelineRange,
    TimelineRulerTickRequest, TimelineSelection, TimelineSelectionTarget,
    TimelineSnapCandidateRequest, TimelineSnapSource, TimelineZoom, TransportControlIntent,
    TransportControls, Vec2, ViewportCursorRequestSource, ViewportCursorShape, ViewportFit,
    ViewportGuideDescriptor, ViewportGuideId, ViewportGuideOrientation, ViewportGuidePlacement,
    ViewportPanZoomHudDescriptor, ViewportRulerDescriptor, ViewportRulerEdge, ViewportRulerId,
    ViewportSafeAreaDescriptor, ViewportSafeAreaId, ViewportSafeAreaSpace, ViewportSurface,
    WidgetId, assert_close, assert_entry, assert_stage_entry, timeline_snap_candidates,
    viewport_action_and_cursor_contracts, viewport_guides, viewport_rulers, viewport_safe_areas,
};

#[test]
fn representative_components_report_honest_categories_and_statuses() {
    assert_entry(
        "Button",
        ComponentCategory::Control,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "TextField",
        ComponentCategory::TextEditing,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Dock",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Table",
        ComponentCategory::Collection,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "CommandPalette",
        ComponentCategory::Overlay,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Viewport",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "NodeGraph",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "StatusBar",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );
}
#[test]
fn stage9_basic_components_report_current_conformance_statuses() {
    assert_entry(
        "Label",
        ComponentCategory::Display,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Button",
        ComponentCategory::Control,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "IconButton",
        ComponentCategory::Control,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Checkbox",
        ComponentCategory::Input,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "RadioButton",
        ComponentCategory::Input,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Toggle",
        ComponentCategory::Input,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Slider",
        ComponentCategory::Input,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Panel",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Experimental,
    );
}

#[test]
fn stage1_basic_control_matrix_reports_experimental_statuses() {
    for (name, category) in [
        ("TextField", ComponentCategory::TextEditing),
        ("MultiLineTextField", ComponentCategory::TextEditing),
        ("SearchField", ComponentCategory::TextEditing),
        ("NumericInput", ComponentCategory::Input),
        ("NumericScrubInput", ComponentCategory::Input),
        ("Button", ComponentCategory::Control),
        ("IconButton", ComponentCategory::Control),
        ("Checkbox", ComponentCategory::Input),
        ("RadioButton", ComponentCategory::Input),
        ("Toggle", ComponentCategory::Input),
        ("Slider", ComponentCategory::Input),
    ] {
        assert_entry(name, category, ComponentConformanceStatus::Experimental);
    }
}

#[test]
fn stage2_control_taxonomy_reports_honest_statuses() {
    for (name, category, status) in [
        (
            "Dropdown",
            ComponentCategory::Overlay,
            ComponentConformanceStatus::Experimental,
        ),
        (
            "Slider",
            ComponentCategory::Input,
            ComponentConformanceStatus::Experimental,
        ),
        (
            "NumericInput",
            ComponentCategory::Input,
            ComponentConformanceStatus::Experimental,
        ),
        (
            "NumericScrubInput",
            ComponentCategory::Input,
            ComponentConformanceStatus::Experimental,
        ),
        (
            "RadioButton",
            ComponentCategory::Input,
            ComponentConformanceStatus::Experimental,
        ),
        (
            "PropertyGrid",
            ComponentCategory::Inspector,
            ComponentConformanceStatus::Experimental,
        ),
    ] {
        assert_entry(name, category, status);
    }
}

#[test]
fn stage7_vector_and_color_fields_report_experimental_inspector_statuses() {
    for name in [
        "PropertyAffordanceControls",
        "Vector2Field",
        "Vector3Field",
        "Vector4Field",
        "ColorField",
        "SelectField",
        "AssetSlotField",
        "PathField",
    ] {
        assert_entry(
            name,
            ComponentCategory::Inspector,
            ComponentConformanceStatus::Experimental,
        );
    }
}

#[test]
fn component_taxonomy_conformance_reports_stage6_status_bar_experimental() {
    assert_entry(
        "StatusBar",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );
}

#[test]
fn component_taxonomy_conformance_reports_stage6_tabs_experimental() {
    assert_entry(
        "Tabs",
        ComponentCategory::Docking,
        ComponentConformanceStatus::Experimental,
    );

    let strip = TabStrip::from_tabs([
        kinetik_ui_widgets::FrameTab {
            panel: PanelId::from_raw(1),
            title: "Viewport".to_owned(),
            active: true,
            close_visible: true,
            draggable: true,
        },
        kinetik_ui_widgets::FrameTab {
            panel: PanelId::from_raw(2),
            title: "Inspector".to_owned(),
            active: false,
            close_visible: false,
            draggable: true,
        },
    ]);

    assert_eq!(strip.active_panel(), Some(PanelId::from_raw(1)));
    assert_eq!(
        strip
            .activation_target_by_panel(PanelId::from_raw(2))
            .map(|target| target.index),
        Some(1)
    );
}

#[test]
fn component_taxonomy_conformance_reports_stage6_modal_experimental() {
    assert_entry(
        "Modal",
        ComponentCategory::Overlay,
        ComponentConformanceStatus::Experimental,
    );
}

#[test]
fn stage9_node_graph_taxonomy_reports_experimental_status_backed_by_public_contracts() {
    assert_entry(
        "NodeGraph",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );

    let color = PortTypeId::from_raw(1);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", color),
            ]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Target",
                GraphRect::new(160.0, 0.0, 100.0, 70.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(1),
                PortDirection::Input,
                "In",
                color,
            )]),
        ],
        edges: vec![EdgeDescriptor::new(
            kinetik_ui_widgets::EdgeId::from_raw(1),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(1)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    graph.validate().expect("taxonomy node graph validates");

    let output = NodeGraphStaticView::new(
        WidgetId::from_key("taxonomy-node-graph"),
        NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 320.0, 160.0),
            NodeGraphPanZoom::default(),
        ),
        &graph,
    )
    .with_selection(
        NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(1))),
    )
    .emit()
    .expect("node graph static output");

    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("node-graph".to_owned())
            && node.label.as_deref() == Some("Node graph")
    }));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("node".to_owned())
            && node.label.as_deref() == Some("Source")
            && node.state.selected
    }));
}

#[test]
fn stage10_to_stage13_entries_report_experimental_status_with_evidence_categories() {
    for (name, stage, category) in [
        ("Outliner", 10, ComponentCategory::Collection),
        ("AssetBrowser", 10, ComponentCategory::Collection),
        ("Timeline", 11, ComponentCategory::Viewport),
        ("Ruler", 11, ComponentCategory::Viewport),
        ("TransportControls", 11, ComponentCategory::Control),
        ("Viewport", 12, ComponentCategory::Viewport),
        ("ViewportTools", 12, ComponentCategory::Viewport),
        ("ViewportActionRouting", 12, ComponentCategory::Viewport),
        ("StatusBar", 13, ComponentCategory::System),
        ("ProgressIndicator", 13, ComponentCategory::Display),
        ("JobList", 13, ComponentCategory::System),
        ("DiagnosticStrip", 13, ComponentCategory::System),
        ("FeedbackStack", 13, ComponentCategory::System),
    ] {
        assert_stage_entry(
            name,
            stage,
            category,
            ComponentConformanceStatus::Experimental,
        );
    }
}

#[test]
fn stage11_timeline_taxonomy_reports_experimental_status_backed_by_public_contracts() {
    assert_entry(
        "Timeline",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Ruler",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "TransportControls",
        ComponentCategory::Control,
        ComponentConformanceStatus::Experimental,
    );

    let ticks = TimelineRulerTickRequest::new(
        TimelineRange::seconds(0.0, 2.0),
        TimelineFrameRate::integer(24),
        TimelineZoom::new(120.0),
    )
    .ticks();

    assert!(!ticks.is_empty());
    assert!(ticks.iter().any(|tick| !tick.label.is_empty()));

    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(0.0, 1.0),
            "Clip",
        )],
        [],
        [],
    );
    let candidates = timeline_snap_candidates(TimelineSnapCandidateRequest::new(
        TimelineId::from_raw(1),
        TimelineRange::seconds(0.0, 1.0),
        TimelineFrameRate::integer(24),
        &descriptor,
    ));
    let selection = TimelineSelection::from_targets([TimelineSelectionTarget::Item(
        TimelineItemId::from_raw(10),
    )]);
    let transport = TransportControls::from_intents([
        TransportControlIntent::PlayPause,
        TransportControlIntent::Stop,
    ]);

    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::ItemBoundary)
    );
    assert!(selection.contains(TimelineSelectionTarget::Item(TimelineItemId::from_raw(10))));
    assert_eq!(transport.visible_controls().len(), 2);
}

#[test]
fn stage12_viewport_taxonomy_reports_experimental_status_backed_by_public_contracts() {
    assert_entry(
        "Viewport",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "ViewportTools",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "ViewportActionRouting",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "Ruler",
        ComponentCategory::Viewport,
        ComponentConformanceStatus::Experimental,
    );

    let mut pan_zoom = PanZoom::default();
    pan_zoom.set_zoom(1.5);
    pan_zoom.pan_by(Vec2::new(8.0, -4.0));
    let surface = ViewportSurface {
        texture: TextureId::from_raw(12),
        source_size: Size::new(640.0, 360.0),
        bounds: Rect::new(10.0, 20.0, 320.0, 180.0),
        pan_zoom,
    };
    let guides = viewport_guides(
        surface,
        &[ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(1),
            ViewportGuideOrientation::Vertical,
            ViewportGuidePlacement::Content(320.0),
        )],
    );
    let safe_areas = viewport_safe_areas(
        surface,
        &[ViewportSafeAreaDescriptor::new(
            ViewportSafeAreaId::from_raw(1),
            Rect::new(64.0, 36.0, 512.0, 288.0),
            ViewportSafeAreaSpace::Content,
        )],
    );
    let rulers = viewport_rulers(
        surface,
        &[
            ViewportRulerDescriptor::new(ViewportRulerId::from_raw(1), ViewportRulerEdge::Top)
                .with_max_ticks(12),
        ],
    );
    let hud = ViewportPanZoomHudDescriptor::new(WidgetId::from_key("taxonomy-viewport-hud"), "HUD")
        .resolve(surface);
    let viewport_id = WidgetId::from_key("taxonomy-viewport");
    let (action_requests, cursor_request) =
        viewport_action_and_cursor_contracts(surface, viewport_id);

    assert_eq!(guides.len(), 1);
    assert_eq!(safe_areas.len(), 1);
    assert_eq!(rulers.len(), 1);
    assert!(!rulers[0].ticks.is_empty());
    assert_eq!(hud.fit, ViewportFit::Zoom);
    assert!(hud.value_text().contains("content 640.000x360.000"));
    assert_eq!(action_requests.len(), 2);
    assert_eq!(
        action_requests[0].action_id,
        ActionId::new("viewport.fit.content")
    );
    assert_eq!(action_requests[1].checked, Some(true));
    assert_eq!(
        cursor_request.source,
        ViewportCursorRequestSource::HoveredOverlay
    );
    assert_eq!(cursor_request.cursor.shape, ViewportCursorShape::Crosshair);
    assert_eq!(
        guides[0]
            .semantics(WidgetId::from_key("taxonomy-viewport"))
            .role,
        SemanticRole::Custom("viewport-guide".to_owned())
    );
    assert_eq!(
        rulers[0]
            .semantics(WidgetId::from_key("taxonomy-viewport"))
            .role,
        SemanticRole::Custom("viewport-ruler".to_owned())
    );
}

#[test]
fn stage13_job_progress_taxonomy_reports_experimental_status_backed_by_public_contracts() {
    assert_entry(
        "StatusBar",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "ProgressIndicator",
        ComponentCategory::Display,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "JobList",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "DiagnosticStrip",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );
    assert_entry(
        "FeedbackStack",
        ComponentCategory::System,
        ComponentConformanceStatus::Experimental,
    );

    let jobs = JobList::from_rows([
        JobRow::new(JobRowId::from_raw(1), "Render", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.25)),
        JobRow::new(JobRowId::from_raw(2), "Export", JobPhase::Queued)
            .with_progress(JobProgress::determinate(0.75)),
    ]);

    assert_eq!(jobs.active_count(), 2);
    assert_close(
        jobs.active_status_progress()
            .expect("status progress for determinate jobs")
            .value,
        0.5,
    );
}
