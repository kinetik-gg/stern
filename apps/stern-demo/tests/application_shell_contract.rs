//! Deterministic contracts for the shared public application bar adopted by
//! the Edit and Graph workspaces (issue #876).
//!
//! The demo owns one retained public `stern::widgets::ApplicationBar` whose
//! File/Edit/View/Window/Help headings are backed by the application action
//! registry and whose Edit and Graph workspace tabs activate through the
//! existing application-owned workspace actions. Every menu surface — F10
//! entry, top-level Left/Right traversal, pointer opening, adjacent
//! replacement, and Escape — flows through the demo's one shared overlay
//! route.

use stern::core::{
    ActionContext, ActionSource, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    Point, PointerButtonState, PointerInput, Rect, SemanticActionKind, SemanticRole, UiInput,
    UiInputEvent, WidgetId,
};
use stern_demo::{
    DemoActionAvailability, DemoApp, DemoWorkspace, application_bar_root, demo_context,
};

const FILE_MENU_RAW: u64 = 201;
const EDIT_MENU_RAW: u64 = 202;
const VIEW_MENU_RAW: u64 = 203;
const WINDOW_MENU_RAW: u64 = 204;
const HELP_MENU_RAW: u64 = 205;
const EDIT_TAB_RAW: u64 = 101;
const GRAPH_TAB_RAW: u64 = 102;

fn heading_widget_id(raw: u64) -> WidgetId {
    application_bar_root()
        .child("application-menu")
        .child(("menu", raw))
}

fn tab_widget_id(raw: u64) -> WidgetId {
    application_bar_root()
        .child("application-workspaces")
        .child(("workspace", raw))
}

#[test]
#[allow(clippy::too_many_lines)]
fn shell_bar_is_retained_with_deterministic_action_backed_menus_and_tabs() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let root = initial
        .semantics
        .get(application_bar_root())
        .expect("retained public application bar root");
    assert_eq!(
        (&root.role, root.label.as_deref()),
        (
            &SemanticRole::Custom("application-bar".to_owned()),
            Some("Application bar")
        )
    );
    assert_eq!(root.bounds, Rect::new(0.0, 0.0, 720.0, 40.0));

    // File, Edit, View, Window, Help in deterministic source order, each an
    // action-backed open target with stable widget identity.
    let menus = initial
        .semantics
        .get(root.children[0])
        .expect("application menu composite");
    let headings: Vec<(WidgetId, &str)> = [
        (FILE_MENU_RAW, "File"),
        (EDIT_MENU_RAW, "Edit"),
        (VIEW_MENU_RAW, "View"),
        (WINDOW_MENU_RAW, "Window"),
        (HELP_MENU_RAW, "Help"),
    ]
    .into_iter()
    .map(|(raw, label)| (heading_widget_id(raw), label))
    .collect();
    assert_eq!(
        menus.children,
        headings.iter().map(|(id, _)| *id).collect::<Vec<_>>()
    );
    // Roving focus: exactly one heading is focusable before any input.
    assert_eq!(headings_focusable_count(&initial), 1);
    for (label, raw) in [
        ("File", FILE_MENU_RAW),
        ("Edit", EDIT_MENU_RAW),
        ("View", VIEW_MENU_RAW),
        ("Window", WINDOW_MENU_RAW),
        ("Help", HELP_MENU_RAW),
    ] {
        let heading = initial
            .semantics
            .get(heading_widget_id(raw))
            .expect("heading");
        assert_eq!(heading.label.as_deref(), Some(label));
        assert_eq!(heading.role, SemanticRole::MenuItem);
        assert!(
            heading
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Open)
        );
        assert_eq!(heading.state.expanded, Some(false));
    }
    // Edit and Graph are the stable public workspace tabs; Edit starts active.
    let workspaces = initial
        .semantics
        .get(root.children[1])
        .expect("workspace tab composite");
    assert_eq!(workspaces.role, SemanticRole::TabList);
    assert_eq!(
        workspaces.children,
        vec![tab_widget_id(EDIT_TAB_RAW), tab_widget_id(GRAPH_TAB_RAW)]
    );
    assert!(
        initial
            .semantics
            .get(tab_widget_id(EDIT_TAB_RAW))
            .expect("Edit tab")
            .state
            .selected
    );
    assert!(
        !initial
            .semantics
            .get(tab_widget_id(GRAPH_TAB_RAW))
            .expect("Graph tab")
            .state
            .selected
    );

    // Retained identity survives frames and workspace switches.
    let _ = switch_to(&mut app, "Graph");
    let graph_frame = app.frame(demo_context(UiInput::default()));
    let graph_root = graph_frame
        .semantics
        .get(application_bar_root())
        .expect("same retained bar root on Graph");
    assert_eq!(graph_root.children, root.children);
    assert!(
        graph_frame
            .semantics
            .get(tab_widget_id(GRAPH_TAB_RAW))
            .expect("Graph tab")
            .state
            .selected
    );
    assert!(
        !graph_frame
            .semantics
            .get(tab_widget_id(EDIT_TAB_RAW))
            .expect("Edit tab")
            .state
            .selected
    );
}

