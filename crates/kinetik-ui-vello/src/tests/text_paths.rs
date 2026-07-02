use super::common::{assert_approx, text_layout_resource};
use crate::{RenderFrameInput, RenderResources, VelloRenderer};
use kinetik_ui_core::{
    Brush, Color, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive, Transform,
    ViewportInfo,
};

#[test]
fn near_uniform_registered_text_uses_physical_hinted_layout() {
    let layout = TextLayoutId::from_raw(57);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.250_01,
            m22: 1.249_99,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 1);
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    assert!(glyph_run.glyph_transform.is_none());
}

#[test]
fn tiny_axis_aligned_skew_still_uses_device_text_path() {
    let layout = TextLayoutId::from_raw(58);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.25,
            m12: 0.000_01,
            m21: -0.000_01,
            m22: 1.25,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 1);
    assert!(glyph_run.hint);
}

#[test]
fn meaningful_rotation_keeps_general_text_path() {
    let layout = TextLayoutId::from_raw(59);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let angle = 0.01_f32;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: angle.cos(),
            m12: angle.sin(),
            m21: -angle.sin(),
            m22: angle.cos(),
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 0);
    assert!(!glyph_run.hint);
}
