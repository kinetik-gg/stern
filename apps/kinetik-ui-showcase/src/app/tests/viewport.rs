use super::helpers::{
    PlatformRequest, Point, Primitive, SemanticActionKind, SemanticRole, ShowcaseApp,
    ShowcaseInput, ShowcasePage, TextureId, contains_text_in_order, count_primitives,
    count_semantic_role, has_text, semantic_node, semantic_role_has_action,
};

#[test]
fn viewport_page_structural_smoke_emits_texture_viewport_semantics_and_platform_requests() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Viewport);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 60);
    assert_eq!(
        count_primitives(&app, |primitive| matches!(primitive, Primitive::Texture(_))),
        2
    );
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 5);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Viewport, Texture, and Overlay Surface",
            "Viewport Controls",
            "Pan/Zoom Texture Surface",
            "3D/Video Boundary",
        ]
    ));
    assert!(has_text(
        &app,
        "Surface: 384x216 | Guides: 3 | Crosshair: 192,108"
    ));

    assert!(semantic_node(
        &app,
        &SemanticRole::Viewport,
        "Pan/Zoom Texture Surface"
    ));
    assert!(semantic_node(&app, &SemanticRole::Button, "Fit"));
    assert!(semantic_node(&app, &SemanticRole::Button, "Actual Size"));
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 3);
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Viewport,
        &SemanticActionKind::Focus
    ));
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

    let resources = app.render_resources();
    assert!(resources.texture(TextureId::from_raw(99)).is_some());
    assert!(resources.texture(TextureId::from_raw(101)).is_some());

    app.update(&ShowcaseInput {
        mouse: Some(Point::new(1090.0, 240.0)),
        ..ShowcaseInput::default()
    });

    assert!(
        app.output()
            .platform_requests
            .contains(&PlatformRequest::SetCursor(
                kinetik_ui::core::CursorShape::PointingHand
            ))
    );
}
