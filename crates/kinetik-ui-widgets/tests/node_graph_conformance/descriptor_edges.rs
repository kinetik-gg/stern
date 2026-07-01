#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn node_graph_ids_round_trip_raw_bits() {
    assert_eq!(NodeId::from_raw(1).raw(), 1);
    assert_eq!(PortId::from_raw(2).raw(), 2);
    assert_eq!(EdgeId::from_raw(3).raw(), 3);
    assert_eq!(RerouteId::from_raw(7).raw(), 7);
    assert_eq!(NodeFrameId::from_raw(4).raw(), 4);
    assert_eq!(NodeGroupId::from_raw(5).raw(), 5);
    assert_eq!(PortTypeId::from_raw(6).raw(), 6);
}

#[test]
fn node_graph_descriptors_preserve_data_only_metadata() {
    let number = PortTypeId::from_raw(10);
    let vector = PortTypeId::from_raw(11);
    let output = PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Color", number);
    let input = PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Vector", vector)
        .with_enabled(false);
    let frame = NodeFrameDescriptor::new(
        NodeFrameId::from_raw(30),
        "Frame A",
        GraphRect::new(-10.0, -20.0, 300.0, 180.0),
    )
    .with_enabled(false);
    let group = NodeGroupDescriptor::new(
        NodeGroupId::from_raw(40),
        "Group A",
        GraphRect::new(0.0, 0.0, 200.0, 120.0),
    )
    .with_nodes(vec![NodeId::from_raw(20)])
    .with_enabled(false);
    let reroute = RerouteDescriptor::new(
        RerouteId::from_raw(45),
        "Bend A",
        GraphPoint::new(40.0, 50.0),
    )
    .with_position(GraphPoint::new(45.0, 55.0))
    .with_enabled(false);
    let node = NodeDescriptor::new(
        NodeId::from_raw(20),
        "Mix",
        GraphRect::new(5.0, 10.0, 140.0, 90.0),
    )
    .with_ports(vec![output.clone(), input.clone()])
    .with_frame(frame.id)
    .with_group(group.id)
    .with_enabled(false);
    let edge = EdgeDescriptor::new(
        EdgeId::from_raw(50),
        PortEndpoint::new(node.id, output.id),
        PortEndpoint::new(NodeId::from_raw(21), PortId::from_raw(3)),
    )
    .with_route_points(vec![NodeGraphEdgeRoutePoint::reroute(reroute.id)])
    .with_enabled(false);
    let graph = NodeGraphDescriptor {
        nodes: vec![node.clone()],
        edges: vec![edge],
        reroutes: vec![reroute.clone()],
        frames: vec![frame.clone()],
        groups: vec![group.clone()],
    };

    assert_eq!(node.title, "Mix");
    assert_eq!(node.rect, GraphRect::new(5.0, 10.0, 140.0, 90.0));
    assert_eq!(node.ports, vec![output.clone(), input.clone()]);
    assert_eq!(node.frame, Some(frame.id));
    assert_eq!(node.group, Some(group.id));
    assert!(!node.enabled);

    assert_eq!(input.direction, PortDirection::Input);
    assert_eq!(input.label, "Vector");
    assert_eq!(input.port_type, vector);
    assert!(!input.enabled);

    assert_eq!(graph.edges[0].id, EdgeId::from_raw(50));
    assert_eq!(graph.edges[0].from, PortEndpoint::new(node.id, output.id));
    assert_eq!(
        graph.edges[0].to,
        PortEndpoint::new(NodeId::from_raw(21), PortId::from_raw(3))
    );
    assert!(!graph.edges[0].enabled);
    assert_eq!(
        graph.edges[0].route_points,
        vec![NodeGraphEdgeRoutePoint::reroute(reroute.id)]
    );
    assert_eq!(graph.reroutes, vec![reroute]);
    assert_eq!(graph.reroutes[0].label, "Bend A");
    assert_eq!(graph.reroutes[0].position, GraphPoint::new(45.0, 55.0));
    assert!(!graph.reroutes[0].enabled);
    assert_eq!(graph.frames, vec![frame]);
    assert_eq!(graph.groups, vec![group]);
    assert_eq!(graph.validate(), Ok(()));
}

