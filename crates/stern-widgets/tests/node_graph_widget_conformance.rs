//! Retained node graph widget conformance.

use stern_core::{
    FrameContext, Modifiers, MouseButton, PhysicalSize, Point, Primitive, Rect, RepaintRequest,
    SemanticRole, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_widgets::Ui;
use stern_widgets::node_graph::{
    GraphPoint, GraphRect, GraphVector, NodeDescriptor, NodeFrameDescriptor, NodeFrameId,
    NodeGraphDescriptor, NodeGraphHitTarget, NodeGraphPanZoom, NodeGraphSelection,
    NodeGraphSelectionOperation, NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphViewport,
    NodeGraphWidgetConfig, NodeGraphWidgetIntent, NodeGraphWidgetOutput, NodeGroupDescriptor,
    NodeGroupId, NodeId, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
};

const ROOT: WidgetId = WidgetId::from_raw(700);
const NODE: NodeId = NodeId::from_raw(10);
const PORT: PortId = PortId::from_raw(20);

#[rustfmt::skip]
fn graph() -> NodeGraphDescriptor {
    let mut graph = NodeGraphDescriptor::new();
    graph.nodes.push(NodeDescriptor::new(NODE, "Mix", GraphRect::new(40.0, 40.0, 100.0, 80.0))
        .with_ports(vec![PortDescriptor::new(PORT, PortDirection::Input, "Input", PortTypeId::from_raw(1))]));
    graph
}

#[rustfmt::skip]
fn viewport() -> NodeGraphViewport {
    NodeGraphViewport::new(Rect::new(0.0, 0.0, 400.0, 240.0), NodeGraphPanZoom::default())
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(400.0, 240.0),
            PhysicalSize::new(400, 240),
            stern_core::ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    )
}

fn click(position: Point, release_modifiers: Modifiers) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(position),
    });
    input.push_event(UiInputEvent::ModifiersChanged(release_modifiers));
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(position),
    });
    input
}

struct Run {
    output: NodeGraphWidgetOutput,
    frame: stern_core::FrameOutput,
}

fn run(
    graph: &NodeGraphDescriptor,
    viewport: NodeGraphViewport,
    selection: NodeGraphSelection,
    input: UiInput,
    disabled: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(input), &mut memory, &theme);
    let view = NodeGraphStaticView::new(ROOT, viewport, graph).with_selection(selection);
    let widget = ui
        .prepare_node_graph_widget(NodeGraphWidgetConfig::new(view).disabled(disabled))
        .expect("valid prepared graph");
    let output = ui.node_graph_widget(&widget).expect("valid release hit");
    let frame = ui.finish_output();
    Run { output, frame }
}

fn assert_selection(
    point: Point,
    modifiers: Modifiers,
    hit: NodeGraphHitTarget,
    operation: NodeGraphSelectionOperation,
) {
    let run = run(
        &graph(),
        viewport(),
        NodeGraphSelection::new(),
        click(point, modifiers),
        false,
    );
    assert_eq!(run.output.hit, Some(hit));
    assert_eq!(
        run.output.intents,
        vec![NodeGraphWidgetIntent::Selection(operation)]
    );
    assert_eq!(run.frame.repaint, RepaintRequest::NextFrame);
}

#[test]
fn node_graph_widget_composes_static_view_and_focusable_root() {
    let graph = graph();
    let selection = NodeGraphSelection::from_targets([NodeGraphSelectionTarget::Node(NODE)]);
    let run = run(
        &graph,
        viewport(),
        selection.clone(),
        UiInput::default(),
        false,
    );

    assert!(!run.frame.primitives.is_empty());
    assert_eq!(
        run.frame
            .semantics
            .nodes()
            .iter()
            .filter(|node| node.focusable)
            .count(),
        1
    );
    assert!(run.frame.semantics.get(ROOT).expect("root").focusable);
    assert!(run.output.response.state.selected);
    assert!(run.output.intents.is_empty());
    assert_eq!(
        selection.selected(),
        vec![NodeGraphSelectionTarget::Node(NODE)]
    );
}