#[test]
fn workspace_tab_activation_emits_application_action_exactly_once() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));

    let activated = click_at(&mut app, center(&initial, &SemanticRole::Tab, "Graph"));
    let mut actions = activated.actions.clone();
    let drained: Vec<_> = actions.drain().collect();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].action_id.as_str(), "workspace.graph");
    assert_eq!(drained[0].source, ActionSource::Button);
    assert_eq!(drained[0].context, ActionContext::Editor);
    assert_eq!(app.workspace(), DemoWorkspace::Graph);

    let settled = app.frame(demo_context(UiInput::default()));
    let graph_tab = settled
        .semantics
        .get(tab_widget_id(GRAPH_TAB_RAW))
        .expect("Graph tab");
    assert!(graph_tab.state.selected && !graph_tab.state.focused);

    let back = click_at(&mut app, center(&settled, &SemanticRole::Tab, "Edit"));
    let mut actions = back.actions.clone();
    let drained: Vec<_> = actions.drain().collect();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].action_id.as_str(), "workspace.edit");
    assert_eq!(app.workspace(), DemoWorkspace::Edit);
}

#[test]
fn active_workspace_focus_selection_and_menu_expansion_stay_independent() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let _activated = click_at(&mut app, center(&initial, &SemanticRole::Tab, "Graph"));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);

    // The activation focus target is application-owned (the workspace
    // action identity), not the selected tab: selected state and keyboard
    // focus remain independent.
    let graph_action_id = WidgetId::from_key("root").child("workspace.graph");
    assert_eq!(app.focused(), Some(graph_action_id));
    let settled = app.frame(demo_context(UiInput::default()));
    let graph_tab = settled
        .semantics
        .get(tab_widget_id(GRAPH_TAB_RAW))
        .expect("Graph tab");
    assert!(graph_tab.state.selected);
    assert!(!graph_tab.state.focused);

    // Expanding a menu moves focus to its heading without changing the
    // selected tab or the active workspace.
    let _ = app.frame(demo_context(key(Key::Function(10))));
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(only_menu_surface(&shown, "File menu"));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
    assert!(
        settled
            .semantics
            .get(tab_widget_id(GRAPH_TAB_RAW))
            .expect("Graph tab")
            .state
            .selected
    );
    let _ = app.frame(demo_context(key(Key::Escape)));
    let closed = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&closed));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
}

