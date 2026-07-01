#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn static_view_selection_marks_matching_semantic_nodes_only() {
    let graph = static_graph();
    let selection = NodeGraphSelection::from_targets([
        NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
        NodeGraphSelectionTarget::Edge(EdgeId::from_raw(50)),
        NodeGraphSelectionTarget::Port(PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3))),
        NodeGraphSelectionTarget::Node(NodeId::from_raw(99)),
        NodeGraphSelectionTarget::Edge(EdgeId::from_raw(99)),
        NodeGraphSelectionTarget::Port(PortEndpoint::new(
            NodeId::from_raw(99),
            PortId::from_raw(99),
        )),
    ]);
    let viewport = NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 700.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(20.0, 10.0), 2.0),
    );
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), viewport, &graph)
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
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
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
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
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
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
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
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(
                    PortId::from_raw(1),
                    PortDirection::Output,
                    "Out",
                    PortTypeId::from_raw(10),
                ),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(
            EdgeId::from_raw(1),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(99), PortId::from_raw(2)),
        )],
        reroutes: Vec::new(),
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
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
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
fn static_view_reroute_semantics_expose_stable_identity_and_label_metadata() {
    let graph = routed_edge_graph(GraphPoint::new(180.0, 20.0));
    let selection = NodeGraphSelection::new()
        .replace(NodeGraphSelectionTarget::Reroute(RerouteId::from_raw(10)));
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), static_viewport(), &graph)
        .with_selection(selection)
        .emit()
        .expect("routed static output");
    let reroute = output
        .semantics
        .iter()
        .find(|node| node.role == SemanticRole::Custom("reroute".to_owned()))
        .expect("reroute semantic node");

    assert_eq!(
        reroute.id,
        WidgetId::from_key("graph").child(("reroute", 10_u64))
    );
    assert_eq!(reroute.label.as_deref(), Some("Bend A"));
    assert!(matches!(
        reroute.state.value,
        Some(SemanticValue::Text(ref value)) if value == "Bend A"
    ));
    assert!(reroute.state.selected);
    assert_rect_close(reroute.bounds, Rect::new(475.0, 95.0, 10.0, 10.0));
}

#[test]
fn static_view_port_state_metadata_is_deterministic() {
    let graph = static_graph();
    let style = NodeGraphStyle::default();
    let viewport = NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 700.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(20.0, 10.0), 2.0),
    );
    let output = NodeGraphStaticView::new(WidgetId::from_key("graph"), viewport, &graph)
        .with_incompatible_ports([PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3))])
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