#[test]
fn descriptor_validation_reports_duplicate_node_ids_deterministically() {
    let id = NodeId::from_raw(1);
    let nodes = vec![
        NodeDescriptor::new(id, "First", GraphRect::ZERO),
        NodeDescriptor::new(NodeId::from_raw(2), "Second", GraphRect::ZERO),
        NodeDescriptor::new(id, "Duplicate", GraphRect::ZERO),
    ];

    assert_eq!(
        validate_node_graph_descriptors(&nodes),
        Err(NodeGraphValidationError::DuplicateNodeId { id })
    );
}

#[test]
fn descriptor_validation_reports_duplicate_port_ids_within_one_node() {
    let node_id = NodeId::from_raw(1);
    let port_id = PortId::from_raw(7);
    let port_type = PortTypeId::from_raw(10);
    let nodes = vec![
        NodeDescriptor::new(node_id, "Node", GraphRect::ZERO).with_ports(vec![
            PortDescriptor::new(port_id, PortDirection::Input, "A", port_type),
            PortDescriptor::new(port_id, PortDirection::Output, "B", port_type),
        ]),
        NodeDescriptor::new(NodeId::from_raw(2), "Other", GraphRect::ZERO).with_ports(vec![
            PortDescriptor::new(port_id, PortDirection::Input, "Scoped", port_type),
        ]),
    ];

    assert_eq!(
        validate_node_graph_descriptors(&nodes),
        Err(NodeGraphValidationError::DuplicatePortId {
            node: node_id,
            port: port_id,
        })
    );
}

#[test]
fn descriptor_validation_scopes_port_ids_by_node() {
    let port_id = PortId::from_raw(7);
    let port_type = PortTypeId::from_raw(10);
    let nodes = vec![
        NodeDescriptor::new(NodeId::from_raw(1), "A", GraphRect::ZERO).with_ports(vec![
            PortDescriptor::new(port_id, PortDirection::Input, "Input", port_type),
        ]),
        NodeDescriptor::new(NodeId::from_raw(2), "B", GraphRect::ZERO).with_ports(vec![
            PortDescriptor::new(port_id, PortDirection::Output, "Output", port_type),
        ]),
    ];

    assert_eq!(validate_node_graph_descriptors(&nodes), Ok(()));
}

#[test]
fn compatibility_is_directed_enabled_and_keyed_by_app_metadata() {
    let number = PortTypeId::from_raw(10);
    let vector = PortTypeId::from_raw(11);
    let output = PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number);
    let input = PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number);
    let other_input =
        PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Other", vector);
    let disabled_input =
        PortDescriptor::new(PortId::from_raw(4), PortDirection::Input, "Off", number)
            .with_enabled(false);

    assert!(ports_are_compatible(&output, &input));
    assert_eq!(validate_port_compatibility(&output, &input), Ok(()));

    assert_eq!(
        validate_port_compatibility(&input, &output),
        Err(PortCompatibilityError::DirectionMismatch {
            output: PortDirection::Input,
            input: PortDirection::Output,
        })
    );
    assert!(matches!(
        validate_port_compatibility(&output, &output),
        Err(PortCompatibilityError::DirectionMismatch { .. })
    ));
    assert!(matches!(
        validate_port_compatibility(&input, &input),
        Err(PortCompatibilityError::DirectionMismatch { .. })
    ));
    assert_eq!(
        validate_port_compatibility(&output, &other_input),
        Err(PortCompatibilityError::TypeMismatch {
            output: number,
            input: vector,
        })
    );
    assert_eq!(
        validate_port_compatibility(&output, &disabled_input),
        Err(PortCompatibilityError::DisabledPort {
            output_enabled: true,
            input_enabled: false,
        })
    );
    assert!(!ports_are_compatible(&output, &disabled_input));
}

