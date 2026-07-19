//! Pure public Graph workspace composition evidence.

use stern::core::{
    FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalSize, Point,
    PointerButtonState, PointerInput, ScaleFactor, SemanticRole, SemanticValue, Size, TimeInfo,
    UiInput, UiInputEvent, Vec2, ViewportInfo, WidgetId,
};
use stern::widgets::node_graph::{
    EdgeId, NodeGraphConnectionCancelReason, NodeGraphSelectionTarget, NodeId, PortEndpoint, PortId,
};
use stern_demo::{DemoApp, DemoWorkspace, GraphConnectionFeedback, demo_context};

const SOURCE_POINT: Point = Point::new(100.0, 370.0);
const SOURCE_PORT_POINT: Point = Point::new(216.0, 390.0);
const VIEWER_POINT: Point = Point::new(440.0, 350.0);
const CANVAS_POINT: Point = Point::new(300.0, 350.0);
const CLEAR_SELECTION_POINT: Point = Point::new(70.0, 244.0);

#[test]
fn graph_workspace_composes_public_retained_node_graph() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let output = app.frame(demo_context(UiInput::default()));

    for role in ["node-graph", "node", "port", "edge"] {
        assert!(has_role(&output, role), "missing public graph role: {role}");
    }
    let root = output
        .semantics
        .get(app.graph_workspace().root_id())
        .expect("public retained graph root");
    assert!(root.focusable);
    assert_eq!(root.label.as_deref(), Some("Node graph"));
    assert!(has_role(&output, "node-graph"));
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Grid && node.label.as_deref() == Some("Property grid")
    }));
}

#[test]
fn graph_workspace_composes_public_dock_panels() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let output = app.frame(demo_context(graph_click(
        SOURCE_POINT,
        Modifiers::default(),
    )));
    let semantics = output.semantics.nodes();
    let dock = semantics
        .iter()
        .find(|node| node.role == SemanticRole::Dock)
        .expect("public Dock semantic root");
    assert_eq!(dock.label.as_deref(), Some("Editor dock"));
    assert_eq!(dock.children.len(), 2);
    assert!(dock.children.iter().all(|id| {
        output
            .semantics
            .get(*id)
            .is_some_and(|node| node.role == SemanticRole::Frame)
    }));

    let graph_panel = panel(&output, "Graph");
    let inspector_panel = panel(&output, "Inspector");
    let graph = output
        .semantics
        .get(app.graph_workspace().root_id())
        .expect("docked graph semantic");
    let inspector = semantics
        .iter()
        .find(|node| node.role == SemanticRole::Grid)
        .expect("docked inspector grid semantic");
    assert_eq!(
        graph_panel.bounds.intersection(graph.bounds),
        Some(graph.bounds)
    );
    assert_eq!(
        inspector_panel.bounds.intersection(inspector.bounds),
        Some(inspector.bounds)
    );
    assert_eq!(
        inspector_text_values(&output),
        ["Image Source", "36", "28", "1"]
    );
}

#[test]
fn graph_pointer_selection_updates_application_owned_state() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, SOURCE_POINT, Modifiers::default());
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(1)]
    );

    select(
        &mut app,
        VIEWER_POINT,
        Modifiers::new(true, false, false, false),
    );
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(1), NodeId::from_raw(2)]
    );
    select(
        &mut app,
        SOURCE_POINT,
        Modifiers::new(false, true, false, false),
    );
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(2)]
    );
    select(&mut app, CANVAS_POINT, Modifiers::default());
    assert!(app.graph_workspace().selection().is_empty());
}

#[test]
fn graph_inspector_values_follow_public_node_selection() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);

    let source = app.frame(demo_context(graph_click(
        SOURCE_POINT,
        Modifiers::default(),
    )));
    assert_eq!(
        inspector_text_values(&source),
        ["Image Source", "36", "28", "1"]
    );
    assert!(has_inspector_rows(&source));

    let viewer = app.frame(demo_context(graph_click(
        VIEWER_POINT,
        Modifiers::default(),
    )));
    assert_eq!(inspector_text_values(&viewer), ["Viewer", "360", "28", "3"]);
    assert!(has_inspector_rows(&viewer));
}

