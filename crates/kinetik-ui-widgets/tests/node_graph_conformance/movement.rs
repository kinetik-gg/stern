#[allow(clippy::wildcard_imports)]
use super::common::*;

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
        NodeGraphSelectionTarget::Port(PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(4))),
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

    let request = NodeGraphCanvasPanRequest::new(selection.clone(), GraphVector::new(12.0, -8.0));

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

    let request =
        NodeGraphSelectedNodeMoveRequest::new(viewport, selection, GraphVector::new(500.0, -360.0));

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
