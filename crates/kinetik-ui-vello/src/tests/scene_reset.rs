use crate::{RenderFrameInput, RenderResources, VelloRenderer};
use kinetik_ui_core::{
    Brush, Color, CornerRadius, Primitive, Rect, RectPrimitive, ScaleFactor, Size, ViewportInfo,
};

#[test]
fn frame_submission_resets_retained_scene() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 40.0, 24.0),
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    assert!(!renderer.scene().encoding().is_empty());

    renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &[],
        resources: &resources,
    });

    assert!(renderer.scene().encoding().is_empty());
}
