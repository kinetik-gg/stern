#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn selection_replace_from_hit_normalizes_node_title_and_body_targets() {
    let node = NodeId::from_raw(10);
    let edge = EdgeId::from_raw(20);
    let selection = NodeGraphSelection::new()
        .replace(NodeGraphSelectionTarget::Edge(edge))
        .replace_from_hit(NodeGraphHitTarget::NodeTitle(node));

    assert_eq!(
        selection.selected(),
        vec![NodeGraphSelectionTarget::Node(node)]
    );
    assert_eq!(
        selection.active(),
        Some(NodeGraphSelectionTarget::Node(node))
    );

    let body_selection = selection.replace_from_hit(NodeGraphHitTarget::NodeBody(node));
    assert_eq!(
        body_selection.selected(),
        vec![NodeGraphSelectionTarget::Node(node)]
    );
}

#[test]
fn selection_toggles_node_edge_and_port_targets() {
    let node = NodeGraphSelectionTarget::Node(NodeId::from_raw(1));
    let edge = NodeGraphSelectionTarget::Edge(EdgeId::from_raw(2));
    let port =
        NodeGraphSelectionTarget::Port(PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(4)));

    let selection = NodeGraphSelection::new()
        .toggle(node)
        .toggle(edge)
        .toggle(port);

    assert!(selection.contains(node));
    assert!(selection.contains(edge));
    assert!(selection.contains(port));
    assert_eq!(selection.active(), Some(port));

    let selection = selection.toggle(edge).toggle(port).toggle(node);
    assert!(selection.is_empty());
    assert_eq!(selection.active(), Some(node));
}

#[test]
fn selection_extend_remove_and_operation_application_are_deterministic() {
    let node = NodeGraphSelectionTarget::Node(NodeId::from_raw(3));
    let edge = NodeGraphSelectionTarget::Edge(EdgeId::from_raw(1));
    let port =
        NodeGraphSelectionTarget::Port(PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(9)));
    let missing = NodeGraphSelectionTarget::Node(NodeId::from_raw(99));

    let selection = NodeGraphSelection::new()
        .apply(NodeGraphSelectionOperation::Extend(port))
        .apply(NodeGraphSelectionOperation::Extend(node))
        .apply(NodeGraphSelectionOperation::Extend(edge))
        .apply(NodeGraphSelectionOperation::Remove(missing));

    assert_eq!(selection.selected(), vec![node, edge, port]);
    assert_eq!(selection.active(), Some(edge));

    let selection = selection
        .apply(NodeGraphSelectionOperation::Remove(edge))
        .apply(NodeGraphSelectionOperation::Toggle(port))
        .apply(NodeGraphSelectionOperation::Replace(edge));

    assert_eq!(selection.selected(), vec![edge]);
    assert_eq!(selection.active(), Some(edge));
}

#[test]
fn selection_canvas_clear_behavior_is_explicit_and_deterministic() {
    let node = NodeGraphSelectionTarget::Node(NodeId::from_raw(1));
    let frame_hit = NodeGraphHitTarget::Frame(NodeFrameId::from_raw(7));
    let group_hit = NodeGraphHitTarget::Group(NodeGroupId::from_raw(8));
    let selection = NodeGraphSelection::new().replace(node);

    assert_eq!(selection.replace_from_hit(frame_hit), selection);
    assert_eq!(selection.replace_from_hit(group_hit), selection);
    assert!(
        selection
            .replace_from_hit(NodeGraphHitTarget::Canvas)
            .selected()
            .is_empty()
    );
    assert_eq!(
        selection.apply(NodeGraphSelectionOperation::Clear),
        NodeGraphSelection::new()
    );
}

