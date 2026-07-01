#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn context_targets_preserve_hit_identity_for_all_graph_surfaces() {
    let endpoint = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(2));

    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::NodeTitle(NodeId::from_raw(
            10
        ))),
        NodeGraphContextTarget::Node(NodeId::from_raw(10))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::NodeBody(NodeId::from_raw(11))),
        NodeGraphContextTarget::Node(NodeId::from_raw(11))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Edge(EdgeId::from_raw(50))),
        NodeGraphContextTarget::Edge(EdgeId::from_raw(50))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Reroute(RerouteId::from_raw(
            60
        ))),
        NodeGraphContextTarget::Reroute(RerouteId::from_raw(60))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Port(endpoint)),
        NodeGraphContextTarget::Port(endpoint)
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Frame(NodeFrameId::from_raw(
            7
        ))),
        NodeGraphContextTarget::Frame(NodeFrameId::from_raw(7))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Group(NodeGroupId::from_raw(
            8
        ))),
        NodeGraphContextTarget::Group(NodeGroupId::from_raw(8))
    );
    assert_eq!(
        NodeGraphContextTarget::from_hit_target(NodeGraphHitTarget::Canvas),
        NodeGraphContextTarget::Canvas
    );
}

#[test]
fn context_delete_and_duplicate_requests_preserve_selected_targets() {
    let graph = link_edit_graph();
    let selection = NodeGraphSelection::from_targets([
        NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
        NodeGraphSelectionTarget::Edge(EdgeId::from_raw(50)),
        NodeGraphSelectionTarget::Port(PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2))),
    ]);

    let actions = graph.context_actions(
        NodeGraphContextTarget::Node(NodeId::from_raw(1)),
        &selection,
    );
    let delete = context_action(&actions, NodeGraphContextActionKind::Delete);
    let duplicate = context_action(&actions, NodeGraphContextActionKind::Duplicate);

    assert!(delete.enabled);
    assert!(duplicate.enabled);
    assert_eq!(
        delete.target,
        NodeGraphContextTarget::Node(NodeId::from_raw(1))
    );
    assert_eq!(
        delete.request,
        Some(NodeGraphContextActionRequest::Delete(
            kinetik_ui_widgets::NodeGraphContextSelectionRequest {
                target: NodeGraphContextTarget::Node(NodeId::from_raw(1)),
                selected_targets: selection.selected(),
            }
        ))
    );
    assert_eq!(
        duplicate.request,
        Some(NodeGraphContextActionRequest::Duplicate(
            kinetik_ui_widgets::NodeGraphContextSelectionRequest {
                target: NodeGraphContextTarget::Node(NodeId::from_raw(1)),
                selected_targets: selection.selected(),
            }
        ))
    );

    let unselected = graph.context_actions(
        NodeGraphContextTarget::Port(PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(5))),
        &selection,
    );
    let unselected_delete = context_action(&unselected, NodeGraphContextActionKind::Delete);
    assert_eq!(
        unselected_delete.request,
        Some(NodeGraphContextActionRequest::Delete(
            kinetik_ui_widgets::NodeGraphContextSelectionRequest {
                target: NodeGraphContextTarget::Port(PortEndpoint::new(
                    NodeId::from_raw(1),
                    PortId::from_raw(5),
                )),
                selected_targets: vec![NodeGraphSelectionTarget::Port(PortEndpoint::new(
                    NodeId::from_raw(1),
                    PortId::from_raw(5),
                ))],
            }
        ))
    );
}

#[test]
fn node_graph_context_default_catalog_keeps_compatibility_order() {
    let graph = link_edit_graph();
    let actions = graph.context_actions(
        NodeGraphContextTarget::Canvas,
        &NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(1))),
    );

    assert_eq!(
        actions.iter().map(|action| action.kind).collect::<Vec<_>>(),
        DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS
    );

    let paste = context_action(&actions, NodeGraphContextActionKind::Paste);
    assert!(!paste.enabled);
    assert_eq!(paste.request, None);
    assert_eq!(
        paste.unavailable_reason,
        Some(NodeGraphContextActionUnavailableReason::RequiresApplicationState)
    );
}

