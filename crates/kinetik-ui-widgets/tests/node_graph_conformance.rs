//! Node graph identity and coordinate-space conformance tests.

mod node_graph_conformance {
    use kinetik_ui_core::{
        Brush, ClipId, Point, Primitive, Rect, RectPrimitive, SemanticRole, SemanticValue, WidgetId,
    };
    use kinetik_ui_widgets::{
        EdgeDescriptor, EdgeEndpointRole, EdgeId, EdgeResolutionError, GraphPoint, GraphRect,
        GraphVector, NodeDescriptor, NodeFrameDescriptor, NodeFrameId, NodeGraphBoxSelectionMode,
        NodeGraphBoxSelectionRequest, NodeGraphCanvasPanRequest, NodeGraphDescriptor,
        NodeGraphEmissionError, NodeGraphGridStyle, NodeGraphHitTarget, NodeGraphHitTestConfig,
        NodeGraphHitTestError, NodeGraphPanZoom, NodeGraphPortState,
        NodeGraphSelectedNodeMoveRequest, NodeGraphSelection, NodeGraphSelectionIntent,
        NodeGraphSelectionOperation, NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphStyle,
        NodeGraphValidationError, NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId,
        PortCompatibilityError, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
        node_graph_drag_delta, node_graph_snap_delta, node_graph_snap_point, node_graph_snap_rect,
        ports_are_compatible, validate_node_graph_descriptors, validate_port_compatibility,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_point_close(actual: Point, expected: Point) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_graph_point_close(actual: GraphPoint, expected: GraphPoint) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_graph_vector_close(actual: GraphVector, expected: GraphVector) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_rect_close(actual: Rect, expected: Rect) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
        assert_close(actual.width, expected.width);
        assert_close(actual.height, expected.height);
    }

    fn is_square_size(rect: Rect, size: f32) -> bool {
        (rect.width - size).abs() <= 0.001 && (rect.height - size).abs() <= 0.001
    }

