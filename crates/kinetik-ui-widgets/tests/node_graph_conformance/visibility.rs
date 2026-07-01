#[allow(clippy::wildcard_imports)]
use super::common::*;

fn visible_projection_node(
    node: u64,
    title: &str,
    rect: GraphRect,
    port: u64,
    direction: PortDirection,
    port_label: &str,
    port_type: PortTypeId,
) -> NodeDescriptor {
    NodeDescriptor::new(NodeId::from_raw(node), title, rect).with_ports(vec![PortDescriptor::new(
        PortId::from_raw(port),
        direction,
        port_label,
        port_type,
    )])
}

fn visible_projection_nodes(number: PortTypeId) -> Vec<NodeDescriptor> {
    vec![
        visible_projection_node(
            1,
            "Left Endpoint",
            GraphRect::new(-160.0, 120.0, 80.0, 40.0),
            1,
            PortDirection::Output,
            "Out",
            number,
        ),
        visible_projection_node(
            2,
            "Right Endpoint",
            GraphRect::new(260.0, 120.0, 80.0, 40.0),
            2,
            PortDirection::Input,
            "In",
            number,
        ),
        visible_projection_node(
            3,
            "Visible Node",
            GraphRect::new(20.0, 20.0, 80.0, 60.0),
            3,
            PortDirection::Input,
            "Visible In",
            number,
        ),
        visible_projection_node(
            4,
            "Far Node",
            GraphRect::new(800.0, 20.0, 80.0, 60.0),
            4,
            PortDirection::Input,
            "Far In",
            number,
        ),
        visible_projection_node(
            5,
            "Upper Source",
            GraphRect::new(700.0, -220.0, 80.0, 40.0),
            5,
            PortDirection::Output,
            "Out",
            number,
        ),
        visible_projection_node(
            6,
            "Upper Target",
            GraphRect::new(900.0, -220.0, 80.0, 40.0),
            6,
            PortDirection::Input,
            "In",
            number,
        ),
    ]
}

fn visible_projection_graph() -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(10);
    NodeGraphDescriptor {
        nodes: visible_projection_nodes(number),
        edges: vec![
            EdgeDescriptor::new(
                EdgeId::from_raw(50),
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            )
            .with_route_points(vec![NodeGraphEdgeRoutePoint::point(GraphPoint::new(
                100.0, 140.0,
            ))]),
            EdgeDescriptor::new(
                EdgeId::from_raw(51),
                PortEndpoint::new(NodeId::from_raw(5), PortId::from_raw(5)),
                PortEndpoint::new(NodeId::from_raw(6), PortId::from_raw(6)),
            ),
        ],
        reroutes: vec![
            RerouteDescriptor::new(
                RerouteId::from_raw(10),
                "Visible Bend",
                GraphPoint::new(120.0, 40.0),
            ),
            RerouteDescriptor::new(
                RerouteId::from_raw(11),
                "Far Bend",
                GraphPoint::new(820.0, 40.0),
            ),
        ],
        frames: vec![
            NodeFrameDescriptor::new(
                NodeFrameId::from_raw(20),
                "Visible Frame",
                GraphRect::new(150.0, 100.0, 80.0, 60.0),
            ),
            NodeFrameDescriptor::new(
                NodeFrameId::from_raw(21),
                "Far Frame",
                GraphRect::new(800.0, 100.0, 80.0, 60.0),
            ),
        ],
        groups: vec![
            NodeGroupDescriptor::new(
                NodeGroupId::from_raw(30),
                "Visible Group",
                GraphRect::new(-20.0, 0.0, 40.0, 40.0),
            ),
            NodeGroupDescriptor::new(
                NodeGroupId::from_raw(31),
                "Far Group",
                GraphRect::new(800.0, 0.0, 40.0, 40.0),
            ),
        ],
    }
}

#[test]
fn node_graph_visible_static_projection_culls_off_viewport_items_and_keeps_order() {
    let graph = visible_projection_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 200.0, 180.0),
        NodeGraphPanZoom::default(),
    );

    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), viewport, &graph)
        .emit()
        .expect("visible static output");
    let line_count = output
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, Primitive::Line(_)))
        .count();
    let edge_labels = output
        .semantics
        .iter()
        .filter(|node| node.role == SemanticRole::Custom("edge".to_owned()))
        .filter_map(|node| node.label.as_deref())
        .collect::<Vec<_>>();
    let reroute_labels = output
        .semantics
        .iter()
        .filter(|node| node.role == SemanticRole::Custom("reroute".to_owned()))
        .filter_map(|node| node.label.as_deref())
        .collect::<Vec<_>>();
    let node_labels = output
        .semantics
        .iter()
        .filter(|node| node.role == SemanticRole::Custom("node".to_owned()))
        .filter_map(|node| node.label.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(line_count, 2);
    assert_eq!(
        edge_labels,
        vec!["Edge 50: Left Endpoint Out to Right Endpoint In"]
    );
    assert_eq!(reroute_labels, vec!["Visible Bend"]);
    assert_eq!(node_labels, vec!["Visible Node"]);
    assert!(output.primitives.iter().all(|primitive| match primitive {
        Primitive::Rect(rect) => rect.rect.x < 500.0,
        Primitive::Text(text) => text.origin.x < 500.0,
        _ => true,
    }));
}