#[test]
fn node_graph_widget_emits_typed_node_port_and_canvas_selection() {
    let endpoint = PortEndpoint::new(NODE, PORT);
    #[rustfmt::skip]
    let cases = [
        (Point::new(80.0, 100.0), NodeGraphHitTarget::NodeBody(NODE), NodeGraphSelectionOperation::Replace(NodeGraphSelectionTarget::Node(NODE))),
        (Point::new(40.0, 80.0), NodeGraphHitTarget::Port(endpoint), NodeGraphSelectionOperation::Replace(NodeGraphSelectionTarget::Port(endpoint))),
        (Point::new(300.0, 200.0), NodeGraphHitTarget::Canvas, NodeGraphSelectionOperation::Clear),
    ];
    for (point, hit, operation) in cases {
        assert_selection(point, Modifiers::default(), hit, operation);
    }
}

#[test]
fn node_graph_widget_preserves_event_time_extend_and_toggle_modifiers() {
    let target = NodeGraphSelectionTarget::Node(NODE);
    #[rustfmt::skip]
    let cases = [
        (Modifiers::new(true, false, false, false), NodeGraphSelectionOperation::Extend(target)),
        (Modifiers::new(false, true, false, false), NodeGraphSelectionOperation::Toggle(target)),
        (Modifiers::new(false, false, false, true), NodeGraphSelectionOperation::Toggle(target)),
    ];
    for (modifiers, operation) in cases {
        assert_selection(
            Point::new(80.0, 100.0),
            modifiers,
            NodeGraphHitTarget::NodeBody(NODE),
            operation,
        );
    }
}

#[test]
fn node_graph_widget_ignores_disabled_and_nonselectable_frame_hits() {
    let disabled = run(
        &graph(),
        viewport(),
        NodeGraphSelection::new(),
        click(Point::new(80.0, 100.0), Modifiers::default()),
        true,
    );
    assert!(disabled.output.response.state.disabled);
    assert!(disabled.output.intents.is_empty());
    assert_eq!(disabled.frame.repaint, RepaintRequest::None);
    assert!(!disabled.frame.semantics.get(ROOT).expect("root").focusable);

    let mut surfaces = graph();
    surfaces.frames.push(NodeFrameDescriptor::new(
        NodeFrameId::from_raw(1),
        "Frame",
        GraphRect::new(180.0, 20.0, 60.0, 60.0),
    ));
    surfaces.groups.push(NodeGroupDescriptor::new(
        NodeGroupId::from_raw(2),
        "Group",
        GraphRect::new(280.0, 20.0, 60.0, 60.0),
    ));
    #[rustfmt::skip]
    let cases = [
        (Point::new(200.0, 40.0), NodeGraphHitTarget::Frame(NodeFrameId::from_raw(1))),
        (Point::new(300.0, 40.0), NodeGraphHitTarget::Group(NodeGroupId::from_raw(2))),
    ];
    for (point, hit) in cases {
        let run = run(
            &surfaces,
            viewport(),
            NodeGraphSelection::new(),
            click(point, Modifiers::default()),
            false,
        );
        assert_eq!(run.output.hit, Some(hit));
        assert!(run.output.intents.is_empty());
        assert_eq!(run.frame.repaint, RepaintRequest::None);
    }
}

#[test]
fn node_graph_widget_shares_viewport_transform_for_paint_hit_and_semantics() {
    let graph = graph();
    let viewport = NodeGraphViewport::new(
        Rect::new(100.0, 50.0, 400.0, 300.0),
        NodeGraphPanZoom::new(GraphVector::new(10.0, -5.0), 2.0),
    );
    let expected = viewport.graph_rect_to_screen(graph.nodes[0].rect);
    let run = run(
        &graph,
        viewport,
        NodeGraphSelection::new(),
        click(
            viewport.graph_to_screen(GraphPoint::new(80.0, 100.0)),
            Modifiers::default(),
        ),
        false,
    );

    assert!(
        run.frame.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(rect) if rect.rect == expected)
        })
    );
    assert!(run.frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Custom("node".to_owned()) && node.bounds == expected
    }));
    assert_eq!(run.output.hit, Some(NodeGraphHitTarget::NodeBody(NODE)));
}