#[test]
fn f10_traversal_pointer_open_adjacent_replacement_and_escape_share_one_route() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let _ = click_at(
        &mut app,
        center(&initial, &SemanticRole::ListItem, "Backdrop"),
    );

    // F10 opens the first visible menu through the shared route.
    let _ = app.frame(demo_context(key(Key::Function(10))));
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "File menu"));
    assert!(only_menu_surface(&shown, "File menu"));
    assert_eq!(
        shown
            .semantics
            .get(heading_widget_id(FILE_MENU_RAW))
            .expect("File heading")
            .state
            .expanded,
        Some(true)
    );
    assert!(has_item(&shown, "Save Color Style"));

    // Top-level ArrowRight replaces the shown menu with the adjacent one.
    let _ = app.frame(demo_context(key(Key::ArrowRight)));
    let moved = app.frame(demo_context(UiInput::default()));
    assert!(only_menu_surface(&moved, "Edit menu"));
    assert_eq!(
        moved
            .semantics
            .get(heading_widget_id(FILE_MENU_RAW))
            .expect("File heading")
            .state
            .expanded,
        Some(false)
    );

    // Traversal wraps deterministically: Edit -> Help via ArrowLeft.
    let _ = app.frame(demo_context(key(Key::ArrowLeft)));
    let _ = app.frame(demo_context(key(Key::ArrowLeft)));
    let wrapped = app.frame(demo_context(UiInput::default()));
    assert!(only_menu_surface(&wrapped, "Help menu"));

    // Escape dismisses through the same route; keyboard menu focus returns
    // to the heading that opened the session (captured when the route
    // opened the File menu).
    let _ = app.frame(demo_context(key(Key::Escape)));
    let closed = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&closed));
    assert_eq!(app.focused(), Some(heading_widget_id(FILE_MENU_RAW)));

    // Pointer open on the same route.
    let reopened = open_menu(&mut app, &closed, "Window");
    assert!(has_label(&reopened, "Window menu"));

    // While the surface is armed with OutsideClickOrEscape dismissal it
    // captures lower-layer pointer input, so clicking another heading
    // dismisses through the shared route; reopening that heading then shows
    // its menu through the identical route.
    let _ = click_at(&mut app, center(&reopened, &SemanticRole::MenuItem, "View"));
    let dismissed_mid = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&dismissed_mid));
    let view_menu = open_menu(&mut app, &dismissed_mid, "View");
    assert!(only_menu_surface(&view_menu, "View menu"));
    assert!(has_item(&view_menu, "Select Tool"));
    assert!(has_item(&view_menu, "Transform Tool"));
    let outside = Point::new(8.0, 440.0);
    let _ = click_at(&mut app, outside);
    let dismissed = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&dismissed));
    // Bar headings do not take keyboard focus from pointer presses, so the
    // dismissal restores exactly the focus that preceded the pointer tour.
    assert_eq!(app.focused(), Some(heading_widget_id(FILE_MENU_RAW)));
}

#[test]
fn menu_item_invocation_emits_application_action_exactly_once() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let menu = open_menu(&mut app, &initial, "Window");
    let transition = click_at(
        &mut app,
        center(&menu, &SemanticRole::MenuItem, "Graph Workspace"),
    );
    let mut actions = transition.actions.clone();
    let drained: Vec<_> = actions.drain().collect();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].action_id.as_str(), "workspace.graph");
    assert_eq!(drained[0].source, ActionSource::Menu);
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
    let settled = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&settled));
}

#[test]
fn menu_state_refreshes_from_registry_without_replacing_active_menu() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let menu = open_menu(&mut app, &initial, "Edit");
    let apply = item_node(&menu, "Apply Shared State");
    assert!(!apply.state.disabled);

    // Availability changes refresh the open menu's item state while the
    // expanded menu stays exactly where it was. The shared route repaints
    // the mirrored surface on the frame after the change.
    app.set_apply_availability(DemoActionAvailability::Unavailable);
    let _ = app.frame(demo_context(UiInput::default()));
    let refreshed = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&refreshed, "Edit menu"));
    assert!(item_node(&refreshed, "Apply Shared State").state.disabled);
    assert_eq!(app.applied_revision(), 0);

    app.set_apply_availability(DemoActionAvailability::Available);
    let _ = app.frame(demo_context(UiInput::default()));
    let restored = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&restored, "Edit menu"));
    assert!(!item_node(&restored, "Apply Shared State").state.disabled);
    let _ = app.frame(demo_context(key(Key::Escape)));

    // Hiding removes every visible item from the Edit menu, so the expanded
    // state reconciles away and its heading drops out of the bar entirely.
    let base = app.frame(demo_context(UiInput::default()));
    let menu = open_menu(&mut app, &base, "Edit");
    assert!(has_label(&menu, "Edit menu"));
    app.set_apply_availability(DemoActionAvailability::Hidden);
    let _ = app.frame(demo_context(UiInput::default()));
    let hidden = app.frame(demo_context(UiInput::default()));
    assert!(!has_any_menu_surface(&hidden));
    assert!(
        hidden
            .semantics
            .get(heading_widget_id(EDIT_MENU_RAW))
            .is_none()
    );
    assert!(!has_node(
        &hidden,
        &SemanticRole::MenuItem,
        "Apply Shared State"
    ));
}

