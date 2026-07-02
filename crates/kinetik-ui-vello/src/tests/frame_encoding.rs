use super::common::{assert_approx, resources};
use crate::{RenderFrameInput, RenderResources, VelloRenderer};
use kinetik_ui_core::render::TexturePrimitive;
use kinetik_ui_core::{
    Brush, Color, CornerRadius, ImageId, ImagePrimitive, LinePrimitive, PathElement, PathPrimitive,
    Point, Primitive, Rect, RectPrimitive, ScaleFactor, Size, Stroke, TextPrimitive, TextureId,
    ViewportInfo,
};

#[test]
fn frame_submission_encodes_vello_geometry() {
    let mut renderer = VelloRenderer::new();
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 40.0, 24.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
            radius: CornerRadius::all(4.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(40.0, 24.0),
            stroke: Stroke::new(2.0, Brush::Solid(Color::WHITE)),
        }),
        Primitive::Path(PathPrimitive::new(
            vec![
                PathElement::MoveTo(Point::new(6.0, 6.0)),
                PathElement::LineTo(Point::new(30.0, 6.0)),
                PathElement::LineTo(Point::new(18.0, 20.0)),
                PathElement::Close,
            ],
            Some(Brush::Solid(Color::rgba(0.2, 0.6, 0.9, 1.0))),
            Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
        )),
    ];
    let resources = RenderResources::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    assert!(!renderer.scene().encoding().is_empty());
    assert!(renderer.scene().encoding().n_paths >= 2);
}

#[test]
fn frame_submission_encodes_fallback_text_and_visible_resource_placeholders() {
    let mut renderer = VelloRenderer::new();
    let resources = resources();
    let primitives = vec![
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.0, 16.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 24.0, 32.0, 24.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(40.0, 24.0, 32.0, 24.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    assert!(!renderer.scene().encoding().is_empty());
    assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
    assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
    assert!(renderer.scene().encoding().resources.patches.len() >= 2);
}

#[test]
fn frame_submission_encodes_axis_aligned_text_at_physical_font_size() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(200, 200),
            ScaleFactor::new(2.0),
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
    assert_approx(glyph_run.font_size, 24.0);
    assert!(glyph_run.hint);
}
