//! Retained node graph widget conformance.

use stern_core::{
    FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalSize, Point, Primitive,
    Rect, RepaintRequest, SemanticRole, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::Ui;
use stern_widgets::node_graph::{
    GraphPoint, GraphRect, GraphVector, NodeDescriptor, NodeFrameDescriptor, NodeFrameId,
    NodeGraphConnectionCancelReason, NodeGraphConnectionController, NodeGraphConnectionIntent,
    NodeGraphConnectionRejection, NodeGraphDescriptor, NodeGraphHitTarget, NodeGraphPanZoom,
    NodeGraphSelection, NodeGraphSelectionOperation, NodeGraphSelectionTarget, NodeGraphStaticView,
    NodeGraphViewport, NodeGraphWidgetConfig, NodeGraphWidgetIntent, NodeGraphWidgetOutput,
    NodeGroupDescriptor, NodeGroupId, NodeId, PortDescriptor, PortDirection, PortEndpoint, PortId,
    PortTypeId,
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

const SOURCE: NodeId = NodeId::from_raw(30);
const SOURCE_PORT: PortId = PortId::from_raw(31);
const TARGET: NodeId = NodeId::from_raw(40);
const COMPATIBLE_PORT: PortId = PortId::from_raw(41);
const INCOMPATIBLE_PORT: PortId = PortId::from_raw(42);

fn connection_graph() -> NodeGraphDescriptor {
    let number = PortTypeId::from_raw(1);
    let vector = PortTypeId::from_raw(2);
    let mut graph = NodeGraphDescriptor::new();
    graph.nodes.push(
        NodeDescriptor::new(SOURCE, "Source", GraphRect::new(40.0, 40.0, 100.0, 80.0)).with_ports(
            vec![PortDescriptor::new(
                SOURCE_PORT,
                PortDirection::Output,
                "Number",
                number,
            )],
        ),
    );
    graph.nodes.push(
        NodeDescriptor::new(TARGET, "Target", GraphRect::new(220.0, 40.0, 100.0, 80.0)).with_ports(
            vec![
                PortDescriptor::new(COMPATIBLE_PORT, PortDirection::Input, "Number", number),
                PortDescriptor::new(INCOMPATIBLE_PORT, PortDirection::Input, "Vector", vector),
            ],
        ),
    );
    graph
}

fn connection_input(event: UiInputEvent, pointer_down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = pointer_down;
    input.push_event(event);
    input
}

fn connection_press(point: Point) -> UiInput {
    connection_input(
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(point),
        },
        true,
    )
}

fn connection_move(point: Point, delta: Vec2) -> UiInput {
    connection_input(
        UiInputEvent::PointerMoved {
            position: point,
            delta,
        },
        true,
    )
}

fn connection_release(point: Point) -> UiInput {
    connection_input(
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            click_count: 1,
            position: Some(point),
        },
        false,
    )
}

fn connection_escape(point: Point) -> UiInput {
    let mut input = connection_input(
        UiInputEvent::Key(KeyEvent::new(
            Key::Escape,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )),
        true,
    );
    input.pointer.position = Some(point);
    input
}

fn connection_capture_loss(point: Point) -> UiInput {
    let mut input = connection_input(UiInputEvent::WindowFocusChanged(false), true);
    input.pointer.position = Some(point);
    input
}

fn run_connection(
    graph: &NodeGraphDescriptor,
    viewport: NodeGraphViewport,
    input: UiInput,
    disabled: bool,
    read_only: bool,
    memory: &mut UiMemory,
    controller: &mut NodeGraphConnectionController,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let view = NodeGraphStaticView::new(ROOT, viewport, graph);
    let widget = ui
        .prepare_node_graph_widget(
            NodeGraphWidgetConfig::new(view)
                .disabled(disabled)
                .read_only(read_only)
                .with_hit_test(
                    stern_widgets::node_graph::NodeGraphHitTestConfig::new().with_port_size(24.0),
                ),
        )
        .expect("valid prepared graph");
    let output = ui
        .node_graph_widget_with_connections(&widget, controller)
        .expect("valid connection targeting");
    let frame = ui.finish_output();
    Run { output, frame }
}