#[test]
fn graph_connection_edit_qualifies_components_and_commits_one_app_owned_edge() {
    let mut app = focused_graph_app();
    let initial = app.frame(demo_context(UiInput::default()));
    let mut qualified = Vec::new();
    if has_role(&initial, "node-graph") {
        qualified.push("node-graph");
    }
    if ["node", "port", "edge"]
        .into_iter()
        .all(|role| has_role(&initial, role))
    {
        qualified.push("node-components");
    }
    assert_eq!(qualified, ["node-graph", "node-components"]);

    let source = graph_port_center(&initial, "Output Image");
    let target = graph_port_center(&initial, "Input Preview Image");
    let original_edges = app.graph_workspace().edges().to_vec();

    let _ = app.frame(demo_context(connection_press(source)));
    let _ = app.frame(demo_context(connection_move(source, target)));
    assert!(app.graph_workspace().connection_active());
    assert_eq!(
        app.graph_workspace().connection_feedback(),
        GraphConnectionFeedback::Previewing
    );
    assert_eq!(app.graph_workspace().edges(), original_edges);

    let _ = app.frame(demo_context(connection_release(target)));
    assert!(!app.graph_workspace().connection_active());
    assert_eq!(
        app.graph_workspace().connection_feedback(),
        GraphConnectionFeedback::Committed(EdgeId::from_raw(2))
    );
    assert_eq!(
        app.graph_workspace().edges().len(),
        original_edges.len() + 1
    );
    let committed = app
        .graph_workspace()
        .edges()
        .last()
        .expect("committed edge");
    assert_eq!(committed.id, EdgeId::from_raw(2));
    assert_eq!(
        committed.from,
        PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1))
    );
    assert_eq!(
        committed.to,
        PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2))
    );
    let feedback = app.frame(demo_context(UiInput::default()));
    assert_eq!(status_text(&feedback), "Connection committed as edge 2");
}

#[test]
fn graph_connection_rejects_incompatible_target_without_mutation() {
    let mut app = focused_graph_app();
    let initial = app.frame(demo_context(UiInput::default()));
    let source = graph_port_center(&initial, "Output Image");
    let incompatible = graph_port_center(&initial, "Input Vector Mask");
    let original_edges = app.graph_workspace().edges().to_vec();

    let _ = app.frame(demo_context(connection_press(source)));
    let _ = app.frame(demo_context(connection_move(source, incompatible)));
    assert_eq!(
        app.graph_workspace().connection_feedback(),
        GraphConnectionFeedback::Rejected
    );
    assert_eq!(app.graph_workspace().edges(), original_edges);

    let feedback = app.frame(demo_context(connection_release(incompatible)));
    assert!(!app.graph_workspace().connection_active());
    assert_eq!(app.graph_workspace().edges(), original_edges);
    assert_eq!(status_text(&feedback), "Incompatible connection rejected");
    assert_eq!(app.focused(), Some(app.graph_workspace().root_id()));
}

#[test]
fn graph_connection_escape_and_capture_loss_restore_focus_and_ownership() {
    for (cancel, reason) in [
        (
            connection_escape as fn(Point) -> UiInput,
            NodeGraphConnectionCancelReason::Escape,
        ),
        (
            connection_capture_loss as fn(Point) -> UiInput,
            NodeGraphConnectionCancelReason::CaptureLost,
        ),
    ] {
        let mut app = focused_graph_app();
        let initial = app.frame(demo_context(UiInput::default()));
        let source = graph_port_center(&initial, "Output Image");
        let preview = Point::new(source.x + 40.0, source.y + 20.0);
        let original_edges = app.graph_workspace().edges().to_vec();

        let _ = app.frame(demo_context(connection_press(source)));
        let _ = app.frame(demo_context(connection_move(source, preview)));
        assert!(app.graph_workspace().connection_active());
        let _ = app.frame(demo_context(cancel(preview)));

        assert_eq!(
            app.graph_workspace().connection_feedback(),
            GraphConnectionFeedback::Cancelled(reason)
        );
        assert!(!app.graph_workspace().connection_active());
        assert_eq!(app.graph_workspace().edges(), original_edges);
        assert_eq!(app.focused(), Some(app.graph_workspace().root_id()));
    }
}

#[test]
fn graph_inspector_empty_selection_is_an_empty_public_grid() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let output = app.frame(demo_context(UiInput::default()));

    assert!(app.graph_workspace().selection().is_empty());
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Grid && node.label.as_deref() == Some("Property grid")
    }));
    assert!(inspector_text_values(&output).is_empty());
    assert!(!has_inspector_rows(&output));
}

