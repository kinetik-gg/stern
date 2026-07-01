pub(crate) use kinetik_ui_core::{
    Brush, ClipId, Point, Primitive, Rect, RectPrimitive, SemanticRole, SemanticValue, WidgetId,
};
pub(crate) use kinetik_ui_widgets::{
    DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS, EdgeDescriptor, EdgeEndpointRole, EdgeId,
    EdgeResolutionError, GraphPoint, GraphRect, GraphVector, NodeDescriptor, NodeFrameDescriptor,
    NodeFrameId, NodeGraphAddNodeDescriptorId, NodeGraphAddNodeSearchEntry,
    NodeGraphAddNodeSearchHighlight, NodeGraphAddNodeSearchSelection, NodeGraphAnnotationField,
    NodeGraphBoxSelectionMode, NodeGraphBoxSelectionRequest, NodeGraphCanvasPanRequest,
    NodeGraphCollapseLinkMetadata, NodeGraphCollapseTarget, NodeGraphContextAction,
    NodeGraphContextActionKind, NodeGraphContextActionRequest,
    NodeGraphContextActionUnavailableReason, NodeGraphContextCanvasOperation,
    NodeGraphContextDisconnectTarget, NodeGraphContextOrganizationOperation,
    NodeGraphContextTarget, NodeGraphDescriptor, NodeGraphEdgeRoutePoint, NodeGraphEmissionError,
    NodeGraphFrameMove, NodeGraphGridStyle, NodeGraphHitTarget, NodeGraphHitTestConfig,
    NodeGraphHitTestError, NodeGraphLinkDraftCompletionError, NodeGraphLinkDraftEndpoint,
    NodeGraphLinkDraftEndpointError, NodeGraphLinkDraftOutcome, NodeGraphLinkDraftTarget,
    NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError, NodeGraphNodeStateAction,
    NodeGraphOrganizationTarget, NodeGraphPanZoom, NodeGraphPortState,
    NodeGraphSelectedNodeMoveRequest, NodeGraphSelection, NodeGraphSelectionIntent,
    NodeGraphSelectionOperation, NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphStyle,
    NodeGraphValidationError, NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId,
    PortCompatibilityError, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
    RerouteDescriptor, RerouteId, filter_node_graph_add_node_search_entries, node_graph_drag_delta,
    node_graph_snap_delta, node_graph_snap_point, node_graph_snap_rect, ports_are_compatible,
    validate_node_graph_descriptors, validate_port_compatibility,
};

pub(crate) fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.001,
        "expected {actual} to equal {expected}"
    );
}

pub(crate) fn assert_point_close(actual: Point, expected: Point) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

pub(crate) fn assert_graph_point_close(actual: GraphPoint, expected: GraphPoint) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

pub(crate) fn assert_graph_vector_close(actual: GraphVector, expected: GraphVector) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

pub(crate) fn assert_rect_close(actual: Rect, expected: Rect) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.width, expected.width);
    assert_close(actual.height, expected.height);
}

pub(crate) fn is_square_size(rect: Rect, size: f32) -> bool {
    (rect.width - size).abs() <= 0.001 && (rect.height - size).abs() <= 0.001
}

pub(crate) fn context_action(
    actions: &[NodeGraphContextAction],
    kind: NodeGraphContextActionKind,
) -> &NodeGraphContextAction {
    actions
        .iter()
        .find(|action| action.kind == kind)
        .expect("node graph context action")
}

pub(crate) fn assert_graph_rect_close(actual: GraphRect, expected: GraphRect) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.width, expected.width);
    assert_close(actual.height, expected.height);
}

pub(crate) fn viewport() -> NodeGraphViewport {
    NodeGraphViewport::new(
        Rect::new(25.0, 40.0, 320.0, 240.0),
        NodeGraphPanZoom::new(GraphVector::new(12.5, -7.25), 1.5),
    )
}

pub(crate) fn static_graph() -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(10);
    NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Source",
                GraphRect::new(10.0, 20.0, 100.0, 80.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Bypass", number)
                    .with_enabled(false),
            ]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Target",
                GraphRect::new(200.0, 40.0, 120.0, 60.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(3),
                PortDirection::Input,
                "In",
                number,
            )]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(50),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    }
}

pub(crate) fn static_viewport() -> NodeGraphViewport {
    NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 400.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(20.0, 10.0), 2.0),
    )
}

pub(crate) fn link_draft_graph() -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(10);
    let vector = PortTypeId::from_raw(11);
    NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Source",
                GraphRect::new(0.0, 0.0, 100.0, 120.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
                PortDescriptor::new(
                    PortId::from_raw(9),
                    PortDirection::Input,
                    "Disabled",
                    number,
                )
                .with_enabled(false),
            ]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Target",
                GraphRect::new(200.0, 0.0, 100.0, 120.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
                PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Vec", vector),
                PortDescriptor::new(PortId::from_raw(4), PortDirection::Output, "Mirror", number),
            ]),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    }
}

pub(crate) fn link_edit_graph() -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(10);
    let vector = PortTypeId::from_raw(11);
    NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Source",
                GraphRect::new(0.0, 0.0, 100.0, 160.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
                PortDescriptor::new(PortId::from_raw(5), PortDirection::Output, "Alt", number),
                PortDescriptor::new(PortId::from_raw(6), PortDirection::Output, "Vector", vector),
                PortDescriptor::new(
                    PortId::from_raw(7),
                    PortDirection::Output,
                    "Disabled",
                    number,
                )
                .with_enabled(false),
            ]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Target",
                GraphRect::new(200.0, 0.0, 100.0, 160.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
                PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Alt In", number),
                PortDescriptor::new(PortId::from_raw(4), PortDirection::Input, "Vec", vector),
                PortDescriptor::new(
                    PortId::from_raw(8),
                    PortDirection::Input,
                    "Disabled",
                    number,
                )
                .with_enabled(false),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(50),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    }
}

pub(crate) fn routed_edge_graph(reroute_position: GraphPoint) -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(10);
    let reroute = RerouteId::from_raw(10);
    NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Source",
                GraphRect::new(0.0, 0.0, 100.0, 100.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(1),
                PortDirection::Output,
                "Out",
                number,
            )]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Target",
                GraphRect::new(300.0, 0.0, 100.0, 100.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(2),
                PortDirection::Input,
                "In",
                number,
            )]),
        ],
        edges: vec![
            EdgeDescriptor::new(
                EdgeId::from_raw(50),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )
            .with_route_points(vec![
                NodeGraphEdgeRoutePoint::point(GraphPoint::new(120.0, 40.0)),
                NodeGraphEdgeRoutePoint::reroute(reroute),
                NodeGraphEdgeRoutePoint::point(GraphPoint::new(240.0, 60.0)),
            ]),
        ],
        reroutes: vec![RerouteDescriptor::new(reroute, "Bend A", reroute_position)],
        frames: Vec::new(),
        groups: Vec::new(),
    }
}