fn connection_intents(output: &NodeGraphWidgetOutput) -> &[NodeGraphConnectionIntent] {
    &output.connection_intents
}

#[test]
fn connection_widget_emits_begin_preview_accept_and_commit_with_frozen_identity() {
    let graph = connection_graph();
    let initial_viewport = viewport();
    let changed_viewport = NodeGraphViewport::new(
        initial_viewport.bounds,
        NodeGraphPanZoom::new(GraphVector::new(60.0, 20.0), 1.75),
    );
    let source = PortEndpoint::new(SOURCE, SOURCE_PORT);
    let target = PortEndpoint::new(TARGET, COMPATIBLE_PORT);
    let source_point = initial_viewport.graph_to_screen(GraphPoint::new(140.0, 80.0));
    let target_point = initial_viewport.graph_to_screen(GraphPoint::new(220.0, 66.666_664));
    let mut memory = UiMemory::new();
    let mut controller = NodeGraphConnectionController::default();

    let pressed = run_connection(
        &graph,
        initial_viewport,
        connection_press(source_point),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    assert!(connection_intents(&pressed.output).is_empty());
    assert_eq!(controller.start_endpoint(), Some(source));
    assert_eq!(controller.frozen_viewport(), Some(initial_viewport));
    assert!(!controller.is_connecting());

    let moved = run_connection(
        &graph,
        changed_viewport,
        connection_move(target_point, Vec2::new(80.0, -13.333_336)),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    let moved = connection_intents(&moved.output);
    assert!(matches!(
        moved,
        [
            NodeGraphConnectionIntent::Begin(begin),
            NodeGraphConnectionIntent::Preview(preview),
            NodeGraphConnectionIntent::Accepted(accepted),
        ] if begin.graph == ROOT
            && begin.start.endpoint == source
            && begin.viewport == initial_viewport
            && preview.graph == ROOT
            && preview.viewport == initial_viewport
            && preview.draft.start.endpoint == source
            && preview.draft.target.hit_target() == NodeGraphHitTarget::Port(target)
            && accepted.from.endpoint == source
            && accepted.to.endpoint == target
    ));
    assert!(controller.is_connecting());
    assert_eq!(controller.frozen_viewport(), Some(initial_viewport));

    let released = run_connection(
        &graph,
        changed_viewport,
        connection_release(target_point),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    let released = connection_intents(&released.output);
    assert!(matches!(
        released,
        [
            NodeGraphConnectionIntent::Preview(_),
            NodeGraphConnectionIntent::Accepted(accepted),
            NodeGraphConnectionIntent::Commit(committed),
        ] if accepted == committed
            && committed.from.endpoint == source
            && committed.to.endpoint == target
    ));
    assert!(!controller.is_connecting());
    assert_eq!(controller.start_endpoint(), None);
    assert_eq!(released.len(), 3);
}

#[test]
fn connection_widget_rejects_incompatible_ports_without_commit() {
    let graph = connection_graph();
    let viewport = viewport();
    let source_point = viewport.graph_to_screen(GraphPoint::new(140.0, 80.0));
    let target_point = viewport.graph_to_screen(GraphPoint::new(220.0, 93.333_33));
    let mut memory = UiMemory::new();
    let mut controller = NodeGraphConnectionController::default();

    let _ = run_connection(
        &graph,
        viewport,
        connection_press(source_point),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    let moved = run_connection(
        &graph,
        viewport,
        connection_move(target_point, Vec2::new(80.0, 13.333_33)),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    assert!(connection_intents(&moved.output).iter().any(|intent| {
        matches!(
            intent,
            NodeGraphConnectionIntent::Rejected(NodeGraphConnectionRejection::Draft(_))
        )
    }));
    assert!(
        !connection_intents(&moved.output)
            .iter()
            .any(|intent| matches!(intent, NodeGraphConnectionIntent::Accepted(_)))
    );

    let released = run_connection(
        &graph,
        viewport,
        connection_release(target_point),
        false,
        false,
        &mut memory,
        &mut controller,
    );
    assert!(connection_intents(&released.output).iter().any(|intent| {
        matches!(
            intent,
            NodeGraphConnectionIntent::Rejected(NodeGraphConnectionRejection::Draft(_))
        )
    }));
    assert!(
        !connection_intents(&released.output)
            .iter()
            .any(|intent| matches!(intent, NodeGraphConnectionIntent::Commit(_)))
    );
}

#[test]
fn connection_widget_cancels_on_escape_and_capture_loss() {
    let graph = connection_graph();
    let viewport = viewport();
    let source_point = viewport.graph_to_screen(GraphPoint::new(140.0, 80.0));
    let preview_point = viewport.graph_to_screen(GraphPoint::new(180.0, 120.0));

    for (cancel_input, expected_reason) in [
        (
            connection_escape(preview_point),
            NodeGraphConnectionCancelReason::Escape,
        ),
        (
            connection_capture_loss(preview_point),
            NodeGraphConnectionCancelReason::CaptureLost,
        ),
    ] {
        let mut memory = UiMemory::new();
        let mut controller = NodeGraphConnectionController::default();
        let _ = run_connection(
            &graph,
            viewport,
            connection_press(source_point),
            false,
            false,
            &mut memory,
            &mut controller,
        );
        let _ = run_connection(
            &graph,
            viewport,
            connection_move(preview_point, Vec2::new(40.0, 40.0)),
            false,
            false,
            &mut memory,
            &mut controller,
        );
        let cancelled = run_connection(
            &graph,
            viewport,
            cancel_input,
            false,
            false,
            &mut memory,
            &mut controller,
        );
        assert!(matches!(
            connection_intents(&cancelled.output),
            [NodeGraphConnectionIntent::Cancel(cancel)]
                if cancel.reason == expected_reason
                    && cancel.graph == ROOT
                    && cancel.draft.start.endpoint == PortEndpoint::new(SOURCE, SOURCE_PORT)
                    && cancel.viewport == viewport
        ));
        assert!(!controller.is_connecting());
        assert_eq!(controller.start_endpoint(), None);
        assert_eq!(memory.pointer_capture(), None);
    }
}

#[test]
fn connection_widget_suppresses_and_cancels_disabled_or_read_only_mutation() {
    let graph = connection_graph();
    let viewport = viewport();
    let source_point = viewport.graph_to_screen(GraphPoint::new(140.0, 80.0));
    let preview_point = viewport.graph_to_screen(GraphPoint::new(180.0, 120.0));

    for (disabled, read_only) in [(true, false), (false, true)] {
        let mut memory = UiMemory::new();
        let mut controller = NodeGraphConnectionController::default();
        let pressed = run_connection(
            &graph,
            viewport,
            connection_press(source_point),
            disabled,
            read_only,
            &mut memory,
            &mut controller,
        );
        let moved = run_connection(
            &graph,
            viewport,
            connection_move(preview_point, Vec2::new(40.0, 40.0)),
            disabled,
            read_only,
            &mut memory,
            &mut controller,
        );
        assert!(connection_intents(&pressed.output).is_empty());
        assert!(connection_intents(&moved.output).is_empty());
        assert_eq!(controller.start_endpoint(), None);
    }

    for (disabled, read_only, reason) in [
        (true, false, NodeGraphConnectionCancelReason::Disabled),
        (false, true, NodeGraphConnectionCancelReason::ReadOnly),
    ] {
        let mut memory = UiMemory::new();
        let mut controller = NodeGraphConnectionController::default();
        let _ = run_connection(
            &graph,
            viewport,
            connection_press(source_point),
            false,
            false,
            &mut memory,
            &mut controller,
        );
        let _ = run_connection(
            &graph,
            viewport,
            connection_move(preview_point, Vec2::new(40.0, 40.0)),
            false,
            false,
            &mut memory,
            &mut controller,
        );
        let cancelled = run_connection(
            &graph,
            viewport,
            UiInput::default(),
            disabled,
            read_only,
            &mut memory,
            &mut controller,
        );
        assert!(matches!(
            connection_intents(&cancelled.output),
            [NodeGraphConnectionIntent::Cancel(cancel)] if cancel.reason == reason
        ));
        assert_eq!(controller.start_endpoint(), None);
        assert_eq!(memory.pointer_capture(), None);
    }
}