#[test]
fn graph_selection_and_semantic_ids_survive_workspace_round_trip() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let first = app.frame(demo_context(graph_click(
        SOURCE_POINT,
        Modifiers::default(),
    )));
    let ids = graph_ids(&first);
    let expected_dock_ids = dock_ids(&first);
    let expected_inspector_ids = inspector_ids(&first);
    let expected_chrome_ids = chrome_ids(&first);
    assert_eq!(app.focused(), Some(app.graph_workspace().root_id()));

    activate_workspace(&mut app, Point::new(60.0, 70.0), DemoWorkspace::Edit);
    assert_eq!(
        app.focused(),
        Some(WidgetId::from_key("root").child("workspace.edit"))
    );
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let graph_action = WidgetId::from_key("root").child("workspace.graph");
    assert_eq!(app.focused(), Some(graph_action));
    let resized = app.frame(resized_context(UiInput::default()));
    assert_eq!(app.focused(), Some(graph_action));
    assert_eq!(graph_ids(&resized), ids);
    assert_eq!(dock_ids(&resized), expected_dock_ids);
    assert_eq!(inspector_ids(&resized), expected_inspector_ids);
    assert_eq!(chrome_ids(&resized), expected_chrome_ids);
    assert!(
        app.graph_workspace()
            .selection()
            .contains(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
    );
}

#[test]
fn graph_workspace_reports_exact_seven_runtime_component_ids() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, SOURCE_POINT, Modifiers::default());
    let output = app.frame(demo_context(UiInput::default()));
    let mut qualified = Vec::new();
    if has_public_dock(&output) {
        qualified.push("dock");
    }
    if has_role(&output, "node-graph") {
        qualified.push("node-graph");
    }
    if ["node", "port", "edge"]
        .into_iter()
        .all(|role| has_role(&output, role))
    {
        qualified.push("node-components");
    }
    if has_inspector_rows(&output) {
        qualified.push("inspector-components");
    }
    if has_workspace_chrome(&output) {
        qualified.push("workspace-chrome");
    }
    if has_public_toolbar(&output) && has_action_semantic(&output, "graph.clear-selection") {
        qualified.push("toolbar-components");
    }
    if has_public_navigation(&output) {
        qualified.push("navigation-surface-components");
    }
    assert_eq!(
        qualified,
        [
            "dock",
            "node-graph",
            "node-components",
            "inspector-components",
            "workspace-chrome",
            "toolbar-components",
            "navigation-surface-components"
        ]
    );

    let required = concat!(
        "button,text-field,dropdown,selection-controls,value-controls,progress-feedback,",
        "overlay-system,virtual-list,editor-frame,workspace-chrome,dock,inspector-collections,",
        "node-graph,timeline,viewport,color-picker,gradient-editor,content-structure-components,",
        "icon-shortcut-components,toolbar-components,menu-components,command-palette-components,",
        "advanced-editor-fields,choice-value-components,feedback-status-components,",
        "overlay-components,navigation-surface-components,collection-components,",
        "inspector-components,editor-chrome-components,color-components,timeline-components,",
        "node-components,viewport-components",
    );
    let uncovered = required
        .split(',')
        .filter(|id| !qualified.contains(id))
        .collect::<Vec<_>>();
    assert_eq!(uncovered.len(), 27);
    for id in qualified {
        assert!(!uncovered.contains(&id));
    }
}

#[test]
fn graph_workspace_composes_public_chrome_above_dock() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let output = app.frame(demo_context(UiInput::default()));

    assert!(has_workspace_chrome(&output));
    let dock_index = semantic_index(&output, |node| node.role == SemanticRole::Dock);
    for label in ["Application toolbar", "Document tabs", "Application status"] {
        assert!(semantic_index(&output, |node| node.label.as_deref() == Some(label)) > dock_index);
    }
    let clear = clear_node(&output);
    assert!(clear.state.disabled);
    assert_eq!(status_text(&output), "0 selected");
    let _ = app.frame(demo_context(pointer_input(
        CLEAR_SELECTION_POINT,
        true,
        true,
        false,
    )));
    let disabled = app.frame(demo_context(pointer_input(
        CLEAR_SELECTION_POINT,
        false,
        false,
        true,
    )));
    assert!(disabled.actions.is_empty());
    assert!(app.graph_workspace().selection().is_empty());
}