#[test]
fn selection_identity_uses_graph_ids_not_viewport_coordinates() {
    let graph = static_graph();
    let first_viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 300.0),
        NodeGraphPanZoom::default(),
    );
    let second_viewport = NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 400.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(20.0, 10.0), 2.0),
    );
    let first_hit = graph
        .hit_test(first_viewport, Point::new(20.0, 30.0))
        .expect("first hit");
    let second_hit = graph
        .hit_test(second_viewport, Point::new(140.0, 110.0))
        .expect("second hit");

    assert_eq!(
        first_hit,
        NodeGraphHitTarget::NodeTitle(NodeId::from_raw(1))
    );
    assert_eq!(second_hit, first_hit);
    assert_eq!(
        NodeGraphSelectionTarget::from_hit_target(first_hit),
        Some(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
    );
}

#[test]
fn box_selection_converts_screen_rect_to_graph_across_pan_zoom() {
    let graph = static_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 400.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(20.0, -10.0), 2.0),
    );

    let request = NodeGraphBoxSelectionRequest::new(
        viewport,
        Rect::new(340.0, 240.0, -205.0, -165.0),
        NodeGraphBoxSelectionMode::Contains,
        NodeGraphSelectionIntent::Replace,
    );
    let selection = request.select(&graph);

    assert_rect_close(request.screen_rect, Rect::new(135.0, 75.0, 205.0, 165.0));
    assert_graph_rect_close(request.graph_rect, GraphRect::new(7.5, 17.5, 102.5, 82.5));
    assert_eq!(
        selection.targets,
        vec![NodeGraphSelectionTarget::Node(NodeId::from_raw(1))]
    );
    assert_eq!(
        selection.operations,
        vec![NodeGraphSelectionOperation::Replace(
            NodeGraphSelectionTarget::Node(NodeId::from_raw(1))
        )]
    );
}

#[test]
fn box_selection_contains_and_intersects_modes_are_distinct() {
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Inside",
                GraphRect::new(0.0, 0.0, 100.0, 100.0),
            ),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Crossing",
                GraphRect::new(90.0, 0.0, 50.0, 50.0),
            ),
            NodeDescriptor::new(
                NodeId::from_raw(3),
                "Outside",
                GraphRect::new(120.0, 0.0, 20.0, 20.0),
            ),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let contains = NodeGraphBoxSelectionRequest::from_graph_rect(
        GraphRect::new(0.0, 0.0, 100.0, 100.0),
        NodeGraphBoxSelectionMode::Contains,
        NodeGraphSelectionIntent::Add,
    )
    .select(&graph);
    let intersects = NodeGraphBoxSelectionRequest::from_graph_rect(
        GraphRect::new(0.0, 0.0, 100.0, 100.0),
        NodeGraphBoxSelectionMode::Intersects,
        NodeGraphSelectionIntent::Add,
    )
    .select(&graph);

    assert_eq!(
        contains.targets,
        vec![NodeGraphSelectionTarget::Node(NodeId::from_raw(1))]
    );
    assert_eq!(
        intersects.targets,
        vec![
            NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
            NodeGraphSelectionTarget::Node(NodeId::from_raw(2)),
        ]
    );
}

#[test]
fn box_selection_additive_and_subtractive_metadata_is_deterministic() {
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(3),
                "Third",
                GraphRect::new(0.0, 0.0, 10.0, 10.0),
            ),
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "First",
                GraphRect::new(5.0, 5.0, 10.0, 10.0),
            ),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Disabled",
                GraphRect::new(0.0, 0.0, 10.0, 10.0),
            )
            .with_enabled(false),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let add = NodeGraphBoxSelectionRequest::from_graph_rect(
        GraphRect::new(0.0, 0.0, 20.0, 20.0),
        NodeGraphBoxSelectionMode::Intersects,
        NodeGraphSelectionIntent::Add,
    )
    .select(&graph);
    let subtract = NodeGraphBoxSelectionRequest::from_graph_rect(
        GraphRect::new(0.0, 0.0, 20.0, 20.0),
        NodeGraphBoxSelectionMode::Intersects,
        NodeGraphSelectionIntent::Subtract,
    )
    .select(&graph);

    let targets = vec![
        NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
        NodeGraphSelectionTarget::Node(NodeId::from_raw(3)),
    ];
    assert_eq!(add.targets, targets);
    assert_eq!(
        add.operations,
        vec![
            NodeGraphSelectionOperation::Extend(NodeGraphSelectionTarget::Node(NodeId::from_raw(
                1
            ))),
            NodeGraphSelectionOperation::Extend(NodeGraphSelectionTarget::Node(NodeId::from_raw(
                3
            ))),
        ]
    );
    assert_eq!(
        subtract.operations,
        vec![
            NodeGraphSelectionOperation::Remove(NodeGraphSelectionTarget::Node(NodeId::from_raw(
                1
            ))),
            NodeGraphSelectionOperation::Remove(NodeGraphSelectionTarget::Node(NodeId::from_raw(
                3
            ))),
        ]
    );
}