    fn assert_graph_rect_close(actual: GraphRect, expected: GraphRect) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
        assert_close(actual.width, expected.width);
        assert_close(actual.height, expected.height);
    }

    fn viewport() -> NodeGraphViewport {
        NodeGraphViewport::new(
            Rect::new(25.0, 40.0, 320.0, 240.0),
            NodeGraphPanZoom::new(GraphVector::new(12.5, -7.25), 1.5),
        )
    }

    fn static_graph() -> NodeGraphDescriptor {
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
                    PortDescriptor::new(
                        PortId::from_raw(2),
                        PortDirection::Input,
                        "Bypass",
                        number,
                    )
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
            frames: Vec::new(),
            groups: Vec::new(),
        }
    }

    fn static_viewport() -> NodeGraphViewport {
        NodeGraphViewport::new(
            Rect::new(100.0, 50.0, 400.0, 300.0),
            NodeGraphPanZoom::new(GraphVector::new(20.0, 10.0), 2.0),
        )
    }

    #[test]
    fn node_graph_ids_round_trip_raw_bits() {
        assert_eq!(NodeId::from_raw(1).raw(), 1);
        assert_eq!(PortId::from_raw(2).raw(), 2);
        assert_eq!(EdgeId::from_raw(3).raw(), 3);
        assert_eq!(NodeFrameId::from_raw(4).raw(), 4);
        assert_eq!(NodeGroupId::from_raw(5).raw(), 5);
        assert_eq!(PortTypeId::from_raw(6).raw(), 6);
    }

    #[test]
    fn node_graph_descriptors_preserve_data_only_metadata() {
        let number = PortTypeId::from_raw(10);
        let vector = PortTypeId::from_raw(11);
        let output =
            PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Color", number);
        let input =
            PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Vector", vector)
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
        .with_enabled(false);
        let graph = NodeGraphDescriptor {
            nodes: vec![node.clone()],
            edges: vec![edge],
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
        let node =
            NodeDescriptor::new(NodeId::from_raw(1), "Node", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]);
        let missing_node_graph = NodeGraphDescriptor {
            nodes: vec![node.clone()],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(20),
                PortEndpoint::new(NodeId::from_raw(9), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            )],
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
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Input,
                        "Wrong",
                        number,
                    )],
                ),
                NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(2),
                        PortDirection::Input,
                        "In",
                        number,
                    )],
                ),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(30),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )],
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
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "Out",
                        number,
                    )],
                ),
                NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(2),
                        PortDirection::Input,
                        "In",
                        vector,
                    )],
                ),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(40),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )],
            frames: Vec::new(),
            groups: Vec::new(),
        };
        let disabled_graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "Out",
                        number,
                    )],
                ),
                NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(
                    vec![
                        PortDescriptor::new(
                            PortId::from_raw(2),
                            PortDirection::Input,
                            "In",
                            number,
                        )
                        .with_enabled(false),
                    ],
                ),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(41),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )],
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
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "Out",
                        number,
                    )],
                ),
                NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(2),
                        PortDirection::Input,
                        "In",
                        number,
                    )],
                ),
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
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "Out",
                        number,
                    )],
                ),
                NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(
                    vec![
                        PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "A", number),
                        PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "B", number),
                    ],
                ),
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

    #[test]
    fn pan_zoom_sanitizes_invalid_zoom_and_pan() {
        let pan_zoom =
            NodeGraphPanZoom::new(GraphVector::new(f32::NAN, f32::INFINITY), -2.0).sanitized();

        assert_close(pan_zoom.pan.x, 0.0);
        assert_close(pan_zoom.pan.y, 0.0);
        assert_close(pan_zoom.zoom, 1.0);

        for invalid_zoom in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let pan_zoom =
                NodeGraphPanZoom::new(GraphVector::new(5.0, -3.0), invalid_zoom).sanitized();

            assert_close(pan_zoom.pan.x, 5.0);
            assert_close(pan_zoom.pan.y, -3.0);
            assert_close(pan_zoom.zoom, 1.0);
        }

        let mut pan_zoom = NodeGraphPanZoom::default();
        pan_zoom.set_zoom(0.0);
        pan_zoom.pan_by(GraphVector::new(5.0, f32::NEG_INFINITY));

        assert_close(pan_zoom.zoom, 1.0);
        assert_close(pan_zoom.pan.x, 5.0);
        assert_close(pan_zoom.pan.y, 0.0);
    }

    #[test]
    fn graph_and_screen_points_round_trip_with_fractional_pan_zoom() {
        let viewport = viewport();
        let graph = GraphPoint::new(100.25, -20.5);
        let screen = viewport.graph_to_screen(graph);
        let round_trip = viewport.screen_to_graph(screen);

        assert_point_close(screen, Point::new(187.875, 1.999_999));
        assert_graph_point_close(round_trip, graph);
    }

    #[test]
    fn conversions_account_for_non_origin_viewport_bounds() {
        let viewport = NodeGraphViewport::new(
            Rect::new(100.0, 200.0, 400.0, 300.0),
            NodeGraphPanZoom::new(GraphVector::new(-25.0, 15.0), 2.0),
        );

        let screen = viewport.graph_to_screen(GraphPoint::new(10.0, 20.0));
        let graph = viewport.screen_to_graph(Point::new(75.0, 215.0));

        assert_point_close(screen, Point::new(95.0, 255.0));
        assert_graph_point_close(graph, GraphPoint::new(0.0, 0.0));
    }

    #[test]
    fn graph_and_screen_rects_round_trip() {
        let viewport = viewport();
        let graph = GraphRect::new(10.0, 20.0, 120.0, 80.0);
        let screen = viewport.graph_rect_to_screen(graph);
        let round_trip = viewport.screen_rect_to_graph(screen);

        assert_rect_close(screen, Rect::new(52.5, 62.75, 180.0, 120.0));
        assert_graph_rect_close(round_trip, graph);
    }

    #[test]
    fn graph_coordinates_and_rect_sizes_sanitize_deterministically() {
        let viewport = NodeGraphViewport::new(
            Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN),
            NodeGraphPanZoom::new(GraphVector::new(f32::INFINITY, f32::NAN), 0.0),
        );

        assert_rect_close(viewport.effective_bounds(), Rect::new(0.0, 0.0, 0.0, 0.0));

        let screen = viewport.graph_to_screen(GraphPoint::new(f32::NAN, f32::NEG_INFINITY));
        let graph = viewport.screen_to_graph(Point::new(f32::NAN, f32::INFINITY));
        let screen_rect = viewport.graph_rect_to_screen(GraphRect::new(
            f32::INFINITY,
            f32::NAN,
            -20.0,
            f32::NEG_INFINITY,
        ));
        let graph_rect = viewport.screen_rect_to_graph(Rect::new(
            f32::NAN,
            f32::INFINITY,
            -30.0,
            f32::NEG_INFINITY,
        ));

        assert_point_close(screen, Point::new(0.0, 0.0));
        assert_graph_point_close(graph, GraphPoint::new(0.0, 0.0));
        assert_rect_close(screen_rect, Rect::new(0.0, 0.0, 0.0, 0.0));
        assert_graph_rect_close(graph_rect, GraphRect::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn hit_testing_prioritizes_ports_over_node_title_and_body() {
        let number = PortTypeId::from_raw(10);
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Node",
                    GraphRect::new(0.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "In", number),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Output, "Out", number),
                ]),
            ],
            edges: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 300.0, 200.0),
            NodeGraphPanZoom::default(),
        );
        let config = NodeGraphHitTestConfig::new().with_port_size(24.0);

        assert_eq!(
            graph.hit_test_with_config(viewport, Point::new(0.0, 40.0), config),
            Ok(NodeGraphHitTarget::Port(PortEndpoint::new(
                NodeId::from_raw(1),
                PortId::from_raw(1)
            )))
        );
    }

    #[test]
    fn hit_testing_node_title_and_body_transform_through_viewport() {
        let graph = NodeGraphDescriptor {
            nodes: vec![NodeDescriptor::new(
                NodeId::from_raw(10),
                "Transformed",
                GraphRect::new(10.0, 20.0, 100.0, 80.0),
            )],
            edges: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(100.0, 50.0, 500.0, 400.0),
            NodeGraphPanZoom::new(GraphVector::new(20.0, -10.0), 2.0),
        );

        assert_eq!(
            graph.hit_test(viewport, Point::new(160.0, 90.0)),
            Ok(NodeGraphHitTarget::NodeTitle(NodeId::from_raw(10)))
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(160.0, 160.0)),
            Ok(NodeGraphHitTarget::NodeBody(NodeId::from_raw(10)))
        );
    }

    #[test]
    fn hit_testing_edges_uses_resolved_anchors_and_tolerance() {
        let number = PortTypeId::from_raw(10);
        let graph = NodeGraphDescriptor {
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
                    GraphRect::new(200.0, 0.0, 100.0, 100.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(2),
                    PortDirection::Input,
                    "In",
                    number,
                )]),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(30),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )],
            frames: Vec::new(),
            groups: Vec::new(),
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 400.0, 200.0),
            NodeGraphPanZoom::default(),
        );
        let config = NodeGraphHitTestConfig::new().with_edge_tolerance(5.0);

        assert_eq!(
            graph.hit_test_with_config(viewport, Point::new(150.0, 53.0), config),
            Ok(NodeGraphHitTarget::Edge(EdgeId::from_raw(30)))
        );
        assert_eq!(
            graph.hit_test_with_config(viewport, Point::new(150.0, 56.0), config),
            Ok(NodeGraphHitTarget::Canvas)
        );
    }

    #[test]
    fn hit_testing_frames_groups_and_canvas_are_deterministic() {
        let graph = NodeGraphDescriptor {
            nodes: Vec::new(),
            edges: Vec::new(),
            frames: vec![
                NodeFrameDescriptor::new(
                    NodeFrameId::from_raw(1),
                    "Back",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
                NodeFrameDescriptor::new(
                    NodeFrameId::from_raw(2),
                    "Front",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
            ],
            groups: vec![NodeGroupDescriptor::new(
                NodeGroupId::from_raw(3),
                "Group",
                GraphRect::new(150.0, 0.0, 100.0, 100.0),
            )],
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 300.0, 200.0),
            NodeGraphPanZoom::default(),
        );

        assert_eq!(
            graph.hit_test(viewport, Point::new(50.0, 50.0)),
            Ok(NodeGraphHitTarget::Frame(NodeFrameId::from_raw(2)))
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(175.0, 50.0)),
            Ok(NodeGraphHitTarget::Group(NodeGroupId::from_raw(3)))
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(275.0, 50.0)),
            Ok(NodeGraphHitTarget::Canvas)
        );
    }

    #[test]
    fn hit_testing_skips_disabled_targets_without_skipping_enabled_fallbacks() {
        let number = PortTypeId::from_raw(10);
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Node",
                    GraphRect::new(0.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "In", number)
                        .with_enabled(false),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Output, "Out", number),
                ]),
                NodeDescriptor::new(
                    NodeId::from_raw(2),
                    "Other",
                    GraphRect::new(200.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(3),
                    PortDirection::Input,
                    "In",
                    number,
                )]),
            ],
            edges: vec![
                EdgeDescriptor::new(
                    EdgeId::from_raw(20),
                    PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(2)),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
                )
                .with_enabled(false),
            ],
            frames: vec![
                NodeFrameDescriptor::new(
                    NodeFrameId::from_raw(30),
                    "Disabled frame",
                    GraphRect::new(120.0, 0.0, 50.0, 50.0),
                )
                .with_enabled(false),
            ],
            groups: vec![
                NodeGroupDescriptor::new(
                    NodeGroupId::from_raw(40),
                    "Disabled group",
                    GraphRect::new(180.0, 0.0, 50.0, 50.0),
                )
                .with_enabled(false),
            ],
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 300.0, 200.0),
            NodeGraphPanZoom::default(),
        );
        let config = NodeGraphHitTestConfig::new().with_port_size(24.0);

        assert_eq!(
            graph.hit_test_with_config(viewport, Point::new(0.0, 40.0), config),
            Ok(NodeGraphHitTarget::NodeBody(NodeId::from_raw(1)))
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(150.0, 40.0)),
            Ok(NodeGraphHitTarget::Canvas)
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(125.0, 5.0)),
            Ok(NodeGraphHitTarget::Canvas)
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(185.0, 5.0)),
            Ok(NodeGraphHitTarget::Canvas)
        );

        let disabled_node_graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(50),
                    "Disabled node",
                    GraphRect::new(0.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(51),
                    PortDirection::Input,
                    "In",
                    number,
                )])
                .with_enabled(false),
            ],
            edges: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        };

        assert_eq!(
            disabled_node_graph.hit_test_with_config(viewport, Point::new(0.0, 40.0), config),
            Ok(NodeGraphHitTarget::Canvas)
        );
    }

    #[test]
    fn hit_testing_tie_breaks_use_topmost_descriptor_order() {
        let number = PortTypeId::from_raw(10);
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Back",
                    GraphRect::new(0.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(1),
                    PortDirection::Input,
                    "In",
                    number,
                )]),
                NodeDescriptor::new(
                    NodeId::from_raw(2),
                    "Front",
                    GraphRect::new(0.0, 0.0, 100.0, 80.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(2),
                    PortDirection::Input,
                    "In",
                    number,
                )]),
            ],
            edges: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        };
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 300.0, 200.0),
            NodeGraphPanZoom::default(),
        );
        let config = NodeGraphHitTestConfig::new().with_port_size(24.0);

        assert_eq!(
            graph.hit_test_with_config(viewport, Point::new(0.0, 40.0), config),
            Ok(NodeGraphHitTarget::Port(PortEndpoint::new(
                NodeId::from_raw(2),
                PortId::from_raw(2)
            )))
        );
        assert_eq!(
            graph.hit_test(viewport, Point::new(50.0, 10.0)),
            Ok(NodeGraphHitTarget::NodeTitle(NodeId::from_raw(2)))
        );
    }

    #[test]
    fn hit_testing_invalid_descriptors_return_structured_errors() {
        let duplicate = NodeId::from_raw(1);
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(duplicate, "First", GraphRect::ZERO),
                NodeDescriptor::new(duplicate, "Second", GraphRect::ZERO),
            ],
            edges: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        };

        assert_eq!(
            graph.hit_test(
                NodeGraphViewport::new(
                    Rect::new(0.0, 0.0, 100.0, 100.0),
                    NodeGraphPanZoom::default()
                ),
                Point::new(1.0, 1.0)
            ),
            Err(NodeGraphHitTestError::Validation(
                NodeGraphValidationError::DuplicateNodeId { id: duplicate }
            ))
        );
    }

    #[test]
    fn hit_testing_rejects_duplicate_frame_ids_before_returning_frame_targets() {
        let duplicate = NodeFrameId::from_raw(7);
        let graph = NodeGraphDescriptor {
            nodes: Vec::new(),
            edges: Vec::new(),
            frames: vec![
                NodeFrameDescriptor::new(
                    duplicate,
                    "First",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
                NodeFrameDescriptor::new(
                    duplicate,
                    "Second",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
            ],
            groups: Vec::new(),
        };

        assert_eq!(
            graph.hit_test(
                NodeGraphViewport::new(
                    Rect::new(0.0, 0.0, 100.0, 100.0),
                    NodeGraphPanZoom::default()
                ),
                Point::new(10.0, 10.0)
            ),
            Err(NodeGraphHitTestError::Validation(
                NodeGraphValidationError::DuplicateFrameId { id: duplicate }
            ))
        );
    }

    #[test]
    fn hit_testing_rejects_duplicate_group_ids_before_returning_group_targets() {
        let duplicate = NodeGroupId::from_raw(8);
        let graph = NodeGraphDescriptor {
            nodes: Vec::new(),
            edges: Vec::new(),
            frames: Vec::new(),
            groups: vec![
                NodeGroupDescriptor::new(
                    duplicate,
                    "First",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
                NodeGroupDescriptor::new(
                    duplicate,
                    "Second",
                    GraphRect::new(0.0, 0.0, 100.0, 100.0),
                ),
            ],
        };

        assert_eq!(
            graph.hit_test(
                NodeGraphViewport::new(
                    Rect::new(0.0, 0.0, 100.0, 100.0),
                    NodeGraphPanZoom::default()
                ),
                Point::new(10.0, 10.0)
            ),
            Err(NodeGraphHitTestError::Validation(
                NodeGraphValidationError::DuplicateGroupId { id: duplicate }
            ))
        );
    }

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
        let port = NodeGraphSelectionTarget::Port(PortEndpoint::new(
            NodeId::from_raw(3),
            PortId::from_raw(4),
        ));

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
        let port = NodeGraphSelectionTarget::Port(PortEndpoint::new(
            NodeId::from_raw(2),
            PortId::from_raw(9),
        ));
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
                NodeGraphSelectionOperation::Extend(NodeGraphSelectionTarget::Node(
                    NodeId::from_raw(1)
                )),
                NodeGraphSelectionOperation::Extend(NodeGraphSelectionTarget::Node(
                    NodeId::from_raw(3)
                )),
            ]
        );
        assert_eq!(
            subtract.operations,
            vec![
                NodeGraphSelectionOperation::Remove(NodeGraphSelectionTarget::Node(
                    NodeId::from_raw(1)
                )),
                NodeGraphSelectionOperation::Remove(NodeGraphSelectionTarget::Node(
                    NodeId::from_raw(3)
                )),
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

    #[test]
    fn drag_delta_accounts_for_viewport_zoom_and_sanitizes_invalid_input() {
        let viewport = NodeGraphViewport::new(
            Rect::new(50.0, 30.0, 300.0, 200.0),
            NodeGraphPanZoom::new(GraphVector::new(8.0, -4.0), 2.5),
        );

        assert_graph_vector_close(
            node_graph_drag_delta(viewport, GraphVector::new(25.0, -10.0)),
            GraphVector::new(10.0, -4.0),
        );
        assert_graph_vector_close(
            viewport.screen_delta_to_graph(GraphVector::new(5.0, 7.5)),
            GraphVector::new(2.0, 3.0),
        );

        let invalid = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            NodeGraphPanZoom::new(GraphVector::new(f32::NAN, 10.0), f32::NAN),
        );
        assert_eq!(
            node_graph_drag_delta(invalid, GraphVector::new(f32::INFINITY, -12.0)),
            GraphVector::new(0.0, -12.0)
        );
    }

    #[test]
    fn snap_helpers_handle_negative_coordinates_and_fractional_zoom() {
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 300.0, 200.0),
            NodeGraphPanZoom::new(GraphVector::ZERO, 2.5),
        );
        let graph_delta = node_graph_drag_delta(viewport, GraphVector::new(-31.25, 18.75));

        assert_graph_point_close(
            node_graph_snap_point(GraphPoint::new(-12.5, 12.49), 5.0),
            GraphPoint::new(-15.0, 10.0),
        );
        assert_graph_rect_close(
            node_graph_snap_rect(GraphRect::new(-12.5, 7.5, 12.4, 17.6), 5.0),
            GraphRect::new(-15.0, 10.0, 10.0, 20.0),
        );
        assert_graph_vector_close(graph_delta, GraphVector::new(-12.5, 7.5));
        assert_graph_vector_close(
            node_graph_snap_delta(graph_delta, 5.0),
            GraphVector::new(-15.0, 10.0),
        );
    }

    #[test]
    fn snap_helpers_sanitize_invalid_grid_sizes_without_snapping() {
        let point = GraphPoint::new(f32::NAN, f32::NEG_INFINITY);
        let rect = GraphRect::new(f32::INFINITY, -12.0, -5.0, 9.0);
        let delta = GraphVector::new(7.5, f32::NAN);

        for invalid_grid in [0.0, -4.0, f32::NAN, f32::INFINITY] {
            assert_graph_point_close(node_graph_snap_point(point, invalid_grid), GraphPoint::ZERO);
            assert_graph_rect_close(
                node_graph_snap_rect(rect, invalid_grid),
                GraphRect::new(0.0, -12.0, 0.0, 9.0),
            );
            assert_graph_vector_close(
                node_graph_snap_delta(delta, invalid_grid),
                GraphVector::new(7.5, 0.0),
            );
        }
    }

    #[test]
    fn selected_node_move_request_moves_selected_nodes_together() {
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 400.0, 300.0),
            NodeGraphPanZoom::new(GraphVector::ZERO, 2.0),
        );
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(3)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(9)),
            NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
            NodeGraphSelectionTarget::Port(PortEndpoint::new(
                NodeId::from_raw(2),
                PortId::from_raw(4),
            )),
        ]);

        let request = NodeGraphSelectedNodeMoveRequest::new(
            viewport,
            selection.clone(),
            GraphVector::new(40.0, -20.0),
        );

        assert_eq!(request.selection, selection);
        assert_eq!(request.screen_delta, GraphVector::new(40.0, -20.0));
        assert_graph_vector_close(request.graph_delta, GraphVector::new(20.0, -10.0));
        assert_eq!(
            request.nodes,
            vec![
                kinetik_ui_widgets::NodeGraphNodeMove {
                    node: NodeId::from_raw(1),
                    delta: GraphVector::new(20.0, -10.0),
                },
                kinetik_ui_widgets::NodeGraphNodeMove {
                    node: NodeId::from_raw(3),
                    delta: GraphVector::new(20.0, -10.0),
                },
            ]
        );
        assert!(!request.is_noop());
    }

    #[test]
    fn canvas_pan_request_preserves_selection_metadata() {
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(2)),
        ]);

        let request =
            NodeGraphCanvasPanRequest::new(selection.clone(), GraphVector::new(12.0, -8.0));

        assert_eq!(request.selection, selection);
        assert_eq!(request.screen_delta, GraphVector::new(12.0, -8.0));
        assert_eq!(request.pan_delta, GraphVector::new(12.0, -8.0));
        assert_eq!(
            request.next_pan_zoom(NodeGraphPanZoom::new(GraphVector::new(20.0, 5.0), 1.5)),
            NodeGraphPanZoom::new(GraphVector::new(32.0, -3.0), 1.5)
        );
    }

    #[test]
    fn drag_request_continues_outside_original_node_bounds_as_metadata() {
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            NodeGraphPanZoom::default(),
        );
        let selection =
            NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(7)));

        let request = NodeGraphSelectedNodeMoveRequest::new(
            viewport,
            selection,
            GraphVector::new(500.0, -360.0),
        );

        assert_eq!(
            request.nodes,
            vec![kinetik_ui_widgets::NodeGraphNodeMove {
                node: NodeId::from_raw(7),
                delta: GraphVector::new(500.0, -360.0),
            }]
        );
        assert!(!request.is_noop());
    }

    #[test]
    fn empty_selection_and_stale_node_ids_are_deterministic_for_drag() {
        let viewport = NodeGraphViewport::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            NodeGraphPanZoom::new(GraphVector::ZERO, 4.0),
        );

        let empty = NodeGraphSelectedNodeMoveRequest::new(
            viewport,
            NodeGraphSelection::new(),
            GraphVector::new(20.0, 8.0),
        );
        assert!(empty.nodes.is_empty());
        assert_graph_vector_close(empty.graph_delta, GraphVector::new(5.0, 2.0));
        assert!(empty.is_noop());

        let stale = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(99)),
            NodeGraphSelectionTarget::Node(NodeId::from_raw(3)),
        ]);
        let request =
            NodeGraphSelectedNodeMoveRequest::new(viewport, stale, GraphVector::new(-8.0, 12.0));

        assert_eq!(
            request.nodes,
            vec![
                kinetik_ui_widgets::NodeGraphNodeMove {
                    node: NodeId::from_raw(3),
                    delta: GraphVector::new(-2.0, 3.0),
                },
                kinetik_ui_widgets::NodeGraphNodeMove {
                    node: NodeId::from_raw(99),
                    delta: GraphVector::new(-2.0, 3.0),
                },
            ]
        );
    }

    #[test]
    fn static_view_selection_marks_matching_semantic_nodes_only() {
        let graph = static_graph();
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(50)),
            NodeGraphSelectionTarget::Port(PortEndpoint::new(
                NodeId::from_raw(2),
                PortId::from_raw(3),
            )),
            NodeGraphSelectionTarget::Node(NodeId::from_raw(99)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(99)),
            NodeGraphSelectionTarget::Port(PortEndpoint::new(
                NodeId::from_raw(99),
                PortId::from_raw(99),
            )),
        ]);
        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .with_selection(selection)
                .emit()
                .expect("static graph output");

        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("node".to_owned())
                && node.label.as_deref() == Some("Source")
                && node.state.selected
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("edge".to_owned())
                && node.label.as_deref() == Some("Edge 50: Source Out to Target In")
                && node.state.selected
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("port".to_owned())
                && node.label.as_deref() == Some("Input In")
                && node.state.selected
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("node".to_owned())
                && node.label.as_deref() == Some("Target")
                && !node.state.selected
        }));
    }

    #[test]
    fn static_view_emits_deterministic_clipped_primitive_order() {
        let graph = static_graph();
        let style = NodeGraphStyle {
            grid: Some(NodeGraphGridStyle::new(
                100.0,
                kinetik_ui_core::Color::rgba(0.2, 0.2, 0.2, 1.0),
                1.0,
            )),
            ..NodeGraphStyle::default()
        };
        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .with_clip(ClipId::from_raw(99))
                .with_style(style)
                .emit()
                .expect("static graph output");

        assert!(matches!(
            output.primitives.first(),
            Some(Primitive::ClipBegin {
                id,
                rect
            }) if *id == ClipId::from_raw(99)
                && *rect == static_viewport().effective_bounds()
        ));
        assert!(matches!(
            output.primitives.last(),
            Some(Primitive::ClipEnd { id }) if *id == ClipId::from_raw(99)
        ));
        assert!(matches!(output.primitives[1], Primitive::Rect(_)));
        assert!(matches!(output.primitives[2], Primitive::Line(_)));

        let first_edge = output
            .primitives
            .iter()
            .position(|primitive| matches!(primitive, Primitive::Line(_)))
            .expect("edge or grid primitive");
        let first_node = output
            .primitives
            .iter()
            .position(|primitive| {
                matches!(
                    primitive,
                    Primitive::Rect(RectPrimitive {
                        stroke: Some(_),
                        ..
                    })
                )
            })
            .expect("node primitive");
        let first_text = output
            .primitives
            .iter()
            .position(|primitive| matches!(primitive, Primitive::Text(_)))
            .expect("label primitive");

        assert!(first_edge < first_node);
        assert!(first_node < first_text);
    }

    #[test]
    fn static_view_transforms_nodes_and_edges_to_screen_space() {
        let graph = static_graph();
        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .emit()
                .expect("static graph output");

        let edge = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Line(line) => Some(line),
                _ => None,
            })
            .expect("edge line");
        let first_node = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect) if rect.stroke.is_some() => Some(rect),
                _ => None,
            })
            .expect("node rect");

        assert_point_close(edge.from, Point::new(340.0, 180.0));
        assert_point_close(edge.to, Point::new(520.0, 200.0));
        assert_rect_close(first_node.rect, Rect::new(140.0, 100.0, 200.0, 160.0));
    }

    #[test]
    fn static_view_edges_use_resolved_anchors_not_raw_endpoint_guesses() {
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Source",
                    GraphRect::new(10.0, 20.0, 100.0, 80.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "First",
                        PortTypeId::from_raw(10),
                    ),
                    PortDescriptor::new(
                        PortId::from_raw(2),
                        PortDirection::Output,
                        "Second",
                        PortTypeId::from_raw(10),
                    ),
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
                    PortTypeId::from_raw(10),
                )]),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(50),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(2)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
            )],
            frames: Vec::new(),
            groups: Vec::new(),
        };

        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .emit()
                .expect("static graph output");
        let Primitive::Line(edge) = output
            .primitives
            .iter()
            .find(|primitive| matches!(primitive, Primitive::Line(_)))
            .expect("edge line")
        else {
            panic!("expected edge line");
        };

        assert_point_close(edge.from, Point::new(340.0, 206.666_67));
    }

    #[test]
    fn static_view_refuses_to_emit_when_resolution_fails() {
        let graph = NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(
                    vec![PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Output,
                        "Out",
                        PortTypeId::from_raw(10),
                    )],
                ),
            ],
            edges: vec![EdgeDescriptor::new(
                EdgeId::from_raw(1),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(99), PortId::from_raw(2)),
            )],
            frames: Vec::new(),
            groups: Vec::new(),
        };

        assert_eq!(
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph).emit(),
            Err(NodeGraphEmissionError::Edge(
                EdgeResolutionError::MissingNode {
                    edge: EdgeId::from_raw(1),
                    endpoint: EdgeEndpointRole::Target,
                    node: NodeId::from_raw(99),
                }
            ))
        );
    }

    #[test]
    fn static_view_semantics_expose_graph_node_port_and_edge_roles() {
        let graph = static_graph();
        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .emit()
                .expect("static graph output");

        assert_eq!(
            output.semantics[0].role,
            SemanticRole::Custom("node-graph".to_owned())
        );
        assert_eq!(output.semantics[0].label.as_deref(), Some("Node graph"));
        assert!(
            output
                .semantics
                .iter()
                .any(|node| node.role == SemanticRole::Custom("edge".to_owned())
                    && node.label.as_deref() == Some("Edge 50: Source Out to Target In"))
        );
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("node".to_owned())
                && node.label.as_deref() == Some("Source")
                && matches!(node.state.value, Some(SemanticValue::Text(ref value)) if value == "Source")
        }));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("port".to_owned())
                && node.label.as_deref() == Some("Input Bypass")
                && node.state.disabled
                && node.description.as_deref() == Some("Disabled port")
        }));
    }

    #[test]
    fn static_view_port_state_metadata_is_deterministic() {
        let graph = static_graph();
        let style = NodeGraphStyle::default();
        let output =
            NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
                .with_incompatible_ports([PortEndpoint::new(
                    NodeId::from_raw(2),
                    PortId::from_raw(3),
                )])
                .emit()
                .expect("static graph output");

        let port_fills = output
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(RectPrimitive {
                    fill: Some(Brush::Solid(fill)),
                    stroke: Some(_),
                    rect,
                    ..
                }) if is_square_size(*rect, style.port_size) => Some(*fill),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            NodeGraphPortState::from_port(&graph.nodes[0].ports[0], false),
            NodeGraphPortState::Normal
        );
        assert_eq!(
            NodeGraphPortState::from_port(&graph.nodes[0].ports[1], true),
            NodeGraphPortState::Disabled
        );
        assert_eq!(
            NodeGraphPortState::from_port(&graph.nodes[1].ports[0], true),
            NodeGraphPortState::Incompatible
        );
        assert!(port_fills.contains(&style.port.fill));
        assert!(port_fills.contains(&style.disabled_port.fill));
        assert!(port_fills.contains(&style.incompatible_port.fill));
        assert!(output.semantics.iter().any(|node| {
            node.role == SemanticRole::Custom("port".to_owned())
                && node.label.as_deref() == Some("Input In")
                && node.description.as_deref() == Some("Incompatible port")
                && !node.state.disabled
        }));
    }
}
