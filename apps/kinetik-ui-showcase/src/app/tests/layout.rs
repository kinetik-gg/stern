use super::helpers::{Point, Primitive, ShowcaseApp, ShowcasePage, click};

#[test]
fn layout_page_split_demo_changes_dock_preview() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Layout);

    click(&mut app, Point::new(700.0, 162.0));

    assert!(
        app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Frame 9")
        })
    );
}
