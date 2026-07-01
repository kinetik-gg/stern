#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn link_draft_starts_from_allowed_endpoint() {
    let graph = link_draft_graph();
    let start = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));
    let draft = graph
        .start_link_draft(start, Point::new(f32::NAN, 12.0))
        .expect("link draft should start from enabled port");

    assert_eq!(
        draft.start,
        NodeGraphLinkDraftEndpoint {
            endpoint: start,
            direction: PortDirection::Output,
            port_type: PortTypeId::from_raw(10),
            anchor: GraphPoint::new(100.0, 60.0),
        }
    );
    assert_point_close(draft.current_pointer, Point::new(0.0, 12.0));
    assert_eq!(draft.current_graph_point, None);
    assert_eq!(
        draft.target,
        NodeGraphLinkDraftTarget::Hit(NodeGraphHitTarget::Canvas)
    );
    assert_eq!(
        graph.start_link_draft(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(9)),
            Point::new(0.0, 0.0)
        ),
        Err(NodeGraphLinkDraftEndpointError::DisabledPort {
            node: NodeId::from_raw(1),
            port: PortId::from_raw(9),
        })
    );
}

#[test]
fn link_draft_hover_target_reports_compatible_and_incompatible_ports() {
    let graph = link_draft_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 240.0),
        NodeGraphPanZoom::default(),
    );
    let config = NodeGraphHitTestConfig::new().with_port_size(24.0);
    let draft = graph
        .start_link_draft(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            Point::new(100.0, 60.0),
        )
        .expect("link draft should start");

    let compatible = draft
        .resolve_hover_target_with_config(&graph, viewport, Point::new(200.0, 40.0), config)
        .expect("compatible hover target should resolve");
    assert!(compatible.target.is_compatible());
    assert_graph_point_close(
        compatible.current_graph_point.expect("graph pointer"),
        GraphPoint::new(200.0, 40.0),
    );
    assert_eq!(
        compatible.target,
        NodeGraphLinkDraftTarget::Port(kinetik_ui_widgets::NodeGraphLinkDraftPortTarget {
            endpoint: NodeGraphLinkDraftEndpoint {
                endpoint: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
                direction: PortDirection::Input,
                port_type: PortTypeId::from_raw(10),
                anchor: GraphPoint::new(200.0, 40.0),
            },
            compatibility: Ok(()),
        })
    );

    let incompatible = draft
        .resolve_hover_target_with_config(&graph, viewport, Point::new(200.0, 80.0), config)
        .expect("incompatible hover target should resolve");
    assert_eq!(
        incompatible.target,
        NodeGraphLinkDraftTarget::Port(kinetik_ui_widgets::NodeGraphLinkDraftPortTarget {
            endpoint: NodeGraphLinkDraftEndpoint {
                endpoint: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
                direction: PortDirection::Input,
                port_type: PortTypeId::from_raw(11),
                anchor: GraphPoint::new(200.0, 80.0),
            },
            compatibility: Err(PortCompatibilityError::TypeMismatch {
                output: PortTypeId::from_raw(10),
                input: PortTypeId::from_raw(11),
            }),
        })
    );
}

#[test]
fn link_draft_cancel_behavior_is_deterministic() {
    let graph = link_draft_graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 240.0),
        NodeGraphPanZoom::default(),
    );
    let draft = graph
        .start_link_draft(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            Point::new(100.0, 60.0),
        )
        .expect("link draft should start")
        .resolve_hover_target(&graph, viewport, Point::new(10.0, 10.0))
        .expect("non-port hover target should resolve");

    assert_eq!(draft.cancel(), draft.cancel());
    match draft.cancel() {
        NodeGraphLinkDraftOutcome::Cancelled(cancelled) => {
            assert_eq!(cancelled.start, draft.start);
            assert_point_close(cancelled.current_pointer, Point::new(10.0, 10.0));
            assert_eq!(
                cancelled.current_graph_point,
                Some(GraphPoint::new(10.0, 10.0))
            );
            assert_eq!(
                cancelled.target,
                NodeGraphLinkDraftTarget::Hit(NodeGraphHitTarget::NodeTitle(NodeId::from_raw(1)))
            );
        }
        outcome => panic!("expected cancelled outcome, got {outcome:?}"),
    }
}