#[test]
fn node_graph_context_request_builders_do_not_require_default_catalog() {
    let graph = link_edit_graph();
    let selection = NodeGraphSelection::from_targets([
        NodeGraphSelectionTarget::Node(NodeId::from_raw(1)),
        NodeGraphSelectionTarget::Edge(EdgeId::from_raw(50)),
    ]);
    let node_target = NodeGraphContextTarget::Node(NodeId::from_raw(1));
    let edge_target = NodeGraphContextTarget::Edge(EdgeId::from_raw(50));
    let source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));

    let delete = graph
        .delete_context_request(node_target, &selection)
        .expect("delete request");
    let duplicate = graph
        .duplicate_context_request(node_target, &selection)
        .expect("duplicate request");
    assert_eq!(delete.target, node_target);
    assert_eq!(delete.selected_targets, selection.selected());
    assert_eq!(duplicate.target, node_target);
    assert_eq!(duplicate.selected_targets, selection.selected());

    let disconnect = graph
        .disconnect_context_request(NodeGraphContextTarget::Port(source))
        .expect("disconnect request");
    match disconnect.disconnect {
        NodeGraphContextDisconnectTarget::Endpoint {
            endpoint,
            connected_edges,
        } => {
            assert_eq!(endpoint, source);
            assert_eq!(connected_edges, vec![EdgeId::from_raw(50)]);
        }
        request @ NodeGraphContextDisconnectTarget::Edge(_) => {
            panic!("expected endpoint disconnect request, got {request:?}")
        }
    }

    let detach = graph
        .detach_context_request(edge_target, EdgeEndpointRole::Target)
        .expect("detach request");
    assert_eq!(detach.target, edge_target);
    assert_eq!(detach.request.detached, EdgeEndpointRole::Target);

    let organization = graph
        .organization_context_request(
            node_target,
            &selection,
            NodeGraphContextOrganizationOperation::FrameSelection,
        )
        .expect("organization request");
    assert_eq!(
        organization.operation,
        NodeGraphContextOrganizationOperation::FrameSelection
    );
    assert_eq!(organization.selected_targets, selection.selected());

    let select_all = graph
        .select_all_context_request(NodeGraphContextTarget::Canvas, &selection)
        .expect("select all request");
    assert_eq!(
        select_all.operation,
        NodeGraphContextCanvasOperation::SelectAll
    );
    assert!(
        select_all
            .selectable_targets
            .contains(&NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
    );

    let paste = graph
        .paste_context_request(NodeGraphContextTarget::Canvas, &selection)
        .expect("paste request");
    assert_eq!(paste.operation, NodeGraphContextCanvasOperation::Paste);
    assert_eq!(paste.selection, selection);
    assert!(paste.selectable_targets.is_empty());

    let custom_paste = graph.context_action(
        NodeGraphContextActionKind::Paste,
        NodeGraphContextTarget::Canvas,
        &selection,
    );
    assert!(custom_paste.enabled);
    assert!(matches!(
        custom_paste.request,
        Some(NodeGraphContextActionRequest::Canvas(_))
    ));
}

#[test]
fn context_disconnect_and_detach_requests_preserve_link_identity() {
    let graph = link_edit_graph();
    let edge = EdgeId::from_raw(50);
    let source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));
    let target = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2));

    let edge_actions = graph.context_actions(
        NodeGraphContextTarget::Edge(edge),
        &NodeGraphSelection::new(),
    );
    let disconnect = context_action(&edge_actions, NodeGraphContextActionKind::Disconnect);
    assert!(disconnect.enabled);
    match disconnect.request.as_ref().expect("disconnect request") {
        NodeGraphContextActionRequest::Disconnect(request) => match &request.disconnect {
            NodeGraphContextDisconnectTarget::Edge(context) => {
                assert_eq!(request.target, NodeGraphContextTarget::Edge(edge));
                assert_eq!(context.edge, edge);
                assert_eq!(context.from.endpoint, source);
                assert_eq!(context.to.endpoint, target);
            }
            request @ NodeGraphContextDisconnectTarget::Endpoint { .. } => {
                panic!("expected edge disconnect request, got {request:?}")
            }
        },
        request => panic!("expected disconnect request, got {request:?}"),
    }

    let detach_source = context_action(&edge_actions, NodeGraphContextActionKind::DetachSource);
    match detach_source
        .request
        .as_ref()
        .expect("detach source request")
    {
        NodeGraphContextActionRequest::DetachEndpoint(request) => {
            assert_eq!(request.target, NodeGraphContextTarget::Edge(edge));
            assert_eq!(request.request.edge.edge, edge);
            assert_eq!(request.request.detached, EdgeEndpointRole::Source);
            assert_eq!(request.request.endpoint.endpoint, source);
        }
        request => panic!("expected detach endpoint request, got {request:?}"),
    }

    let detach_target = context_action(&edge_actions, NodeGraphContextActionKind::DetachTarget);
    match detach_target
        .request
        .as_ref()
        .expect("detach target request")
    {
        NodeGraphContextActionRequest::DetachEndpoint(request) => {
            assert_eq!(request.target, NodeGraphContextTarget::Edge(edge));
            assert_eq!(request.request.edge.edge, edge);
            assert_eq!(request.request.detached, EdgeEndpointRole::Target);
            assert_eq!(request.request.endpoint.endpoint, target);
        }
        request => panic!("expected detach endpoint request, got {request:?}"),
    }

    let endpoint_actions = graph.context_actions(
        NodeGraphContextTarget::Port(source),
        &NodeGraphSelection::new(),
    );
    let disconnect_endpoint =
        context_action(&endpoint_actions, NodeGraphContextActionKind::Disconnect);
    match disconnect_endpoint
        .request
        .as_ref()
        .expect("endpoint disconnect request")
    {
        NodeGraphContextActionRequest::Disconnect(request) => match &request.disconnect {
            NodeGraphContextDisconnectTarget::Endpoint {
                endpoint,
                connected_edges,
            } => {
                assert_eq!(request.target, NodeGraphContextTarget::Port(source));
                assert_eq!(*endpoint, source);
                assert_eq!(connected_edges, &vec![edge]);
            }
            request @ NodeGraphContextDisconnectTarget::Edge(_) => {
                panic!("expected endpoint disconnect request, got {request:?}")
            }
        },
        request => panic!("expected disconnect request, got {request:?}"),
    }
}

