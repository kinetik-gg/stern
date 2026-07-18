//! Pure public Graph workspace composition evidence.

use stern::core::{
    FrameContext, Modifiers, MouseButton, PhysicalSize, Point, PointerButtonState, PointerInput,
    ScaleFactor, SemanticRole, SemanticValue, Size, TimeInfo, UiInput, UiInputEvent, ViewportInfo,
    WidgetId,
};
use stern::widgets::node_graph::{NodeGraphSelectionTarget, NodeId};
use stern_demo::{DemoApp, DemoWorkspace, demo_context};

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
        Point::new(100.0, 300.0),
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
    select(&mut app, Point::new(100.0, 300.0), Modifiers::default());
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(1)]
    );

    select(
        &mut app,
        Point::new(440.0, 360.0),
        Modifiers::new(true, false, false, false),
    );
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(1), NodeId::from_raw(2)]
    );
    select(
        &mut app,
        Point::new(100.0, 300.0),
        Modifiers::new(false, true, false, false),
    );
    assert_eq!(
        app.graph_workspace().selection().selected_nodes(),
        [NodeId::from_raw(2)]
    );
    select(&mut app, Point::new(300.0, 430.0), Modifiers::default());
    assert!(app.graph_workspace().selection().is_empty());
}

#[test]
fn graph_inspector_values_follow_public_node_selection() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);

    let source = app.frame(demo_context(graph_click(
        Point::new(100.0, 300.0),
        Modifiers::default(),
    )));
    assert_eq!(
        inspector_text_values(&source),
        ["Image Source", "36", "28", "1"]
    );
    assert!(has_inspector_rows(&source));

    let viewer = app.frame(demo_context(graph_click(
        Point::new(440.0, 360.0),
        Modifiers::default(),
    )));
    assert_eq!(inspector_text_values(&viewer), ["Viewer", "360", "88", "1"]);
    assert!(has_inspector_rows(&viewer));
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
        Point::new(100.0, 300.0),
        Modifiers::default(),
    )));
    let ids = graph_ids(&first);
    let expected_dock_ids = dock_ids(&first);
    let expected_inspector_ids = inspector_ids(&first);
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
    assert!(
        app.graph_workspace()
            .selection()
            .contains(NodeGraphSelectionTarget::Node(NodeId::from_raw(1)))
    );
}

#[test]
fn graph_workspace_reports_exact_four_runtime_component_ids() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, Point::new(100.0, 300.0), Modifiers::default());
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
    assert_eq!(
        qualified,
        [
            "dock",
            "node-graph",
            "node-components",
            "inspector-components"
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
    assert_eq!(uncovered.len(), 30);
    assert!(!uncovered.contains(&"dock"));
    assert!(!uncovered.contains(&"node-graph"));
    assert!(!uncovered.contains(&"node-components"));
    assert!(!uncovered.contains(&"inspector-components"));
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
