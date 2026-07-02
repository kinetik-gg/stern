use super::helpers::{Point, Primitive, ShowcaseApp, click};

#[test]
fn state_changes_produce_different_frames() {
    let mut app = ShowcaseApp::new();
    let before = crate::raster::rasterize(&app.primitives(), 1440, 900);

    click(&mut app, Point::new(70.0, 154.0));
    let after = crate::raster::rasterize(&app.primitives(), 1440, 900);

    assert_ne!(before.pixels, after.pixels);
}

#[test]
fn showcase_uses_widget_generated_primitives() {
    let app = ShowcaseApp::new();
    let primitives = app.primitives();

    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Texture(_)))
    );
    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Line(_) | Primitive::Path(_)))
    );
    assert!(
        primitives
            .iter()
            .filter(|item| matches!(item, Primitive::Rect(_)))
            .count()
            > 20
    );
}