#[test]
fn graph_toolbar_clear_selection_routes_once_and_updates_next_frame() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, SOURCE_PORT_POINT, Modifiers::default());
    assert!(matches!(
        app.graph_workspace().selection().selected().as_slice(),
        [NodeGraphSelectionTarget::Port(_)]
    ));
    let selected = app.frame(demo_context(UiInput::default()));
    assert!(!clear_node(&selected).state.disabled);
    assert!(has_action_semantic(&selected, "graph.clear-selection"));
    assert_eq!(status_text(&selected), "1 selected");

    let _ = app.frame(demo_context(pointer_input(
        CLEAR_SELECTION_POINT,
        true,
        true,
        false,
    )));
    let activated = app.frame(demo_context(pointer_input(
        CLEAR_SELECTION_POINT,
        false,
        false,
        true,
    )));
    assert_eq!(activated.actions.len(), 1);
    assert_eq!(
        activated
            .actions
            .clone()
            .pop_front()
            .unwrap()
            .action_id
            .as_str(),
        "graph.clear-selection"
    );
    assert!(app.graph_workspace().selection().is_empty());

    let next = app.frame(demo_context(UiInput::default()));
    assert!(clear_node(&next).state.disabled);
    assert!(inspector_text_values(&next).is_empty());
    assert_eq!(status_text(&next), "0 selected");
}

#[test]
fn graph_workspace_source_uses_only_public_stern_composition() {
    let source = rust_sources(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src"
    )));
    assert!(source.contains("use stern::"));
    assert!(source.contains("prepare_node_graph_widget"));
    assert!(source.contains("node_graph_widget"));
    assert!(source.contains("DockScene::new"));
    assert!(source.contains(".dock_scene("));
    assert!(source.contains("ChromeScene::new"));
    assert!(source.contains(".declare_pointer_targets"));
    assert!(source.contains(".chrome_scene("));
    let forbidden_source = concat!(
        "stern_core|stern_widgets|Primitive|SemanticNode|.emit(|push_primitive|push_semantic|",
        "rustfmt::skip|unsafe|include!(|#[path|mod widget|mod theme|fn paint_",
    );
    for forbidden in forbidden_source.split('|') {
        assert!(
            !source.contains(forbidden),
            "forbidden Graph source: {forbidden}"
        );
    }
}

fn has_public_dock(output: &stern::core::FrameOutput) -> bool {
    let semantics = output.semantics.nodes();
    semantics.iter().any(|node| node.role == SemanticRole::Dock)
        && semantics
            .iter()
            .filter(|node| node.role == SemanticRole::Frame)
            .count()
            == 2
        && ["Graph", "Inspector"].into_iter().all(|title| {
            semantics.iter().any(|node| {
                node.role == SemanticRole::Panel && node.label.as_deref() == Some(title)
            })
        })
}

fn has_workspace_chrome(output: &stern::core::FrameOutput) -> bool {
    has_public_toolbar(output)
        && has_public_navigation(output)
        && output.semantics.nodes().iter().any(|node| {
            matches!(&node.role, SemanticRole::Custom(role) if role == "status-bar")
                && node.children.iter().any(|id| {
                    output
                        .semantics
                        .get(*id)
                        .is_some_and(|item| item.label.as_deref() == Some(status_text(output)))
                })
        })
}

fn has_public_toolbar(output: &stern::core::FrameOutput) -> bool {
    output.semantics.nodes().iter().any(|node| {
        matches!(&node.role, SemanticRole::Custom(role) if role == "toolbar")
            && node.children.contains(&clear_node(output).id)
    })
}

fn has_action_semantic(output: &stern::core::FrameOutput, action_id: &str) -> bool {
    output.semantics.nodes().iter().any(|node| {
        node.actions.iter().any(|action| {
            action
                .action_id
                .as_ref()
                .is_some_and(|id| id.as_str() == action_id)
        })
    })
}

fn has_public_navigation(output: &stern::core::FrameOutput) -> bool {
    output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::TabList
            && node.children.iter().any(|id| {
                output.semantics.get(*id).is_some_and(|tab| {
                    tab.role == SemanticRole::Tab
                        && tab.label.as_deref() == Some("Graph")
                        && tab.state.selected
                })
            })
    })
}

fn clear_node(output: &stern::core::FrameOutput) -> &stern::core::SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Clear selection"))
        .expect("public clear-selection control")
}

fn status_text(output: &stern::core::FrameOutput) -> &str {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| matches!(&node.role, SemanticRole::Custom(role) if role == "status-bar"))
        .and_then(|surface| surface.children.first())
        .and_then(|id| output.semantics.get(*id))
        .and_then(|node| node.label.as_deref())
        .expect("public selection status")
}

