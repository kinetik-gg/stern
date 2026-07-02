use super::helpers::{
    Point, Primitive, Rect, ShowcaseApp, ShowcaseInput, ShowcasePage, Size, WidgetId, click,
    has_text,
};

#[test]
fn clicking_navigation_changes_page() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    let point = Point::new(620.0, 20.0);
    let visible_nav_id = WidgetId::from_key("root").child(("nav", ShowcasePage::Viewport as u8));

    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: true,
        ..ShowcaseInput::default()
    });

    assert_eq!(app.memory.pressed(), Some(visible_nav_id));

    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: false,
        ..ShowcaseInput::default()
    });

    assert_eq!(app.page(), ShowcasePage::Viewport);
    assert!(has_text(&app, "Viewport, Texture, and Overlay Surface"));
    assert!(has_text(&app, "Page: Viewport"));
    assert!(!has_text(&app, "Component Gallery"));
}

#[test]
fn viewport_size_sets_logical_frame_context() {
    let mut app = ShowcaseApp::new();

    app.set_viewport_size(Size::new(720.0, 450.0));

    assert_eq!(app.viewport_size(), Size::new(720.0, 450.0));
    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.primitives().iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if rect.rect == Rect::new(0.0, 0.0, 720.0, 450.0)
    )));
}

#[test]
fn resized_hit_testing_uses_logical_coordinates() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    app.set_viewport_size(Size::new(720.0, 450.0));

    click(&mut app, Point::new(35.0, 77.0));
    assert_eq!(app.action_count(), 0);

    click(&mut app, Point::new(70.0, 154.0));

    assert_eq!(app.action_count(), 1);
}

#[test]
fn page_names_are_parseable_for_render_tools() {
    assert_eq!(
        ShowcaseApp::page_from_name("layout"),
        Some(ShowcasePage::Layout)
    );
    assert_eq!(ShowcaseApp::page_from_name("unknown"), None);
}

#[test]
fn showcase_docs_reach_s10_s13_review_matrix() {
    let matrix = include_str!("../../../../../docs/catalogue-conformance-matrix.md");

    for slug in [
        "s10-outliner-tree-selection-semantics",
        "s10-asset-browser-grid-list-metadata",
        "s10-inline-edit-rename-lifecycle",
        "s10-collection-drag-drop-context",
        "s10-collection-filter-sort-selection-preservation",
        "s11-timeline-layout-coordinate-selection",
        "s11-ruler-ticks-timecode",
        "s11-transport-action-controls",
        "s11-timeline-snapping",
        "s11-timeline-preservation",
        "s12-viewport-surface-overlays",
        "s12-viewport-tools-transform-handles",
        "s12-viewport-action-routing",
        "s12-viewport-guides-rulers-safe-areas-hud",
        "s13-progress-indicator-metadata",
        "s13-job-list-progress-cancel",
        "s13-diagnostic-strip-codes-fields-ordering",
        "s13-feedback-stack-lifetime-repaint",
    ] {
        assert!(matrix.contains(slug), "{slug}");
    }

    for required in [
        "`Partial`",
        "Editor page",
        "Viewport page",
        "cargo run -p kinetik-ui-showcase -- --dump-review-artifacts s14-s10-s13-matrix",
        "Do not commit them as raster baselines",
    ] {
        assert!(matrix.contains(required), "{required}");
    }
}