#[test]
fn context_frame_group_and_canvas_actions_are_metadata_only() {
    let graph = NodeGraphDescriptor {
        nodes: vec![NodeDescriptor::new(
            NodeId::from_raw(1),
            "Node",
            GraphRect::ZERO,
        )],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: vec![NodeFrameDescriptor::new(
            NodeFrameId::from_raw(7),
            "Frame",
            GraphRect::new(-10.0, -10.0, 120.0, 120.0),
        )],
        groups: vec![NodeGroupDescriptor::new(
            NodeGroupId::from_raw(8),
            "Group",
            GraphRect::new(0.0, 0.0, 100.0, 100.0),
        )],
    };
    let selection =
        NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)));

    let node_actions = graph.context_actions(
        NodeGraphContextTarget::Node(NodeId::from_raw(1)),
        &selection,
    );
    let frame_selection = context_action(&node_actions, NodeGraphContextActionKind::FrameSelection);
    match frame_selection.request.as_ref().expect("frame request") {
        NodeGraphContextActionRequest::Organization(request) => {
            assert_eq!(
                request.operation,
                NodeGraphContextOrganizationOperation::FrameSelection
            );
            assert_eq!(
                request.selected_targets,
                vec![NodeGraphSelectionTarget::Node(NodeId::from_raw(1))]
            );
        }
        request => panic!("expected organization request, got {request:?}"),
    }

    let group_actions = graph.context_actions(
        NodeGraphContextTarget::Group(NodeGroupId::from_raw(8)),
        &selection,
    );
    let ungroup = context_action(&group_actions, NodeGraphContextActionKind::Ungroup);
    assert_eq!(
        ungroup.target,
        NodeGraphContextTarget::Group(NodeGroupId::from_raw(8))
    );
    match ungroup.request.as_ref().expect("ungroup request") {
        NodeGraphContextActionRequest::Organization(request) => {
            assert_eq!(
                request.operation,
                NodeGraphContextOrganizationOperation::Ungroup
            );
            assert_eq!(
                request.target,
                NodeGraphContextTarget::Group(NodeGroupId::from_raw(8))
            );
            assert!(request.selected_targets.is_empty());
        }
        request => panic!("expected organization request, got {request:?}"),
    }

    let frame_actions = graph.context_actions(
        NodeGraphContextTarget::Frame(NodeFrameId::from_raw(7)),
        &selection,
    );
    assert_eq!(
        context_action(&frame_actions, NodeGraphContextActionKind::Delete).request,
        Some(NodeGraphContextActionRequest::Delete(
            kinetik_ui_widgets::NodeGraphContextSelectionRequest {
                target: NodeGraphContextTarget::Frame(NodeFrameId::from_raw(7)),
                selected_targets: Vec::new(),
            }
        ))
    );

    let canvas_actions = graph.context_actions(NodeGraphContextTarget::Canvas, &selection);
    let select_all = context_action(&canvas_actions, NodeGraphContextActionKind::SelectAll);
    match select_all.request.as_ref().expect("select all request") {
        NodeGraphContextActionRequest::Canvas(request) => {
            assert_eq!(request.target, NodeGraphContextTarget::Canvas);
            assert_eq!(
                request.operation,
                NodeGraphContextCanvasOperation::SelectAll
            );
            assert_eq!(request.selection, selection);
            assert!(
                request
                    .selectable_targets
                    .contains(&NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
            );
        }
        request => panic!("expected canvas request, got {request:?}"),
    }
}

