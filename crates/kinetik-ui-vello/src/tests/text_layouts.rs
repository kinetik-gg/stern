use super::common::assert_approx;
use crate::{RenderFrameInput, RenderResources, TextLayoutResource, VelloRenderer};
use kinetik_ui_core::{
    Brush, Color, PhysicalSize, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive,
    Transform, Vec2, ViewportInfo,
};
use kinetik_ui_text::{CosmicTextEngine, TextLayoutKey, TextStyle};

fn resource(id: TextLayoutId, key: TextLayoutKey) -> TextLayoutResource {
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&key);
    TextLayoutResource {
        id,
        key,
        layout: std::sync::Arc::new(layout),
    }
}

fn viewport(scale: f64) -> ViewportInfo {
    let physical = match scale {
        1.25 => 125,
        1.5 => 150,
        1.75 => 175,
        _ => panic!("unsupported fixture scale {scale}"),
    };
    ViewportInfo::new(
        Size::new(100.0, 100.0),
        PhysicalSize::new(physical, physical),
        ScaleFactor::new(scale),
    )
}

fn primitive(layout: Option<TextLayoutId>, text: &str) -> Primitive {
    Primitive::Text(TextPrimitive {
        layout,
        origin: Point::new(4.3, 16.4),
        text: text.to_owned(),
        family: "serif".to_owned(),
        size: 7.0,
        line_height: 9.0,
        brush: Brush::Solid(Color::WHITE),
    })
}

#[test]
fn layoutless_text_shapes_logically_and_scales_without_metric_quantization() {
    let primitives = [Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Fallback".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 17.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &RenderResources::new(),
    });
    let run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_approx(run.font_size, 16.25);
    assert!(run.hint);
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn registered_text_ignores_conflicting_primitive_metadata() {
    let id = TextLayoutId::from_raw(44);
    let key = TextLayoutKey::new(
        "Registered authority",
        TextStyle::new("sans-serif", 13.0, 17.0),
        200.0,
        false,
    );
    let expected = resource(id, key);
    let expected_ids = expected
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let mut resources = RenderResources::new();
    resources.register_text_layout(expected);
    let primitives = [primitive(Some(id), "wrong fallback")];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>(),
        expected_ids
    );
    assert!(
        encoding
            .resources
            .glyph_runs
            .iter()
            .all(|run| (run.font_size - 16.25).abs() <= 0.000_1)
    );
    assert_eq!(renderer.cached_text_layout_count(), 0);
}

#[test]
fn registered_wrapped_layout_keeps_its_original_line_and_glyph_topology() {
    let id = TextLayoutId::from_raw(45);
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon zeta",
        TextStyle::new("sans-serif", 13.0, 17.0),
        72.0,
        true,
    );
    let expected = resource(id, key);
    assert!(expected.layout.line_count > 1);
    let expected_glyphs = expected.layout.glyph_count();
    let mut resources = RenderResources::new();
    resources.register_text_layout(expected);
    let primitives = [primitive(Some(id), "unwrapped conflict")];
    let mut renderer = VelloRenderer::new();

    for scale in [1.25, 1.5, 1.75] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(output.diagnostics.is_empty());
        assert_eq!(
            renderer.scene().encoding().resources.glyphs.len(),
            expected_glyphs
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
    }
}

#[test]
fn translated_registered_text_uses_exact_scaled_font_size_and_absolute_snapping() {
    let id = TextLayoutId::from_raw(47);
    let key = TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 13.0, 17.0),
        200.0,
        false,
    );
    let mut resources = RenderResources::new();
    resources.register_text_layout(resource(id, key));
    let primitives = [
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        primitive(Some(id), "wrong"),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.5),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let run = encoding.resources.glyph_runs.first().expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_approx(run.font_size, 19.5);
    assert!(run.hint);
    assert!(
        encoding
            .resources
            .glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001
                && (glyph.y - glyph.y.round()).abs() <= 0.001)
    );
    assert_eq!(renderer.cached_text_layout_count(), 0);
}
