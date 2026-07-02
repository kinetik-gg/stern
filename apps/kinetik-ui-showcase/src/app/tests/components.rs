use super::helpers::{
    PlatformRequest, Point, Primitive, SemanticActionKind, SemanticRole, SemanticValue,
    ShowcaseApp, ShowcasePage, click, contains_text_in_order, count_primitives,
    count_semantic_role, has_text, semantic_node, semantic_role_has_action,
};

#[test]
fn components_page_structural_smoke_emits_controls_semantics_and_platform_requests() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 120);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 40);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 25);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Image(_))) >= 2);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 1);
    assert!(contains_text_in_order(
        &app,
        &[
            "Component Gallery",
            "Controls",
            "Text Input",
            "Lists, Grids, Tables",
            "Reusable Panel States",
            "Primitive Stream",
        ]
    ));

    assert_eq!(count_semantic_role(&app, &SemanticRole::Button), 2);
    assert_eq!(count_semantic_role(&app, &SemanticRole::IconButton), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::CheckBox), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::Toggle), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::RadioButton), 2);
    assert_eq!(count_semantic_role(&app, &SemanticRole::Slider), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::SearchField), 1);
    assert!(count_semantic_role(&app, &SemanticRole::TextField) >= 3);
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 5);
    assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 4);
    assert!(count_semantic_role(&app, &SemanticRole::Tab) >= 3);
    assert!(semantic_node(&app, &SemanticRole::Button, "Run Action"));
    assert!(semantic_node(&app, &SemanticRole::Button, "Disabled"));
    assert!(semantic_node(
        &app,
        &SemanticRole::IconButton,
        "Icon button"
    ));

    let slider = app
        .output()
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Slider)
        .expect("slider semantics");
    assert!(
        slider
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetValue)
    );
    assert!(matches!(
        slider.state.value,
        Some(SemanticValue::Number { current, min: 0.0, max: 1.0 })
            if (current - app.strength()).abs() < f32::EPSILON
    ));

    click(&mut app, Point::new(940.0, 160.0));

    assert!(!app.output().platform_requests.iter().any(|request| {
        matches!(
            request,
            PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
        )
    }));
}

#[test]
fn layout_page_structural_smoke_emits_layout_dock_table_and_actions() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Layout);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 100);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 60);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 30);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Layout, Docking, and Data Surfaces",
            "Measurement-Aware Layout",
            "Interactive Dock Model",
            "Virtualized Table Model",
        ]
    ));
    assert!(has_text(&app, "Rows: 7 | Columns: 4 | Overscan: 0"));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text.starts_with("Frames: "))
    }));

    assert!(semantic_node(
        &app,
        &SemanticRole::Dock,
        "Interactive Dock Model"
    ));
    assert!(semantic_node(
        &app,
        &SemanticRole::Table,
        "Virtualized Table Model"
    ));
    assert!(semantic_node(&app, &SemanticRole::Button, "Split Tab"));
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 4);
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Slider,
        &SemanticActionKind::SetValue
    ));
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Button,
        &SemanticActionKind::Invoke
    ));

    click(&mut app, Point::new(700.0, 162.0));

    assert!(has_text(&app, "Frame 9"));
}

#[test]
fn component_status_reflects_toggle_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(231.0, 204.0));

    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Toggle: true")
    }));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Text(text)
                if text.text == "checkbox=true toggle=true radio=1 selected_row=2"
        )
    }));
}

#[test]
fn component_status_reflects_checkbox_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(71.0, 204.0));

    assert!(!app.checkbox);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Checkbox: false")
    }));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Text(text)
                if text.text == "checkbox=false toggle=false radio=1 selected_row=2"
        )
    }));
}

#[test]
fn component_status_reflects_radio_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(170.0, 252.0));

    assert_eq!(app.radio, 1);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Radio: Radio B")
    }));
}