#[test]
fn disabled_and_unavailable_context_actions_are_deterministic() {
    let number = PortTypeId::from_raw(10);
    let source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));
    let target = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2));
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Source", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(1), PortDirection::Output, "Out", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(2), "Target", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "In", number),
            ]),
            NodeDescriptor::new(NodeId::from_raw(3), "Unused", GraphRect::ZERO).with_ports(vec![
                PortDescriptor::new(PortId::from_raw(3), PortDirection::Output, "Unused", number),
            ]),
        ],
        edges: vec![EdgeDescriptor::new(EdgeId::from_raw(50), source, target).with_enabled(false)],
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };

    let first = graph.context_actions(
        NodeGraphContextTarget::Edge(EdgeId::from_raw(50)),
        &NodeGraphSelection::new(),
    );
    let second = graph.context_actions(
        NodeGraphContextTarget::Edge(EdgeId::from_raw(50)),
        &NodeGraphSelection::new(),
    );
    assert_eq!(first, second);

    let disabled_disconnect = context_action(&first, NodeGraphContextActionKind::Disconnect);
    assert!(!disabled_disconnect.enabled);
    assert_eq!(disabled_disconnect.request, None);
    assert_eq!(
        disabled_disconnect.unavailable_reason,
        Some(NodeGraphContextActionUnavailableReason::DisabledTarget)
    );

    let unused_endpoint = PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(3));
    let endpoint_actions = graph.context_actions(
        NodeGraphContextTarget::Port(unused_endpoint),
        &NodeGraphSelection::new(),
    );
    let unavailable_disconnect =
        context_action(&endpoint_actions, NodeGraphContextActionKind::Disconnect);
    assert!(!unavailable_disconnect.enabled);
    assert_eq!(
        unavailable_disconnect.unavailable_reason,
        Some(NodeGraphContextActionUnavailableReason::NoConnectedEdges)
    );

    let empty_canvas = NodeGraphDescriptor::new()
        .context_actions(NodeGraphContextTarget::Canvas, &NodeGraphSelection::new());
    assert_eq!(
        context_action(&empty_canvas, NodeGraphContextActionKind::Delete).unavailable_reason,
        Some(NodeGraphContextActionUnavailableReason::EmptySelection)
    );
    assert_eq!(
        context_action(&empty_canvas, NodeGraphContextActionKind::Paste).unavailable_reason,
        Some(NodeGraphContextActionUnavailableReason::RequiresApplicationState)
    );
}

#[test]
fn node_graph_context_request_builders_return_deterministic_unavailable_reasons() {
    let graph = NodeGraphDescriptor {
        nodes: vec![
            NodeDescriptor::new(NodeId::from_raw(1), "Disabled", GraphRect::ZERO)
                .with_enabled(false),
        ],
        edges: Vec::new(),
        reroutes: Vec::new(),
        frames: Vec::new(),
        groups: Vec::new(),
    };
    let selection =
        NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)));

    assert_eq!(
        graph
            .delete_context_request(
                NodeGraphContextTarget::Node(NodeId::from_raw(99)),
                &selection
            )
            .unwrap_err(),
        NodeGraphContextActionUnavailableReason::MissingTarget
    );
    assert_eq!(
        graph
            .duplicate_context_request(
                NodeGraphContextTarget::Node(NodeId::from_raw(1)),
                &selection
            )
            .unwrap_err(),
        NodeGraphContextActionUnavailableReason::DisabledTarget
    );
    assert_eq!(
        graph
            .detach_context_request(
                NodeGraphContextTarget::Node(NodeId::from_raw(1)),
                EdgeEndpointRole::Source,
            )
            .unwrap_err(),
        NodeGraphContextActionUnavailableReason::UnsupportedTarget
    );
    assert_eq!(
        graph
            .select_all_context_request(NodeGraphContextTarget::Canvas, &selection)
            .unwrap_err(),
        NodeGraphContextActionUnavailableReason::EmptySelection
    );
    assert_eq!(
        graph
            .paste_context_request(
                NodeGraphContextTarget::Node(NodeId::from_raw(1)),
                &selection
            )
            .unwrap_err(),
        NodeGraphContextActionUnavailableReason::UnsupportedTarget
    );
}