fn semantic_index(
    output: &stern::core::FrameOutput,
    predicate: impl Fn(&stern::core::SemanticNode) -> bool,
) -> usize {
    output
        .semantics
        .nodes()
        .iter()
        .position(predicate)
        .expect("runtime semantic")
}

fn panel<'a>(output: &'a stern::core::FrameOutput, title: &str) -> &'a stern::core::SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Panel && node.label.as_deref() == Some(title))
        .expect("titled public Dock panel")
}

fn rust_sources(path: &std::path::Path) -> String {
    std::fs::read_dir(path)
        .expect("demo source directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                rust_sources(&path)
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                std::fs::read_to_string(path).expect("read demo Rust source")
            } else {
                String::new()
            }
        })
        .collect()
}

fn has_inspector_rows(output: &stern::core::FrameOutput) -> bool {
    let labels = output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::Row)
        .filter_map(|node| node.label.as_deref())
        .collect::<Vec<_>>();
    ["Title", "Position X", "Position Y", "Ports"]
        .into_iter()
        .all(|label| labels.contains(&label))
        && inspector_text_values(output).len() == 4
}

fn inspector_text_values(output: &stern::core::FrameOutput) -> Vec<&str> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::TextField)
        .filter_map(|node| match node.state.value.as_ref() {
            Some(SemanticValue::Text(value)) => Some(value.as_str()),
            _ => None,
        })
        .collect()
}

fn focused_graph_app() -> DemoApp {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, SOURCE_POINT, Modifiers::default());
    assert_eq!(app.focused(), Some(app.graph_workspace().root_id()));
    app
}

fn graph_port_center(output: &stern::core::FrameOutput, label: &str) -> Point {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            matches!(&node.role, SemanticRole::Custom(role) if role == "port")
                && node.label.as_deref() == Some(label)
        })
        .map_or_else(
            || panic!("public graph port: {label}"),
            |node| node.bounds.center(),
        )
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

fn connection_release(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn connection_escape(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input.pointer.position = Some(point);
    input
}

fn connection_capture_loss(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::WindowFocusChanged(false));
    input.pointer.position = Some(point);
    input
}

fn activate_workspace(app: &mut DemoApp, point: Point, expected: DemoWorkspace) {
    let _ = app.frame(demo_context(pointer_input(point, true, true, false)));
    let _ = app.frame(demo_context(pointer_input(point, false, false, true)));
    assert_eq!(app.workspace(), expected);
}

fn graph_click(point: Point, modifiers: Modifiers) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input.push_event(UiInputEvent::ModifiersChanged(modifiers));
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn select(app: &mut DemoApp, point: Point, modifiers: Modifiers) {
    let _ = app.frame(demo_context(graph_click(point, modifiers)));
}

fn pointer_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn has_role(output: &stern::core::FrameOutput, expected: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(&node.role, SemanticRole::Custom(role) if role == expected))
}

fn graph_ids(output: &stern::core::FrameOutput) -> Vec<WidgetId> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| matches!(&node.role, SemanticRole::Custom(role) if ["node-graph", "node", "port", "edge"].contains(&role.as_str())))
        .map(|node| node.id)
        .collect()
}

fn inspector_ids(output: &stern::core::FrameOutput) -> Vec<WidgetId> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| {
            matches!(
                node.role,
                SemanticRole::Grid | SemanticRole::Row | SemanticRole::TextField
            )
        })
        .map(|node| node.id)
        .collect()
}

fn dock_ids(output: &stern::core::FrameOutput) -> Vec<WidgetId> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| {
            matches!(
                node.role,
                SemanticRole::Dock
                    | SemanticRole::Frame
                    | SemanticRole::Panel
                    | SemanticRole::TabList
                    | SemanticRole::Tab
            )
        })
        .map(|node| node.id)
        .collect()
}

fn chrome_ids(output: &stern::core::FrameOutput) -> Vec<WidgetId> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| {
            node.label.as_deref() == Some("Clear selection")
                || node.label.as_deref() == Some("Application toolbar")
                || node.label.as_deref() == Some("Application status")
                || (node.role == SemanticRole::TabList
                    && node.label.as_deref() == Some("Document tabs"))
                || (node.role == SemanticRole::Tab && node.label.as_deref() == Some("Graph"))
        })
        .map(|node| node.id)
        .collect()
}

fn resized_context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(840.0, 560.0),
            PhysicalSize::new(1260, 840),
            ScaleFactor::new(1.5),
        ),
        input,
        TimeInfo::default(),
    )
}
