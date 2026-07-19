//! Public-facade contract for the scenario-gated Graph reorder journey.

use stern::core::{
    ActionSource, FrameOutput, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point,
    PointerButtonState, PointerInput, SemanticRole, UiInput, UiInputEvent, Vec2,
};
use stern::widgets::node_graph::{
    NodeGraphConnectionCancelReason, NodeGraphSelectionTarget, NodeId, PortEndpoint, PortId,
};
use stern_demo::{DemoApp, DemoScenario, DemoWorkspace, GraphConnectionFeedback, demo_context};

const REVERSE_NODE_ORDER_ACTION: &str = "graph.reverse-node-order";

#[test]
fn default_scenario_omits_reorder_action_and_preserves_pinned_output() {
    const BASE_FRAME_FINGERPRINT: u64 = 0x2b26_63ad_1046_fead;
    let mut maintained = DemoApp::new();
    let mut explicit = DemoApp::for_scenario(DemoScenario::Default);

    for _ in 0..2 {
        let maintained = maintained.frame(demo_context(UiInput::default()));
        let explicit = explicit.frame(demo_context(UiInput::default()));
        assert_eq!(maintained, explicit);
        assert_eq!(frame_fingerprint(&maintained), BASE_FRAME_FINGERPRINT);
        assert!(!has_action(&maintained, REVERSE_NODE_ORDER_ACTION));
    }
}

#[test]
fn public_graph_action_reverses_only_presentation_order() {
    let mut app = DemoApp::for_scenario(DemoScenario::GraphJourney);
    let edit = app.frame(demo_context(UiInput::default()));
    let graph_action = semantic_center(&edit, "Graph Workspace");
    let _ = click(&mut app, graph_action);
    assert_eq!(app.workspace(), DemoWorkspace::Graph);

    let graph = app.frame(demo_context(UiInput::default()));
    let source_node = custom_center(&graph, "node", "Image Source");
    let _ = app.frame(demo_context(pointer_click(source_node)));
    assert_eq!(
        app.graph_workspace().selection().active(),
        Some(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
    );

    let graph = app.frame(demo_context(UiInput::default()));
    let source = custom_center(&graph, "port", "Output Image");
    let target = custom_center(&graph, "port", "Input Preview Image");
    let _ = app.frame(demo_context(connection_press(source)));
    let _ = app.frame(demo_context(connection_move(source, target)));
    assert_eq!(
        app.graph_workspace().connection_feedback(),
        GraphConnectionFeedback::Accepted {
            from: PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            to: PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
        }
    );
    let _ = app.frame(demo_context(connection_escape(target)));
    assert_eq!(
        app.graph_workspace().connection_feedback(),
        GraphConnectionFeedback::Cancelled(NodeGraphConnectionCancelReason::Escape)
    );

    let nodes_before = app.graph_workspace().nodes().to_vec();
    let edges_before = app.graph_workspace().edges().to_vec();
    let selection_before = app.graph_workspace().selection().clone();
    let transform_before = app.graph_workspace().pan_zoom();
    let feedback_before = app.graph_workspace().connection_feedback();
    let active_before = app.graph_workspace().connection_active();
    let start_before = app.graph_workspace().connection_start_endpoint();
    let semantic_ids_before = graph_semantic_ids(&graph);
    assert_eq!(
        nodes_before.iter().map(|node| node.id).collect::<Vec<_>>(),
        [NodeId::from_raw(1), NodeId::from_raw(2)]
    );
    assert_eq!(app.graph_workspace().node_order_revision(), 0);

    let graph = app.frame(demo_context(UiInput::default()));
    let reverse = semantic_center(&graph, "Reverse node order");
    let activated = click(&mut app, reverse);
    assert!(exact_action(
        &activated,
        ActionSource::Button,
        REVERSE_NODE_ORDER_ACTION
    ));

    let nodes_after = app.graph_workspace().nodes();
    assert_eq!(
        nodes_after.iter().map(|node| node.id).collect::<Vec<_>>(),
        [NodeId::from_raw(2), NodeId::from_raw(1)]
    );
    assert_eq!(app.graph_workspace().node_order_revision(), 1);
    for node in &nodes_before {
        assert_eq!(
            nodes_after.iter().find(|candidate| candidate.id == node.id),
            Some(node)
        );
    }
    assert_eq!(app.graph_workspace().edges(), edges_before);
    assert_eq!(app.graph_workspace().selection(), &selection_before);
    assert_eq!(app.graph_workspace().pan_zoom(), transform_before);
    assert_eq!(app.graph_workspace().connection_feedback(), feedback_before);
    assert_eq!(app.graph_workspace().connection_active(), active_before);
    assert_eq!(
        app.graph_workspace().connection_start_endpoint(),
        start_before
    );
    let reordered = app.frame(demo_context(UiInput::default()));
    assert_eq!(graph_semantic_ids(&reordered), semantic_ids_before);
}

fn semantic_center(output: &FrameOutput, label: &str) -> Point {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic control: {label}"))
        .bounds
        .center()
}

fn custom_center(output: &FrameOutput, role: &str, label: &str) -> Point {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.role == SemanticRole::Custom(role.to_owned())
                && node.label.as_deref() == Some(label)
        })
        .unwrap_or_else(|| panic!("semantic {role}: {label}"))
        .bounds
        .center()
}

fn has_action(output: &FrameOutput, action_id: &str) -> bool {
    output.semantics.nodes().iter().any(|node| {
        node.actions.iter().any(|action| {
            action
                .action_id
                .as_ref()
                .is_some_and(|id| id.as_str() == action_id)
        })
    })
}

fn exact_action(output: &FrameOutput, source: ActionSource, action_id: &str) -> bool {
    let mut actions = output.actions.clone();
    let actions = actions.drain().collect::<Vec<_>>();
    matches!(actions.as_slice(), [action]
        if action.action_id.as_str() == action_id
            && action.source == source
            && action.context == stern::core::ActionContext::Editor)
}

fn graph_semantic_ids(output: &FrameOutput) -> Vec<(String, String, stern::core::WidgetId)> {
    let mut identities = output
        .semantics
        .nodes()
        .iter()
        .filter_map(|node| match (&node.role, node.label.as_deref()) {
            (SemanticRole::Custom(role), Some(label)) if role == "node" || role == "port" => {
                Some((role.clone(), label.to_owned(), node.id))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    identities.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    identities
}

fn click(app: &mut DemoApp, point: Point) -> FrameOutput {
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pointer_click(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn connection_press(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn connection_move(from: Point, to: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: to,
        delta: Vec2::new(to.x - from.x, to.y - from.y),
    });
    input
}

fn connection_escape(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.pointer.position = Some(point);
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input
}

fn frame_fingerprint(output: &FrameOutput) -> u64 {
    let fields = format!(
        "{:?}",
        (
            &output.primitives,
            &output.semantics,
            &output.repaint,
            &output.actions,
            &output.platform_requests,
            &output.warnings,
        )
    );
    fields.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
