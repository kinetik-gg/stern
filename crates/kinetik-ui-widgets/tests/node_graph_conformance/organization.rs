#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn parent_frame_movement_produces_child_deltas_deterministically() {
    let frame = NodeFrameId::from_raw(7);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(3),
                "Third",
                GraphRect::new(20.0, 0.0, 10.0, 10.0),
            )
            .with_frame(frame),
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "First",
                GraphRect::new(0.0, 0.0, 10.0, 10.0),
            )
            .with_frame(frame),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Outside",
                GraphRect::new(80.0, 0.0, 10.0, 10.0),
            ),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: vec![NodeFrameDescriptor::new(
            frame,
            "Frame",
            GraphRect::new(-10.0, -10.0, 60.0, 40.0),
        )],
        groups: Vec::new(),
    };
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::ZERO, 2.0),
    );

    let request = graph
        .move_frame_request(viewport, frame, GraphVector::new(40.0, -20.0))
        .expect("frame move metadata");

    assert_eq!(request.screen_delta, GraphVector::new(40.0, -20.0));
    assert_graph_vector_close(request.graph_delta, GraphVector::new(20.0, -10.0));
    assert_eq!(
        request.frame,
        NodeGraphFrameMove {
            frame,
            delta: GraphVector::new(20.0, -10.0),
        }
    );
    assert_eq!(
        request.children,
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
fn collapse_metadata_preserves_port_and_link_identity() {
    let frame = NodeFrameId::from_raw(7);
    let number = PortTypeId::from_raw(10);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(
                NodeId::from_raw(1),
                "Inside",
                GraphRect::new(0.0, 0.0, 100.0, 80.0),
            )
            .with_frame(frame)
            .with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
            ]),
            NodeDescriptor::new(
                NodeId::from_raw(2),
                "Outside",
                GraphRect::new(160.0, 0.0, 100.0, 80.0),
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
        frames: vec![
            NodeFrameDescriptor::new(frame, "Frame", GraphRect::new(-10.0, -10.0, 140.0, 100.0))
                .with_collapsed(false),
        ],
        groups: Vec::new(),
    };

    let request = graph
        .collapse_frame_request(frame, true)
        .expect("collapse metadata");

    assert_eq!(request.target, NodeGraphCollapseTarget::Frame(frame));
    assert!(!request.previous_collapsed);
    assert!(request.collapsed);
    assert_eq!(request.nodes, vec![NodeId::from_raw(1)]);
    assert_eq!(
        request.ports,
        vec![
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(2)),
        ]
    );
    assert_eq!(
        request.links,
        vec![NodeGraphCollapseLinkMetadata {
            edge: EdgeId::from_raw(50),
            from: PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            to: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
        }]
    );
    assert!(!request.is_noop());
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.nodes[0].ports.len(), 2);
}

#[test]
fn mute_and_bypass_emit_state_action_metadata_only() {
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Node", GraphRect::ZERO)
                .with_muted(true)
                .with_bypassed(false),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let mute = graph
        .mute_node_request(NodeId::from_raw(1), false)
        .expect("mute request");
    let bypass = graph
        .bypass_node_request(NodeId::from_raw(1), true)
        .expect("bypass request");

    assert_eq!(mute.node, NodeId::from_raw(1));
    assert_eq!(mute.action, NodeGraphNodeStateAction::Mute);
    assert!(mute.previous);
    assert!(!mute.requested);
    assert!(!mute.is_noop());
    assert_eq!(bypass.node, NodeId::from_raw(1));
    assert_eq!(bypass.action, NodeGraphNodeStateAction::Bypass);
    assert!(!bypass.previous);
    assert!(bypass.requested);
    assert!(!bypass.is_noop());
    assert!(graph.nodes[0].muted);
    assert!(!graph.nodes[0].bypassed);
}

#[test]
fn label_and_comment_semantics_are_deterministic_metadata() {
    let group = NodeGroupId::from_raw(8);
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Node", GraphRect::ZERO)
                .with_label("Display A")
                .with_comment("Existing note"),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: vec![
            NodeGroupDescriptor::new(group, "Group", GraphRect::ZERO)
                .with_comment("Old group comment"),
        ],
    };

    let label = graph
        .label_request(
            NodeGraphOrganizationTarget::Node(NodeId::from_raw(1)),
            "Display B",
        )
        .expect("label request");
    let comment = graph
        .comment_request(
            NodeGraphOrganizationTarget::Group(group),
            "New group comment",
        )
        .expect("comment request");

    assert_eq!(
        label.target,
        NodeGraphOrganizationTarget::Node(NodeId::from_raw(1))
    );
    assert_eq!(label.field, NodeGraphAnnotationField::Label);
    assert_eq!(label.previous.as_deref(), Some("Display A"));
    assert_eq!(label.requested.as_deref(), Some("Display B"));
    assert!(!label.is_noop());
    assert_eq!(comment.target, NodeGraphOrganizationTarget::Group(group));
    assert_eq!(comment.field, NodeGraphAnnotationField::Comment);
    assert_eq!(comment.previous.as_deref(), Some("Old group comment"));
    assert_eq!(comment.requested.as_deref(), Some("New group comment"));
    assert!(!comment.is_noop());
    assert_eq!(graph.nodes[0].label.as_deref(), Some("Display A"));
    assert_eq!(
        graph.groups[0].comment.as_deref(),
        Some("Old group comment")
    );
}
