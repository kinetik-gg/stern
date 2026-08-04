//! Public-facade contract for the scenario-gated Graph reorder journey.

use std::collections::BTreeMap;

use stern::core::{
    ActionSource, FrameOutput, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point,
    PointerButtonState, PointerInput, Primitive, SemanticRole, UiInput, UiInputEvent, Vec2,
};
use stern::widgets::node_graph::{
    NodeGraphConnectionCancelReason, NodeGraphSelectionTarget, NodeId, PortEndpoint, PortId,
};
use stern_demo::{DemoApp, DemoScenario, DemoWorkspace, GraphConnectionFeedback, demo_context};

const REVERSE_NODE_ORDER_ACTION: &str = "graph.reverse-node-order";

#[test]
fn default_scenario_omits_reorder_action_and_preserves_pinned_output() {
    // Structural pin (#916): the opaque frame-fingerprint hash this replaced
    // broke on every paint-recipe change (color/radius/spacing token PRs)
    // even though nothing structural moved. These assertions instead pin
    // primitive counts by type, the exact semantic-node label set, and the
    // workspace-switch row's key geometry — they only break when a control
    // is genuinely added, removed, or reordered.
    let mut maintained = DemoApp::new();
    let mut explicit = DemoApp::for_scenario(DemoScenario::Default);

    for _ in 0..2 {
        let maintained = maintained.frame(demo_context(UiInput::default()));
        let explicit = explicit.frame(demo_context(UiInput::default()));
        assert_eq!(maintained, explicit);
        assert!(!has_action(&maintained, REVERSE_NODE_ORDER_ACTION));
        assert_default_base_frame_structure(&maintained);
    }
}

/// Asserts the deterministic Default-scenario base frame's primitive-type
/// counts, exact semantic-node label multiset, and workspace-switch row
/// geometry (shared with `overlay_recovery_journey_contract.rs`).
#[allow(clippy::too_many_lines)] // structural inventory reads better linear
fn assert_default_base_frame_structure(output: &FrameOutput) {
    let expected_counts: BTreeMap<&'static str, usize> = BTreeMap::from([
        ("clip_begin", 25),
        ("clip_end", 25),
        ("icon", 7),
        ("line", 1),
        ("rect", 123),
        ("text", 81),
        ("texture", 1),
        ("transform_begin", 2),
        ("transform_end", 2),
    ]);
    assert_eq!(primitive_counts(output), expected_counts);

    let expected_labels: Vec<&'static str> = vec![
        "Application menu",
        "Application status",
        "Application toolbar",
        "Applied revision 0",
        "Apply Shared State",
        "Assets",
        "Assets",
        "Assets",
        "Assets",
        "Backdrop",
        "Background jobs",
        "Bloom",
        "Character",
        "Clouds",
        "Color",
        "Credits",
        "Document tabs",
        "Edit Workspace",
        "Edit Workspace",
        "Editor dock",
        "Effects",
        "Fill color",
        "Foreground",
        "Frame tabs",
        "Frame tabs",
        "Frame tabs",
        "Frame tabs",
        "Gallery Workspace",
        "Gallery Workspace",
        "Grade",
        "Gradient editor",
        "Gradient stop 1",
        "Gradient stop 2",
        "Graph Workspace",
        "Graph Workspace",
        "Hero clip",
        "Inspector",
        "Inspector",
        "Inspector",
        "Kind",
        "Lighting",
        "Mountains",
        "Name",
        "Notifications",
        "Opacity",
        "Preview 40%",
        "Preview render",
        "Property grid",
        "Raster layer",
        "Reset Kind to default",
        "Reset Name to default",
        "Reset Opacity to default",
        "Reset Visible to default",
        "Save Color Style",
        "Select Tool",
        "Select Tool",
        "Selection",
        "Subtitle",
        "Text field",
        "Text field",
        "Timeline",
        "Timeline",
        "Timeline",
        "Timeline",
        "Title",
        "Transform Tool",
        "Transform Tool",
        "Video",
        "Viewport",
        "Viewport",
        "Viewport",
        "Viewport",
        "Visible",
        "Visible",
        "Workspace",
        "sRGB · Reverse",
    ];
    assert_eq!(
        semantic_labels(output),
        expected_labels
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );

    // Key geometry: the five workspace/shared-action toolbar buttons sit in
    // one row, left to right in registry order, sharing height and baseline.
    let edit = semantic_center_rect(output, "Edit Workspace");
    let graph = semantic_center_rect(output, "Graph Workspace");
    let gallery = semantic_center_rect(output, "Gallery Workspace");
    let apply = semantic_center_rect(output, "Apply Shared State");
    let save = semantic_center_rect(output, "Save Color Style");
    for rect in [&graph, &gallery, &apply, &save] {
        assert_eq!(rect.y.to_bits(), edit.y.to_bits());
        assert_eq!(rect.height.to_bits(), edit.height.to_bits());
    }
    assert!(edit.x < graph.x);
    assert!(graph.x < gallery.x);
    assert!(gallery.x < apply.x);
    assert!(apply.x < save.x);
}

fn primitive_kind(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::Rect(_) => "rect",
        Primitive::Line(_) => "line",
        Primitive::Shadow(_) => "shadow",
        Primitive::Path(_) => "path",
        Primitive::Icon(_) => "icon",
        Primitive::Text(_) => "text",
        Primitive::Image(_) => "image",
        Primitive::Texture(_) => "texture",
        Primitive::ClipBegin { .. } => "clip_begin",
        Primitive::ClipEnd { .. } => "clip_end",
        Primitive::LayerBegin { .. } => "layer_begin",
        Primitive::LayerEnd { .. } => "layer_end",
        Primitive::TransformBegin(_) => "transform_begin",
        Primitive::TransformEnd => "transform_end",
    }
}

fn primitive_counts(output: &FrameOutput) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for primitive in &output.primitives {
        *counts.entry(primitive_kind(primitive)).or_insert(0) += 1;
    }
    counts
}

fn semantic_labels(output: &FrameOutput) -> Vec<String> {
    let mut labels = output
        .semantics
        .nodes()
        .iter()
        .filter_map(|node| node.label.clone())
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

fn semantic_center_rect(output: &FrameOutput, label: &str) -> stern::core::Rect {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic control: {label}"))
        .bounds
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
