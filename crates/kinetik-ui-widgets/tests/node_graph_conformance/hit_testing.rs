#[allow(clippy::wildcard_imports)]
use super::common::*;

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
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
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
fn hit_testing_reroutes_prioritizes_topmost_reroute_before_edges() {
    let mut graph = routed_edge_graph(GraphPoint::new(180.0, 20.0));
    graph.reroutes.push(RerouteDescriptor::new(
        RerouteId::from_raw(11),
        "Top bend",
        GraphPoint::new(180.0, 20.0),
    ));
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 200.0),
        NodeGraphPanZoom::default(),
    );
    let config = NodeGraphHitTestConfig::new()
        .with_reroute_size(20.0)
        .with_edge_tolerance(20.0);

    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(180.0, 20.0), config),
        Ok(NodeGraphHitTarget::Reroute(RerouteId::from_raw(11)))
    );

    graph.reroutes[1].enabled = false;
    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(180.0, 20.0), config),
        Ok(NodeGraphHitTarget::Reroute(RerouteId::from_raw(10)))
    );

    graph.reroutes[0].enabled = false;
    assert_eq!(
        graph.hit_test_with_config(viewport, Point::new(180.0, 20.0), config),
        Ok(NodeGraphHitTarget::Edge(EdgeId::from_raw(50)))
    );
}

#[test]
fn hit_testing_frames_groups_and_canvas_are_deterministic() {
    let graph = NodeGraphDescriptor {
        nodes: Vec::new(),
        edges: Vec::new(),
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
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
        reroutes: Vec::new(),
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
