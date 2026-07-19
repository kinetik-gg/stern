//! Pure public Graph workspace composition evidence.

use stern::core::{
    ActionSource, FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, PointerInput, ScaleFactor, SemanticRole,
    SemanticValue, Size, TimeInfo, UiInput, UiInputEvent, Vec2, ViewportInfo, WidgetId,
};
use stern::widgets::node_graph::{
    EdgeId, NodeGraphConnectionCancelReason, NodeGraphSelectionTarget, NodeId, PortEndpoint, PortId,
};
use stern_demo::{DemoApp, DemoViewportTool, DemoWorkspace, GraphConnectionFeedback, demo_context};

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
    assert_eq!(dock.children.len(), 3);
    assert!(dock.children.iter().all(|id| {
        output
            .semantics
            .get(*id)
            .is_some_and(|node| node.role == SemanticRole::Frame)
    }));

    let graph_panel = panel(&output, "Graph");
    let viewport_panel = panel(&output, "Viewport");
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
    let viewport = semantic_node(&output, &SemanticRole::Viewport, "Graph preview viewport");
    assert_eq!(
        viewport_panel.bounds.intersection(viewport.bounds),
        Some(viewport.bounds)
    );
    assert_eq!(
        inspector_text_values(&output),
        ["Image Source", "36", "28", "1"]
    );
}

#[test]
fn graph_viewport_projects_tool_action_and_retained_non_default_transform() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    let initial = app.frame(demo_context(UiInput::default()));

    let transform = app.graph_workspace().pan_zoom();
    assert_eq!(
        transform.pan,
        stern::widgets::node_graph::GraphVector::new(2.0, 2.0)
    );
    assert_eq!(transform.zoom.to_bits(), 1.0_f32.to_bits());
    semantic_node(&initial, &SemanticRole::Viewport, "Graph preview viewport");
    assert_eq!(app.viewport_tool(), DemoViewportTool::Select);

    let action = click_point(
        &mut app,
        semantic_center(&initial, &SemanticRole::Button, "Transform Tool"),
    );
    assert!(exact_action(
        &action,
        ActionSource::Button,
        "viewport.tool.transform",
    ));
    assert_eq!(app.viewport_tool(), DemoViewportTool::Transform);

    let transformed = app.frame(demo_context(UiInput::default()));
    assert_eq!(
        semantic_node(&transformed, &SemanticRole::Toggle, "Transform Tool")
            .state
            .checked,
        Some(true)
    );
    assert!(has_semantic_role(&transformed, &SemanticRole::Viewport));
}

#[test]
fn graph_menu_routes_once_and_escape_or_outside_press_restore_focus() {
    let mut app = focused_graph_app();
    let owner = app.focused();
    let initial = app.frame(demo_context(UiInput::default()));
    let trigger = semantic_center(&initial, &SemanticRole::MenuItem, "Workspace");

    let _ = click_point(&mut app, trigger);
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "Workspace commands"));
    let revision = app.applied_revision();
    let action = click_point(
        &mut app,
        semantic_center(&shown, &SemanticRole::MenuItem, "Apply Shared State"),
    );
    assert!(exact_action(&action, ActionSource::Menu, "shared.apply"));
    assert_eq!(app.applied_revision(), revision + 1);
    assert_eq!(app.focused(), owner);

    let closed = app.frame(demo_context(UiInput::default()));
    let _ = click_point(
        &mut app,
        semantic_center(&closed, &SemanticRole::MenuItem, "Workspace"),
    );
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "Workspace commands"));
    let _ = app.frame(demo_context(key_input(Key::Escape, Modifiers::default())));
    let closed = app.frame(demo_context(UiInput::default()));
    assert!(!has_label(&closed, "Workspace commands"));
    assert_eq!(app.focused(), owner);

    let _ = click_point(
        &mut app,
        semantic_center(&closed, &SemanticRole::MenuItem, "Workspace"),
    );
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "Workspace commands"));
    let outside = Point::new(700.0, 190.0);
    let _ = app.frame(demo_context(pointer_input(outside, true, true, false)));
    let closed = app.frame(demo_context(pointer_input(outside, false, false, true)));
    assert!(!has_label(&closed, "Workspace commands"));
    assert_eq!(app.focused(), owner);
}