#[test]
fn box_selection_empty_and_invalid_inputs_are_deterministic() {
    let request = NodeGraphBoxSelectionRequest::new(
        NodeGraphViewport::new(
            Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN),
            NodeGraphPanZoom::new(GraphVector::new(f32::INFINITY, f32::NAN), 0.0),
        ),
        Rect::new(f32::NAN, f32::INFINITY, -30.0, f32::NEG_INFINITY),
        NodeGraphBoxSelectionMode::Intersects,
        NodeGraphSelectionIntent::Replace,
    );
    let selection = request.select(&NodeGraphDescriptor::new());

    assert!(request.is_empty());
    assert_rect_close(request.screen_rect, Rect::ZERO);
    assert_graph_rect_close(request.graph_rect, GraphRect::ZERO);
    assert!(selection.targets.is_empty());
    assert_eq!(
        selection.operations,
        vec![NodeGraphSelectionOperation::Clear]
    );
}

#[test]
fn box_selection_finite_origin_with_non_finite_size_is_noop() {
    let graph = NodeGraphDescriptor {
        nodes: vec![NodeDescriptor::new(
            NodeId::from_raw(1),
            "Would Have Matched",
            GraphRect::new(20.0, 15.0, 5.0, 5.0),
        )],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let request = NodeGraphBoxSelectionRequest::new(
        NodeGraphViewport::new(Rect::ZERO, NodeGraphPanZoom::default()),
        Rect::new(100.0, 10.0, f32::INFINITY, 20.0),
        NodeGraphBoxSelectionMode::Intersects,
        NodeGraphSelectionIntent::Add,
    );
    let selection = request.select(&graph);

    assert!(request.is_empty());
    assert_rect_close(request.screen_rect, Rect::ZERO);
    assert_graph_rect_close(request.graph_rect, GraphRect::ZERO);
    assert!(selection.targets.is_empty());
    assert!(selection.is_noop());
}

#[test]
fn box_selection_graph_rect_non_finite_components_are_noop() {
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Would Have Matched Sanitized X",
                GraphRect::new(0.0, 10.0, 20.0, 20.0),
            ),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Would Have Matched Sanitized Y",
                GraphRect::new(10.0, 0.0, 20.0, 20.0),
            ),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    for graph_rect in [
        GraphRect::new(f32::INFINITY, 10.0, 20.0, 20.0),
        GraphRect::new(10.0, f32::NAN, 20.0, 20.0),
        GraphRect::new(10.0, 10.0, f32::INFINITY, 20.0),
        GraphRect::new(10.0, 10.0, 20.0, f32::NEG_INFINITY),
    ] {
        let request = NodeGraphBoxSelectionRequest::from_graph_rect(
            graph_rect,
            NodeGraphBoxSelectionMode::Intersects,
            NodeGraphSelectionIntent::Add,
        );
        let selection = request.select(&graph);

        assert!(request.is_empty());
        assert_graph_rect_close(request.graph_rect, GraphRect::ZERO);
        assert!(selection.targets.is_empty());
        assert!(selection.is_noop());
    }
}