#[test]
fn workspace_switching_preserves_each_workspaces_application_state() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let _ = click_at(
        &mut app,
        center(&initial, &SemanticRole::ListItem, "Character"),
    );
    assert_eq!(
        app.selected_asset().expect("selected asset").name,
        "Character"
    );
    let committed_playhead = app.committed_playhead_frame();

    let _ = switch_to(&mut app, "Graph");
    let graph = app.frame(demo_context(graph_click(Point::new(100.0, 184.0))));
    assert!(app.graph_workspace().selection().contains(
        stern::widgets::node_graph::NodeGraphSelectionTarget::Node(
            stern::widgets::node_graph::NodeId::from_raw(1)
        )
    ));
    let _ = graph;

    // Back to Edit: asset selection and timeline commit survive.
    let graph_settled = app.frame(demo_context(UiInput::default()));
    let _ = switch_to_from(&mut app, &graph_settled, "Edit");
    assert_eq!(
        app.selected_asset().expect("selected asset").name,
        "Character"
    );
    assert_eq!(app.committed_playhead_frame(), committed_playhead);

    // And Graph keeps its own selection across the round trip.
    let edit_settled = app.frame(demo_context(UiInput::default()));
    let _ = switch_to_from(&mut app, &edit_settled, "Graph");
    assert!(app.graph_workspace().selection().contains(
        stern::widgets::node_graph::NodeGraphSelectionTarget::Node(
            stern::widgets::node_graph::NodeId::from_raw(1)
        )
    ));
}

fn headings_focusable_count(output: &FrameOutput) -> usize {
    [
        FILE_MENU_RAW,
        EDIT_MENU_RAW,
        VIEW_MENU_RAW,
        WINDOW_MENU_RAW,
        HELP_MENU_RAW,
    ]
    .into_iter()
    .filter(|raw| {
        output
            .semantics
            .get(heading_widget_id(*raw))
            .is_some_and(|heading| heading.focusable)
    })
    .count()
}

fn switch_to(app: &mut DemoApp, tab: &str) -> FrameOutput {
    let current = app.frame(demo_context(UiInput::default()));
    switch_to_from(app, &current, tab)
}

fn switch_to_from(app: &mut DemoApp, current: &FrameOutput, tab: &str) -> FrameOutput {
    click_at(app, center(current, &SemanticRole::Tab, tab))
}

fn open_menu(app: &mut DemoApp, current: &FrameOutput, heading: &str) -> FrameOutput {
    let _ = click_at(app, center(current, &SemanticRole::MenuItem, heading));
    app.frame(demo_context(UiInput::default()))
}

fn only_menu_surface(output: &FrameOutput, label: &str) -> bool {
    has_label(output, label)
        && ![
            "File menu",
            "Edit menu",
            "View menu",
            "Window menu",
            "Help menu",
        ]
        .into_iter()
        .any(|candidate| candidate != label && has_label(output, candidate))
}

fn has_any_menu_surface(output: &FrameOutput) -> bool {
    [
        "File menu",
        "Edit menu",
        "View menu",
        "Window menu",
        "Help menu",
    ]
    .into_iter()
    .any(|label| has_label(output, label))
}

fn has_item(output: &FrameOutput, label: &str) -> bool {
    has_node(output, &SemanticRole::MenuItem, label)
}

fn item_node<'a>(output: &'a FrameOutput, label: &'a str) -> &'a stern::core::SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::MenuItem && node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("menu item {label}"))
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn has_node(output: &FrameOutput, role: &SemanticRole, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role && node.label.as_deref() == Some(label))
}

fn center(output: &FrameOutput, role: &SemanticRole, label: &str) -> Point {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic {role:?}: {label}"))
        .bounds
        .center()
}

fn click_at(app: &mut DemoApp, point: Point) -> FrameOutput {
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

fn key(key: Key) -> UiInput {
    let event = KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn graph_click(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: stern::core::MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: stern::core::MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}