#[test]
fn link_draft_completion_returns_metadata_without_mutating_descriptors() {
    let graph = link_draft_graph();
    let original = graph.clone();
    let viewport = NodeGraphViewport::new(
        Rect::new(0.0, 0.0, 400.0, 240.0),
        NodeGraphPanZoom::default(),
    );
    let config = NodeGraphHitTestConfig::new().with_port_size(24.0);
    let draft = graph
        .start_link_draft(
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            Point::new(200.0, 40.0),
        )
        .expect("link draft can start from input")
        .resolve_hover_target_with_config(&graph, viewport, Point::new(100.0, 60.0), config)
        .expect("output hover target should resolve");

    match draft.complete() {
        NodeGraphLinkDraftOutcome::Completed(completed) => {
            assert_eq!(
                completed.from.endpoint,
                PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1))
            );
            assert_eq!(completed.from.direction, PortDirection::Output);
            assert_eq!(
                completed.to.endpoint,
                PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2))
            );
            assert_eq!(completed.to.direction, PortDirection::Input);
            assert_point_close(completed.current_pointer, Point::new(100.0, 60.0));
            assert_eq!(
                completed.current_graph_point,
                Some(GraphPoint::new(100.0, 60.0))
            );
        }
        outcome => panic!("expected completed outcome, got {outcome:?}"),
    }
    assert_eq!(graph, original);
    assert!(graph.edges.is_empty());

    let rejected = graph
        .start_link_draft(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            Point::new(100.0, 60.0),
        )
        .expect("link draft should start")
        .resolve_hover_target_with_config(&graph, viewport, Point::new(200.0, 80.0), config)
        .expect("incompatible hover target should resolve")
        .complete();

    assert!(matches!(
        rejected,
        NodeGraphLinkDraftOutcome::Rejected(kinetik_ui_widgets::NodeGraphLinkDraftRejected {
            error: NodeGraphLinkDraftCompletionError::IncompatiblePort {
                error: PortCompatibilityError::TypeMismatch {
                    output,
                    input,
                },
                ..
            },
            ..
        }) if output == PortTypeId::from_raw(10) && input == PortTypeId::from_raw(11)
    ));
    assert_eq!(graph, original);
}

#[test]
fn create_link_request_preserves_endpoint_identity_without_mutation() {
    let graph = link_edit_graph();
    let original = graph.clone();
    let from = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(5));
    let to = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3));

    let request = graph
        .create_link_request(from, to)
        .expect("compatible create request");

    match request {
        NodeGraphLinkEditRequest::CreateLink(request) => {
            assert_eq!(request.from.endpoint, from);
            assert_eq!(request.from.direction, PortDirection::Output);
            assert_eq!(request.from.port_type, PortTypeId::from_raw(10));
            assert_eq!(request.to.endpoint, to);
            assert_eq!(request.to.direction, PortDirection::Input);
            assert_eq!(request.to.port_type, PortTypeId::from_raw(10));
        }
        request => panic!("expected create-link request, got {request:?}"),
    }

    assert_eq!(graph, original);
}

