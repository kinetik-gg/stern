//! Public-facade integration contract for the Stern demo.

use stern::core::{
    PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, ScaleFactor, SemanticRole,
    UiInput, UiInputEvent,
};
use stern::platform_winit::{WinitInputAdapter, WinitPlatformRequests};
use stern_demo::{DEMO_TITLE, DemoApp, DemoWorkspace, demo_context, has_component_semantics};

#[test]
fn public_consumer_contract_emits_components_semantics_focus_and_platform_evidence() {
    let mut app = DemoApp::new();

    let mut platform_input = WinitInputAdapter::new(ScaleFactor::ONE);
    platform_input.set_window_focused(true);
    let normalized_input = platform_input.into_input();
    assert!(normalized_input.window_focused);
    assert!(
        normalized_input
            .events
            .iter()
            .any(|event| matches!(event, UiInputEvent::WindowFocusChanged(true)))
    );
    let normalized_output = app.frame(demo_context(normalized_input));
    let painted_icons = normalized_output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Icon(icon) => Some(icon.icon.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        painted_icons,
        vec![
            stern_icons_phosphor::regular::CURSOR.icon().id(),
            stern_icons_phosphor::regular::ARROWS_OUT_CARDINAL
                .icon()
                .id(),
            stern_icons_phosphor::regular::PENCIL_SIMPLE.icon().id(),
            stern_icons_phosphor::regular::GRAPH.icon().id(),
            stern_icons_phosphor::regular::CHECK_CIRCLE.icon().id(),
            stern_icons_phosphor::regular::FLOPPY_DISK.icon().id(),
        ]
    );
    let translated_requests = WinitPlatformRequests::from_frame_output(&normalized_output);
    assert_eq!(translated_requests.window_title(), Some(DEMO_TITLE));

    let point = semantic_center(
        &normalized_output,
        &SemanticRole::IconButton,
        "Edit Workspace",
    );
    let _ = app.frame(demo_context(pointer_input(point, true, true, false)));
    let output = app.frame(demo_context(pointer_input(point, false, false, true)));

    assert!(has_component_semantics(&output));
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Edit Workspace")
    }));
    let row = semantic_center(&output, &SemanticRole::ListItem, "Backdrop");
    let _ = app.frame(demo_context(pointer_input(row, true, true, false)));
    let focused_output = app.frame(demo_context(pointer_input(row, false, false, true)));
    assert!(focused_output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::ListItem
            && node.label.as_deref() == Some("Backdrop")
            && node.state.focused
    }));
    assert!(output.platform_requests.iter().any(
        |request| matches!(request, PlatformRequest::SetWindowTitle(title) if title == DEMO_TITLE)
    ));

    let resources = app.render_resources();
    let translation =
        stern::render_vello::translate_primitives(&focused_output.primitives, &resources);
    assert!(!translation.commands.is_empty());
    let accessibility = stern::platform_winit::WinitAccessibilityUpdate::from_frame_output(
        &focused_output,
        app.focused(),
    )
    .expect("public semantic output is structurally valid");
    assert!(!accessibility.snapshot.nodes.is_empty());
}

#[test]
fn public_consumer_contract_routes_workspace_actions_to_application_state() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let point = semantic_center(&initial, &SemanticRole::IconButton, "Graph Workspace");
    let _ = app.frame(demo_context(pointer_input(point, true, true, false)));
    let output = app.frame(demo_context(pointer_input(point, false, false, true)));

    let mut actions = output.actions.clone();
    assert!(
        actions
            .drain()
            .any(|invocation| invocation.action_id.as_str() == "workspace.graph")
    );
    assert_eq!(app.workspace(), DemoWorkspace::Graph);

    let graph_output = app.frame(demo_context(UiInput::default()));
    assert!(
        graph_output
            .semantics
            .nodes()
            .iter()
            .any(|node| matches!(&node.role, SemanticRole::Custom(role) if role == "node-graph"))
    );
    assert!(
        graph_output
            .semantics
            .get(app.graph_workspace().root_id())
            .expect("public Graph workspace root")
            .focusable
    );
    assert!(graph_output.platform_requests.iter().any(
        |request| matches!(request, PlatformRequest::SetWindowTitle(title) if title == DEMO_TITLE)
    ));
    assert_eq!(
        WinitPlatformRequests::from_frame_output(&graph_output).window_title(),
        Some(DEMO_TITLE)
    );
}

fn semantic_center(output: &stern::core::FrameOutput, role: &SemanticRole, label: &str) -> Point {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .expect("semantic control")
        .bounds
        .center()
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