#[test]
fn graph_command_palette_routes_once_and_escape_restores_focus() {
    let mut app = focused_graph_app();
    let owner = app.focused();
    let revision = app.applied_revision();

    let shown = app.frame(demo_context(key_input(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    assert!(has_semantic_role(&shown, &SemanticRole::SearchField));
    assert!(has_label(&shown, "Apply Shared State"));
    let action = app.frame(demo_context(key_input(Key::Enter, Modifiers::default())));
    assert!(exact_action(
        &action,
        ActionSource::CommandPalette,
        "shared.apply",
    ));
    assert_eq!(app.applied_revision(), revision + 1);
    assert_eq!(app.focused(), owner);

    let shown = app.frame(demo_context(key_input(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    assert!(has_semantic_role(&shown, &SemanticRole::SearchField));
    let _ = app.frame(demo_context(key_input(Key::Escape, Modifiers::default())));
    let closed = app.frame(demo_context(UiInput::default()));
    assert!(!has_semantic_role(&closed, &SemanticRole::SearchField));
    assert_eq!(app.focused(), owner);
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
fn graph_workspace_reports_exact_fourteen_runtime_component_ids() {
    let mut app = DemoApp::new();
    activate_workspace(&mut app, Point::new(180.0, 70.0), DemoWorkspace::Graph);
    select(&mut app, SOURCE_POINT, Modifiers::default());
    let output = app.frame(demo_context(UiInput::default()));
    let owner = app.focused();

    let menu_trigger = semantic_center(&output, &SemanticRole::MenuItem, "Workspace");
    let _ = click_point(&mut app, menu_trigger);
    let menu = app.frame(demo_context(UiInput::default()));
    let menu_projected = has_label(&menu, "Workspace commands")
        && has_semantic_role(&menu, &SemanticRole::Menu)
        && has_label(&menu, "Apply Shared State");
    let menu_apply = semantic_center(&menu, &SemanticRole::MenuItem, "Apply Shared State");
    let menu_action = click_point(&mut app, menu_apply);
    let menu_exact = exact_action(&menu_action, ActionSource::Menu, "shared.apply");
    let menu_focus = app.focused() == owner;

    let palette = app.frame(demo_context(key_input(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_projected = has_semantic_role(&palette, &SemanticRole::SearchField)
        && has_label(&palette, "Apply Shared State");
    let palette_action = app.frame(demo_context(key_input(Key::Enter, Modifiers::default())));
    let palette_exact = exact_action(
        &palette_action,
        ActionSource::CommandPalette,
        "shared.apply",
    );
    let palette_focus = app.focused() == owner;

    let mut qualified = Vec::new();
    if output.platform_requests.iter().any(
        |request| matches!(request, PlatformRequest::SetWindowTitle(title) if title == stern_demo::DEMO_TITLE),
    ) {
        qualified.push("editor-frame");
    }
    if has_workspace_chrome(&output) {
        qualified.push("workspace-chrome");
    }
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
    let viewport = has_semantic_role(&output, &SemanticRole::Viewport);
    if viewport {
        qualified.push("viewport");
    }
    if viewport && has_label(&output, "Select Tool") && has_label(&output, "Transform Tool") {
        qualified.push("viewport-components");
    }
    if has_inspector_rows(&output) {
        qualified.push("inspector-components");
    }
    if has_public_toolbar(&output) && has_action_semantic(&output, "graph.clear-selection") {
        qualified.push("toolbar-components");
    }
    if has_public_navigation(&output) {
        qualified.push("navigation-surface-components");
    }
    if menu_projected && menu_exact {
        qualified.push("menu-components");
    }
    if palette_projected && palette_exact {
        qualified.push("command-palette-components");
    }
    if menu_projected && palette_projected && menu_focus && palette_focus {
        qualified.push("overlay-system");
        qualified.push("overlay-components");
    }
    assert_eq!(
        qualified,
        [
            "editor-frame",
            "workspace-chrome",
            "dock",
            "node-graph",
            "node-components",
            "viewport",
            "viewport-components",
            "inspector-components",
            "toolbar-components",
            "navigation-surface-components",
            "menu-components",
            "command-palette-components",
            "overlay-system",
            "overlay-components",
        ]
    );
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
    assert!(source.contains("MenuBar::from_menus"));
    assert!(source.contains("prepare_viewport_widget"));
    assert!(source.contains(".viewport_widget("));
    assert!(source.contains("prepare_viewport_tool_scene"));
    assert!(source.contains(".viewport_tool_scene("));
    assert!(source.contains(".overlay_scene("));
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
            == 3
        && ["Graph", "Viewport", "Inspector"].into_iter().all(|title| {
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

fn click_point(app: &mut DemoApp, point: Point) -> stern::core::FrameOutput {
    let _ = app.frame(demo_context(pointer_input(point, true, true, false)));
    app.frame(demo_context(pointer_input(point, false, false, true)))
}

fn key_input(key: Key, modifiers: Modifiers) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        key,
        KeyState::Pressed,
        modifiers,
        false,
    )));
    input
}

fn semantic_node<'a>(
    output: &'a stern::core::FrameOutput,
    role: &SemanticRole,
    label: &str,
) -> &'a stern::core::SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .expect("semantic control")
}

fn semantic_center(output: &stern::core::FrameOutput, role: &SemanticRole, label: &str) -> Point {
    semantic_node(output, role, label).bounds.center()
}

fn has_label(output: &stern::core::FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn has_semantic_role(output: &stern::core::FrameOutput, role: &SemanticRole) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role)
}

fn exact_action(output: &stern::core::FrameOutput, source: ActionSource, action_id: &str) -> bool {
    let mut actions = output.actions.clone();
    let actions = actions.drain().collect::<Vec<_>>();
    matches!(actions.as_slice(), [action]
        if action.action_id.as_str() == action_id
            && action.source == source
            && action.context == stern::core::ActionContext::Editor)
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