#[test]
fn valid_edge_resolves_descriptors_and_anchor_points() {
    let number = PortTypeId::from_raw(10);
    let source = NodeDescriptor::new(
        NodeId::from_raw(1),
        "Source",
        GraphRect::new(10.0, 20.0, 100.0, 80.0),
    )
    .with_ports(vec![
        PortDescriptor::new(
            PortId::from_raw(9),
            PortDirection::Input,
            "Passthrough",
            number,
        ),
        PortDescriptor::new(
            PortId::from_raw(1),
            PortDirection::Output,
            "Primary",
            number,
        ),
        PortDescriptor::new(
            PortId::from_raw(2),
            PortDirection::Output,
            "Secondary",
            number,
        ),
    ]);
    let target = NodeDescriptor::new(
        NodeId::from_raw(2),
        "Target",
        GraphRect::new(200.0, 40.0, 120.0, 60.0),
    )
    .with_ports(vec![
        PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "A", number),
        PortDescriptor::new(PortId::from_raw(4), PortDirection::Input, "B", number),
        PortDescriptor::new(PortId::from_raw(8), PortDirection::Output, "Mirror", number),
    ]);
    let graph = NodeGraphDescriptor {
        nodes: vec![source, target],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(50),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(4)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let resolved = graph.resolve_edges().expect("edge should resolve");

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].edge.id, EdgeId::from_raw(50));
    assert_eq!(resolved[0].from.role, EdgeEndpointRole::Source);
    assert_eq!(resolved[0].from.node.id, NodeId::from_raw(1));
    assert_eq!(resolved[0].from.port.id, PortId::from_raw(1));
    assert_eq!(resolved[0].from.port.direction, PortDirection::Output);
    assert_graph_point_close(resolved[0].from.anchor, GraphPoint::new(110.0, 46.666_668));
    assert_eq!(resolved[0].to.role, EdgeEndpointRole::Target);
    assert_eq!(resolved[0].to.node.id, NodeId::from_raw(2));
    assert_eq!(resolved[0].to.port.id, PortId::from_raw(4));
    assert_eq!(resolved[0].to.port.direction, PortDirection::Input);
    assert_graph_point_close(resolved[0].to.anchor, GraphPoint::new(200.0, 80.0));
}

#[test]
fn edge_resolution_reports_missing_node_and_port_with_endpoint_context() {
    let number = PortTypeId::from_raw(10);
    let node = NodeDescriptor::new(NodeId::from_raw(1), "Node", GraphRect::ZERO).with_ports(vec![
        PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
    ]);
    let missing_node_graph = NodeGraphDescriptor {
        nodes: vec![node.clone()],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(20),
            PortEndpoint::new(NodeId::from_raw(9), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let missing_port_graph = NodeGraphDescriptor {
        nodes: vec![node],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(21),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(99)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    assert_eq!(
        missing_node_graph.resolve_edges(),
        Err(EdgeResolutionError::MissingNode {
            edge: EdgeId::from_raw(20),
            endpoint: EdgeEndpointRole::Source,
            node: NodeId::from_raw(9),
        })
    );
    assert_eq!(
        missing_port_graph.resolve_edges(),
        Err(EdgeResolutionError::MissingPort {
            edge: EdgeId::from_raw(21),
            endpoint: EdgeEndpointRole::Target,
            node: NodeId::from_raw(1),
            port: PortId::from_raw(99),
        })
    );
}

#[test]
fn edge_resolution_reports_wrong_direction_deterministically() {
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "Wrong", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(30),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    assert_eq!(
        graph.resolve_edges(),
        Err(EdgeResolutionError::WrongDirection {
            edge: EdgeId::from_raw(30),
            endpoint: EdgeEndpointRole::Source,
            node: NodeId::from_raw(1),
            port: PortId::from_raw(1),
            expected: PortDirection::Output,
            actual: PortDirection::Input,
        })
    );
}

#[test]
fn edge_resolution_reports_incompatible_and_disabled_ports() {
    let number = PortTypeId::from_raw(10);
    let vector = PortTypeId::from_raw(11);
    let incompatible_graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", vector),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(40),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let disabled_graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number)
                    .with_enabled(false),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(41),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    assert_eq!(
        incompatible_graph.resolve_edges(),
        Err(EdgeResolutionError::IncompatiblePortType {
            edge: EdgeId::from_raw(40),
            from: PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            to: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            output: number,
            input: vector,
        })
    );
    assert_eq!(
        disabled_graph.resolve_edges(),
        Err(EdgeResolutionError::DisabledPort {
            edge: EdgeId::from_raw(41),
            endpoint: EdgeEndpointRole::Target,
            node: NodeId::from_raw(2),
            port: PortId::from_raw(2),
        })
    );
}

#[test]
fn edge_resolution_reports_duplicate_edge_ids_deterministically() {
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
            ]),
        ],
        edges: vec![
            EdgeDescriptor::new(
                EdgeId::from_raw(50),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            ),
            EdgeDescriptor::new(
                EdgeId::from_raw(51),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            ),
            EdgeDescriptor::new(
                EdgeId::from_raw(50),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            ),
        ],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    assert_eq!(
        graph.resolve_edges(),
        Err(EdgeResolutionError::DuplicateEdgeId {
            edge: EdgeId::from_raw(50),
        })
    );
}