#[test]
fn node_graph_visible_hit_testing_keeps_routed_edges_with_offscreen_endpoints() {
    let graph = visible_projection_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 200.0, 180.0),
        NodeGraphPanZoom::default(),
    );
    let config = NodeGraphHitTestConfig::new().with_edge_tolerance(5.0);

    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(100.0, 140.0), config),
        Ok(NodeGraphHitTarget::Edge(EdgeId::from_raw(50)))
    );
    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(100.0, 10.0), config),
        Ok(NodeGraphHitTarget::Canvas)
    );
}

#[test]
fn node_graph_visible_hit_testing_keeps_edge_positive_tolerance_boundary() {
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Top",
                GraphRect::new(0.0, 0.0, 100.0, 8.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(1),
                PortDirection::Output,
                "Out",
                number,
            )]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Bottom",
                GraphRect::new(100.0, 100.0, 100.0, 8.0),
            )
            .with_ports(vec![PortDescriptor::new(
                PortId::from_raw(2),
                PortDirection::Input,
                "In",
                number,
            )]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(70),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 240.0, 140.0),
        NodeGraphPanZoom::default(),
    );
    let config = NodeGraphHitTestConfig::new().with_edge_tolerance(5.0);

    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(100.0, 109.0), config),
        Ok(NodeGraphHitTarget::Edge(EdgeId::from_raw(70)))
    );
}

#[test]
fn node_graph_visible_hit_testing_projects_frames_groups_and_culled_candidates() {
    let graph = visible_projection_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 200.0, 180.0),
        NodeGraphPanZoom::default(),
    );

    assert_eq!(
        graph.hit_test(viewport, Point::new(175.0, 125.0)),
        Ok(NodeGraphHitTarget::Frame(NodeFrameId::from_raw(20)))
    );
    assert_eq!(
        graph.hit_test(viewport, Point::new(5.0, 20.0)),
        Ok(NodeGraphHitTarget::Group(NodeGroupId::from_raw(30)))
    );
    assert_eq!(
        graph.hit_test(viewport, Point::new(199.0, 20.0)),
        Ok(NodeGraphHitTarget::Canvas)
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
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
        frames: vec![
            NodeFrameDescriptor::new(duplicate, "First", GraphRect::new(0.0, 0.0, 100.0, 100.0)),
            NodeFrameDescriptor::new(duplicate, "Second", GraphRect::new(0.0, 0.0, 100.0, 100.0)),
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
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: vec![
            NodeGroupDescriptor::new(duplicate, "First", GraphRect::new(0.0, 0.0, 100.0, 100.0)),
            NodeGroupDescriptor::new(duplicate, "Second", GraphRect::new(0.0, 0.0, 100.0, 100.0)),
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
fn frame_and_group_membership_validation_is_deterministic() {
    let frame = NodeFrameId::from_raw(7);
    let group = NodeGroupId::from_raw(8);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(3),
                "Third",
                GraphRect::new(30.0, 0.0, 20.0, 20.0),
            )
            .with_frame(frame),
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "First",
                GraphRect::new(0.0, 0.0, 20.0, 20.0),
            )
            .with_frame(frame),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Second",
                GraphRect::new(15.0, 0.0, 20.0, 20.0),
            )
            .with_group(group),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: vec![NodeFrameDescriptor::new(
            frame,
            "Frame",
            GraphRect::new(-10.0, -10.0, 80.0, 50.0),
        )],
        groups: vec![
            NodeGroupDescriptor::new(group, "Group", GraphRect::new(-5.0, -5.0, 60.0, 40.0))
                .with_nodes(vec![NodeId::from_raw(1)]),
        ],
    };

    assert_eq!(graph.validate(), Ok(()));
    assert_eq!(
        graph.frame_member_nodes(frame),
        Ok(vec![NodeId::from_raw(1), NodeId::from_raw(3)])
    );
    assert_eq!(
        graph.group_member_nodes(group),
        Ok(vec![NodeId::from_raw(1), NodeId::from_raw(2)])
    );

    let duplicate_group_member = NodeGraphDescriptor {
        nodes: vec![NodeDescriptor::new(
            NodeId::from_raw(1),
            "Node",
            GraphRect::ZERO,
        )],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: vec![
            NodeGroupDescriptor::new(group, "Group", GraphRect::ZERO)
                .with_nodes(vec![NodeId::from_raw(1), NodeId::from_raw(1)]),
        ],
    };
    assert_eq!(
        duplicate_group_member.validate(),
        Err(NodeGraphValidationError::DuplicateGroupMember {
            group,
            node: NodeId::from_raw(1),
        })
    );

    let conflicting_group_member = NodeGraphDescriptor {
        nodes: vec![NodeDescriptor::new(
            NodeId::from_raw(1),
            "Node",
            GraphRect::ZERO,
        )],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: vec![
            NodeGroupDescriptor::new(NodeGroupId::from_raw(8), "A", GraphRect::ZERO)
                .with_nodes(vec![NodeId::from_raw(1)]),
            NodeGroupDescriptor::new(NodeGroupId::from_raw(9), "B", GraphRect::ZERO)
                .with_nodes(vec![NodeId::from_raw(1)]),
        ],
    };
    assert_eq!(
        conflicting_group_member.validate(),
        Err(NodeGraphValidationError::DuplicateGroupMembership {
            node: NodeId::from_raw(1),
            first: NodeGroupId::from_raw(8),
            second: NodeGroupId::from_raw(9),
        })
    );
}
