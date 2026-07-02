use super::common::{assert_approx, text_layout_resource};
use crate::{
    RenderFrameInput, RenderResources, ShapedTextCache, VelloRenderer, physical_text_layout,
    physical_text_layout_for_key, quantize_physical_text_extent, root_transform,
};
use kinetik_ui_core::{
    Brush, Color, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive, Transform,
    Vec2, ViewportInfo,
};
use kinetik_ui_text::{CosmicTextEngine, TextLayoutKey, TextStyle, fonts};

#[test]
fn physical_text_layout_shapes_at_device_font_size() {
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();
    let layout = physical_text_layout(
        &mut engine,
        &mut cache,
        root_transform(1.5),
        "Label",
        "monospace",
        12.0,
        17.0,
    )
    .expect("axis-aligned physical layout");

    assert!(!layout.runs.is_empty());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.font.data.data() == fonts::GEIST_MONO_VARIABLE)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 26.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_layout_quantizes_fractional_device_metrics() {
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();
    let layout = physical_text_layout(
        &mut engine,
        &mut cache,
        root_transform(1.25),
        "Sharp",
        "sans-serif",
        14.0,
        19.0,
    )
    .expect("axis-aligned physical layout");

    assert!(!layout.runs.is_empty());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.font.data.data() == fonts::INTER_VARIABLE)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 24.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_extent_quantizes_fractional_device_widths() {
    assert_approx(quantize_physical_text_extent(86.25), 86.0);
    assert_approx(quantize_physical_text_extent(86.5), 87.0);
    assert_approx(quantize_physical_text_extent(0.0), 0.0);
}

#[test]
fn physical_text_layout_for_key_quantizes_wrap_width_at_device_scale() {
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon",
        TextStyle::new("sans-serif", 12.0, 17.0),
        69.0,
        true,
    );
    let mut expected_engine = CosmicTextEngine::new();
    let expected = expected_engine.shape_text(&TextLayoutKey::new(
        key.text.clone(),
        TextStyle::new("sans-serif", 15.0, 21.0),
        86.0,
        true,
    ));
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();

    let layout = physical_text_layout_for_key(&mut engine, &mut cache, root_transform(1.25), &key)
        .expect("axis-aligned physical layout");

    assert_eq!(layout.line_count, expected.line_count);
    assert_eq!(layout.lines.len(), expected.lines.len());
    assert_approx(layout.size.width, expected.size.width);
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 15.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_layout_for_key_preserves_wrap_width_at_device_scale() {
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon",
        TextStyle::new("sans-serif", 12.0, 17.0),
        68.0,
        true,
    );
    let mut expected_engine = CosmicTextEngine::new();
    let expected = expected_engine.shape_text(&TextLayoutKey::new(
        key.text.clone(),
        TextStyle::new("sans-serif", 18.0, 26.0),
        102.0,
        true,
    ));
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();

    let layout = physical_text_layout_for_key(&mut engine, &mut cache, root_transform(1.5), &key)
        .expect("axis-aligned physical layout");

    assert_eq!(layout.line_count, expected.line_count);
    assert_eq!(layout.lines.len(), expected.lines.len());
    assert!(layout.line_count > 1);
    assert_approx(layout.size.width, expected.size.width);
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 26.0).abs() < f32::EPSILON)
    );
}

#[test]
fn frame_submission_encodes_registered_shaped_text_layout() {
    let layout = TextLayoutId::from_raw(44);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
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

    assert!(output.diagnostics.is_empty());
    assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
    assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
}

#[test]
fn registered_text_layout_renders_with_fractional_scale_physical_shape() {
    let layout = TextLayoutId::from_raw(45);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.3, 16.4),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
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
    let glyph = renderer
        .scene()
        .encoding()
        .resources
        .glyphs
        .first()
        .expect("glyph");

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    assert_approx(glyph.x, 5.0);
    assert_approx(glyph.y, 21.0);
}

#[test]
fn translated_registered_text_layout_stays_snapped_at_fractional_dpi() {
    let layout = TextLayoutId::from_raw(47);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
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
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    let first_glyph = glyphs.first().expect("glyph");
    assert!((first_glyph.x - first_glyph.x.round()).abs() <= 0.001);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "registered text should snap glyph baselines under translation"
    );
}