#[test]
fn edge_resolution_preserves_descriptor_order() {
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "A", number),
                PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "B", number),
            ]),
        ],
        edges: vec![
            EdgeDescriptor::new(
                EdgeId::from_raw(70),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
            ),
            EdgeDescriptor::new(
                EdgeId::from_raw(60),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            ),
        ],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let resolved = graph.resolve_edges().expect("edges should resolve");

    assert_eq!(
        resolved.iter().map(|edge| edge.edge.id).collect::<Vec<_>>(),
        vec![EdgeId::from_raw(70), EdgeId::from_raw(60)]
    );
}

#[test]
fn routed_edge_route_point_order_is_preserved() {
    let graph = routed_edge_graph(GraphPoint::new(180.0, 20.0));
    let resolved = graph.resolve_edges().expect("routed edge should resolve");

    assert_eq!(
        resolved[0]
            .route_points
            .iter()
            .map(|point| point.route_point)
            .collect::<Vec<_>>(),
        vec![
            NodeGraphEdgeRoutePoint::point(GraphPoint::new(120.0, 40.0)),
            NodeGraphEdgeRoutePoint::reroute(RerouteId::from_raw(10)),
            NodeGraphEdgeRoutePoint::point(GraphPoint::new(240.0, 60.0)),
        ]
    );
    assert_eq!(
        resolved[0]
            .route_points
            .iter()
            .map(|point| point.position)
            .collect::<Vec<_>>(),
        vec![
            GraphPoint::new(120.0, 40.0),
            GraphPoint::new(180.0, 20.0),
            GraphPoint::new(240.0, 60.0),
        ]
    );

    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
        .emit()
        .expect("routed static output");
    let edge_segments = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Line(line) => Some((line.from, line.to)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(edge_segments.len(), 4);
    assert_point_close(edge_segments[0].0, Point::new(320.0, 160.0));
    assert_point_close(edge_segments[0].1, Point::new(360.0, 140.0));
    assert_point_close(edge_segments[1].0, Point::new(360.0, 140.0));
    assert_point_close(edge_segments[1].1, Point::new(480.0, 100.0));
    assert_point_close(edge_segments[2].0, Point::new(480.0, 100.0));
    assert_point_close(edge_segments[2].1, Point::new(600.0, 180.0));
    assert_point_close(edge_segments[3].0, Point::new(600.0, 180.0));
    assert_point_close(edge_segments[3].1, Point::new(720.0, 160.0));
}

#[test]
fn stale_reroute_ids_report_deterministic_edge_resolution_errors() {
    let mut graph = routed_edge_graph(GraphPoint::new(180.0, 20.0));
    graph.reroutes.clear();

    assert_eq!(
        graph.resolve_edges(),
        Err(EdgeResolutionError::MissingReroute {
            edge: EdgeId::from_raw(50),
            reroute: RerouteId::from_raw(10),
        })
    );
}

#[test]
fn moving_reroute_changes_edge_route_metadata_deterministically() {
    let original = routed_edge_graph(GraphPoint::new(180.0, 20.0));
    let moved = routed_edge_graph(GraphPoint::new(210.0, 70.0));

    assert_eq!(original.edges[0].route_points, moved.edges[0].route_points);
    assert_eq!(original.reroutes[0].id, moved.reroutes[0].id);
    assert_eq!(original.reroutes[0].label, moved.reroutes[0].label);

    let original_route = original.resolve_edges().expect("original route");
    let moved_route = moved.resolve_edges().expect("moved route");

    assert_eq!(
        original_route[0].route_points[1].route_point,
        NodeGraphEdgeRoutePoint::reroute(RerouteId::from_raw(10))
    );
    assert_graph_point_close(
        original_route[0].route_points[1].position,
        GraphPoint::new(180.0, 20.0),
    );
    assert_graph_point_close(
        moved_route[0].route_points[1].position,
        GraphPoint::new(210.0, 70.0),
    );
}

#[test]
fn edge_resolution_allows_same_node_edges() {
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Node",
                GraphRect::new(-10.0, 5.0, 80.0, 50.0),
            )
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(80),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let resolved = graph
        .resolve_edges()
        .expect("same-node edge should resolve");

    assert_eq!(resolved[0].from.node.id, resolved[0].to.node.id);
    assert_graph_point_close(resolved[0].from.anchor, GraphPoint::new(70.0, 30.0));
    assert_graph_point_close(resolved[0].to.anchor, GraphPoint::new(-10.0, 30.0));
}
