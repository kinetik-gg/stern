use super::super::{editor_nav_bounds, editor_nav_items, nav_items};
use super::helpers::{
    Point, Primitive, Rect, ShowcaseApp, ShowcaseInput, ShowcasePage, Size, WidgetId, click,
    has_text,
};

#[test]
fn showcase_navigation_clicking_gallery_page_changes_page() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    let point = nav_point(ShowcasePage::Viewport);
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
fn showcase_navigation_catalogue_has_five_unique_round_trip_entries() {
    let slugs = ShowcasePage::ALL.map(ShowcasePage::slug);
    let labels = ShowcasePage::ALL.map(ShowcasePage::label);

    assert_eq!(
        slugs,
        ["editor", "components", "layout", "viewport", "systems"]
    );
    assert_eq!(
        labels,
        ["Editor", "Components", "Layout", "Viewport", "Systems"]
    );
    for (index, page) in ShowcasePage::ALL.into_iter().enumerate() {
        assert_eq!(ShowcasePage::parse(page.slug()), Some(page));
        assert_eq!(ShowcasePage::parse(page.label()), Some(page));
        assert!(!slugs[..index].contains(&page.slug()));
        assert!(!labels[..index].contains(&page.label()));
    }
}

#[test]
fn showcase_navigation_editor_and_gallery_are_clickable_both_ways() {
    let mut app = ShowcaseApp::new();
    assert_eq!(app.page(), ShowcasePage::Editor);

    click(&mut app, editor_nav_point(ShowcasePage::Components));
    assert_eq!(app.page(), ShowcasePage::Components);

    click(&mut app, nav_point(ShowcasePage::Editor));
    assert_eq!(app.page(), ShowcasePage::Editor);
}

#[test]
fn showcase_navigation_editor_selector_is_unoccluded_and_owns_clicks() {
    let mut app = ShowcaseApp::new();
    for viewport in [
        Rect::new(0.0, 0.0, 1440.0, 900.0),
        Rect::new(0.0, 0.0, 720.0, 450.0),
        Rect::new(0.0, 0.0, 240.0, 160.0),
    ] {
        let protected_editor_chrome = Rect::new(0.0, 0.0, viewport.width, 64.0);
        let nav_bounds = editor_nav_bounds(viewport);
        assert!(viewport.contains_rect(nav_bounds));
        assert!(protected_editor_chrome.intersection(nav_bounds).is_none());
        assert!(editor_nav_items(viewport).iter().all(|(_, item)| {
            nav_bounds.contains_rect(*item) && protected_editor_chrome.intersection(*item).is_none()
        }));
    }

    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let status_bar = Rect::new(0.0, viewport.max_y() - 24.0, viewport.width, 24.0);
    let nav_bounds = editor_nav_bounds(viewport);

    let primitives = app.primitives();
    let status_index = primitives
        .iter()
        .rposition(
            |primitive| matches!(primitive, Primitive::Rect(rect) if rect.rect == status_bar),
        )
        .expect("Editor status bar is painted");
    let nav_background_index = primitives
        .iter()
        .rposition(
            |primitive| matches!(primitive, Primitive::Rect(rect) if rect.rect == nav_bounds),
        )
        .expect("Editor navigation background is painted");
    assert!(nav_background_index > status_index);
    for page in ShowcasePage::ALL {
        assert!(primitives[nav_background_index + 1..].iter().any(
            |primitive| matches!(primitive, Primitive::Text(text) if text.text == page.label())
        ));
    }

    let point = editor_nav_point(ShowcasePage::Components);
    let nav_id = WidgetId::from_key("root").child(("nav", ShowcasePage::Components as u8));
    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: true,
        ..ShowcaseInput::default()
    });
    assert_eq!(app.memory.pressed(), Some(nav_id));

    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: false,
        ..ShowcaseInput::default()
    });
    assert_eq!(app.page(), ShowcasePage::Components);
}

#[test]
fn showcase_navigation_selector_is_rendered_on_every_page() {
    let mut app = ShowcaseApp::new();

    for current in ShowcasePage::ALL {
        app.set_page(current);
        for page in ShowcasePage::ALL {
            assert!(
                has_text(&app, page.label()),
                "{} selector should include {}",
                current.label(),
                page.label()
            );
        }
    }
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
fn showcase_navigation_page_names_are_parseable_for_render_tools() {
    assert_eq!(
        ShowcaseApp::page_from_name("layout"),
        Some(ShowcasePage::Layout)
    );
    assert_eq!(ShowcaseApp::page_from_name("unknown"), None);
}

fn nav_point(page: ShowcasePage) -> Point {
    nav_items(1440.0)
        .into_iter()
        .find_map(|(candidate, rect)| (candidate == page).then_some(rect.center()))
        .expect("canonical page has a navigation item")
}

fn editor_nav_point(page: ShowcasePage) -> Point {
    editor_nav_items(Rect::new(0.0, 0.0, 1440.0, 900.0))
        .into_iter()
        .find_map(|(candidate, rect)| (candidate == page).then_some(rect.center()))
        .expect("canonical page has an Editor navigation item")
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
        "`Experimental`",
        "`M/P/I/A11y/PF/LW`",
        "currently proves only `M`",
        "Editor page",
        "Viewport page",
        "cargo run -p kinetik-ui-showcase -- --dump-review-artifacts s14-s10-s13-matrix",
        "Do not commit them as raster baselines",
    ] {
        assert!(matrix.contains(required), "{required}");
    }
}
