use super::helpers::{
    Point, Primitive, SemanticActionKind, SemanticRole, ShowcaseApp, ShowcasePage, click,
    contains_text_in_order, count_primitives, count_semantic_role, has_text, semantic_node,
};

#[test]
fn systems_palette_invokes_actions() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    click(&mut app, Point::new(930.0, 160.0));

    assert_eq!(app.action_count(), 1);
    assert!(app.workspace_snapshot.is_some());
}

#[test]
fn systems_page_exposes_runtime_diagnostics() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    let has_snapshot = app.primitives().iter().any(
        |primitive| matches!(primitive, Primitive::Text(text) if text.text == "Runtime Snapshot"),
    );

    assert!(has_snapshot);
}

#[test]
fn systems_page_structural_smoke_emits_actions_overlays_palette_and_stress() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 180);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 130);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 20);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 1
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Actions, Overlays, Diagnostics, Stress",
            "Action Router",
            "Overlay Stack",
            "Command Palette",
            "Primitive Stress",
            "Runtime Snapshot",
        ]
    ));

    assert!(semantic_node(
        &app,
        &SemanticRole::Button,
        "Record Dispatch"
    ));
    assert!(semantic_node(&app, &SemanticRole::Button, "Menu Save"));
    assert!(semantic_node(&app, &SemanticRole::Menu, "Menu"));
    assert!(semantic_node(
        &app,
        &SemanticRole::CommandPalette,
        "Command Palette"
    ));
    assert!(semantic_node(
        &app,
        &SemanticRole::Custom("popover".to_owned()),
        "Popover"
    ));
    assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 3);
    assert!(app.output().semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Menu
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Dismiss)
    }));

    click(&mut app, Point::new(100.0, 210.0));

    assert_eq!(app.action_count(), 1);
    assert!(app.workspace_snapshot.is_some());
    assert!(has_text(&app, "Workspace snapshot captured in memory"));

    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    click(&mut app, Point::new(930.0, 160.0));

    assert_eq!(app.action_count(), 1);
    assert!(app.workspace_snapshot.is_some());
    assert!(has_text(&app, "Workspace snapshot captured in memory"));
}

#[test]
fn showcase_action_truth_disabled_palette_row_cannot_invoke() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    click(&mut app, Point::new(930.0, 192.0));

    assert_eq!(app.action_count(), 0);
    assert_eq!(app.workspace_snapshot, None);
}