#[test]
fn reconnect_link_requests_preserve_edge_context() {
    let graph = link_edit_graph();
    let edge = EdgeId::from_raw(50);
    let old_source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));
    let old_target = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2));
    let new_source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(5));
    let new_target = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3));

    let source_request = graph
        .reconnect_link_source_request(edge, new_source)
        .expect("compatible source reconnect");
    match source_request {
        NodeGraphLinkEditRequest::ReconnectSource(request) => {
            assert_eq!(request.edge.edge, edge);
            assert_eq!(request.edge.from.endpoint, old_source);
            assert_eq!(request.edge.to.endpoint, old_target);
            assert_eq!(request.old_source.endpoint, old_source);
            assert_eq!(request.new_source.endpoint, new_source);
            assert_eq!(request.target.endpoint, old_target);
        }
        request => panic!("expected reconnect-source request, got {request:?}"),
    }

    let target_request = graph
        .reconnect_link_target_request(edge, new_target)
        .expect("compatible target reconnect");
    match target_request {
        NodeGraphLinkEditRequest::ReconnectTarget(request) => {
            assert_eq!(request.edge.edge, edge);
            assert_eq!(request.edge.from.endpoint, old_source);
            assert_eq!(request.edge.to.endpoint, old_target);
            assert_eq!(request.source.endpoint, old_source);
            assert_eq!(request.old_target.endpoint, old_target);
            assert_eq!(request.new_target.endpoint, new_target);
        }
        request => panic!("expected reconnect-target request, got {request:?}"),
    }
}

#[test]
fn detach_and_cut_link_requests_preserve_target_identity() {
    let graph = link_edit_graph();
    let edge = EdgeId::from_raw(50);
    let source = PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1));
    let target = PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2));

    let detach = graph
        .detach_link_endpoint_request(edge, EdgeEndpointRole::Target)
        .expect("detach request");
    match detach {
        NodeGraphLinkEditRequest::DetachEdge(request) => {
            assert_eq!(request.edge.edge, edge);
            assert_eq!(request.edge.from.endpoint, source);
            assert_eq!(request.edge.to.endpoint, target);
            assert_eq!(request.detached, EdgeEndpointRole::Target);
            assert_eq!(request.endpoint.endpoint, target);
        }
        request => panic!("expected detach-edge request, got {request:?}"),
    }

    let cut = graph.cut_link_request(edge).expect("cut request");
    match cut {
        NodeGraphLinkEditRequest::CutEdge(request) => {
            assert_eq!(request.edge.edge, edge);
            assert_eq!(request.edge.from.endpoint, source);
            assert_eq!(request.edge.to.endpoint, target);
        }
        request => panic!("expected cut-edge request, got {request:?}"),
    }
}

#[test]
fn disabled_or_incompatible_ports_reject_link_edit_requests() {
    let graph = link_edit_graph();

    assert_eq!(
        graph.create_link_request(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(7)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
        ),
        Err(NodeGraphLinkEditRequestError::Endpoint(
            NodeGraphLinkDraftEndpointError::DisabledPort {
                node: NodeId::from_raw(1),
                port: PortId::from_raw(7),
            }
        ))
    );
    assert_eq!(
        graph.create_link_request(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(6)),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
        ),
        Err(NodeGraphLinkEditRequestError::IncompatiblePort {
            from: PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(6)),
            to: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(3)),
            error: PortCompatibilityError::TypeMismatch {
                output: PortTypeId::from_raw(11),
                input: PortTypeId::from_raw(10),
            },
        })
    );
    assert_eq!(
        graph.reconnect_link_source_request(
            EdgeId::from_raw(50),
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(6)),
        ),
        Err(NodeGraphLinkEditRequestError::IncompatiblePort {
            from: PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(6)),
            to: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
            error: PortCompatibilityError::TypeMismatch {
                output: PortTypeId::from_raw(11),
                input: PortTypeId::from_raw(10),
            },
        })
    );
    assert_eq!(
        graph.reconnect_link_target_request(
            EdgeId::from_raw(50),
            PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(8)),
        ),
        Err(NodeGraphLinkEditRequestError::Endpoint(
            NodeGraphLinkDraftEndpointError::DisabledPort {
                node: NodeId::from_raw(2),
                port: PortId::from_raw(8),
            }
        ))
    );
}
