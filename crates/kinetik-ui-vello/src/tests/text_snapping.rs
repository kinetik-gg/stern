use super::common::{assert_approx, text_layout_resource};
use crate::{
    RenderFrameInput, RenderResources, VelloRenderer, exact_positive_axis_aligned_scale,
    project_text_point_to_device, root_transform, snap_axis_aligned_translation, transform_point,
};
use kinetik_ui_core::{
    Brush, Color, PhysicalSize, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive,
    Transform, Vec2, ViewportInfo,
};
use kinetik_ui_text::{CosmicTextEngine, TextLayoutKey, TextStyle};
use vello::kurbo::Affine;

fn viewport(scale: f64) -> ViewportInfo {
    let physical = match scale {
        1.0 => 100,
        1.25 => 125,
        1.5 => 150,
        1.75 => 175,
        2.0 => 200,
        _ => panic!("unsupported fixture scale {scale}"),
    };
    ViewportInfo::new(
        Size::new(100.0, 100.0),
        PhysicalSize::new(physical, physical),
        ScaleFactor::new(scale),
    )
}

fn scale_f32(scale: f64) -> f32 {
    match scale {
        1.0 => 1.0,
        1.25 => 1.25,
        1.5 => 1.5,
        1.75 => 1.75,
        2.0 => 2.0,
        _ => panic!("unsupported fixture scale {scale}"),
    }
}

fn text(layout: Option<TextLayoutId>, value: &str, size: f32) -> Primitive {
    Primitive::Text(TextPrimitive {
        layout,
        origin: Point::new(4.3, 16.4),
        text: value.to_owned(),
        family: "sans-serif".to_owned(),
        size,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })
}

#[test]
fn text_projection_rounds_one_absolute_point_instead_of_origin_and_offset() {
    let transform = Affine::IDENTITY;
    let absolute = project_text_point_to_device(transform, Point::new(0.6, 0.6));
    let origin_first = Point::new(0.4_f32.round() + 0.2_f32.round(), 0.4_f32.round());

    assert_eq!(absolute, Point::new(1.0, 1.0));
    assert_eq!(origin_first, Point::new(0.0, 0.0));
}

#[test]
fn text_axis_classification_is_exact_and_rejects_any_skew() {
    assert_eq!(
        exact_positive_axis_aligned_scale(Affine::scale_non_uniform(1.25, 1.5)),
        Some((1.25, 1.5))
    );
    assert_eq!(
        exact_positive_axis_aligned_scale(Affine::new([1.25, 0.0, 0.0, 1.249_99, 0.0, 0.0])),
        Some((1.25, 1.249_99))
    );
    assert!(
        exact_positive_axis_aligned_scale(Affine::new([1.25, 0.000_001, 0.0, 1.25, 0.0, 0.0,]))
            .is_none()
    );
    assert!(exact_positive_axis_aligned_scale(Affine::scale_non_uniform(-1.0, 1.0)).is_none());
}

#[test]
fn logical_fallback_policy_holds_across_fractional_dpi_scales() {
    let resources = RenderResources::new();
    let primitives = [text(None, "Kinetik", 13.0)];

    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let run = encoding.resources.glyph_runs.first().expect("glyph run");

        assert!(output.diagnostics.is_empty());
        assert!((run.font_size - 13.0 * scale_f32(scale)).abs() <= 0.000_1);
        assert!(run.hint);
        assert!(
            encoding
                .resources
                .glyphs
                .iter()
                .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001
                    && (glyph.y - glyph.y.round()).abs() <= 0.001)
        );
        assert_eq!(renderer.cached_text_layout_count(), 1);
    }
}

#[test]
fn fallback_uses_uniform_framebuffer_scale_when_declared_scale_is_stale() {
    let resources = RenderResources::new();
    let primitives = [text(None, "Kinetik", 13.0)];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            PhysicalSize::new(125, 125),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
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
}

#[test]
fn translated_registered_text_projects_absolute_positions_from_shared_transform() {
    let id = TextLayoutId::from_raw(70);
    let resource = text_layout_resource(id, "Kinetik");
    let first = resource.layout.runs[0].glyphs[0];
    let mut resources = RenderResources::new();
    resources.register_text_layout(resource);
    let primitives = [
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        text(Some(id), "ignored", 7.0),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph = renderer
        .scene()
        .encoding()
        .resources
        .glyphs
        .first()
        .expect("glyph");
    let effective =
        snap_axis_aligned_translation(root_transform(1.25) * Affine::translate((2.2_f64, 3.4_f64)));
    let expected =
        project_text_point_to_device(effective, Point::new(4.3 + first.x, 16.4 + first.y));

    assert!(output.diagnostics.is_empty());
    assert_eq!(Point::new(glyph.x, glyph.y), expected);
    assert_eq!(renderer.cached_text_layout_count(), 0);
}

#[test]
fn exact_non_uniform_registered_text_uses_exact_outline_ratio() {
    let id = TextLayoutId::from_raw(71);
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "Kinetik",
        TextStyle::new("sans-serif", 13.0, 18.0),
        0.0,
        false,
    );
    let layout = engine.shape_text(&key);
    let mut resources = RenderResources::new();
    resources.register_text_layout(crate::TextLayoutResource {
        id,
        key,
        layout: std::sync::Arc::new(layout),
    });
    let primitives = [
        Primitive::TransformBegin(Transform {
            m11: 1.25,
            m22: 1.5,
            dx: 2.2,
            dy: 3.4,
            ..Transform::IDENTITY
        }),
        text(Some(id), "ignored", 7.0),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.0),
        primitives: &primitives,
        resources: &resources,
    });
    let run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");
    let glyph_transform = run.glyph_transform.expect("non-uniform outline transform");

    assert!(output.diagnostics.is_empty());
    assert_approx(run.font_size, 19.5);
    assert!((glyph_transform.matrix[0] - 1.25 / 1.5).abs() <= 0.000_1);
    assert!(run.hint);
    assert_eq!(renderer.cached_text_layout_count(), 0);
}

#[test]
fn rotated_and_reflected_registered_text_use_unsnapped_general_affines() {
    let id = TextLayoutId::from_raw(72);
    let resource = text_layout_resource(id, "Kinetik");
    let expected = resource.layout.runs[0].glyphs[0];
    let mut resources = RenderResources::new();
    resources.register_text_layout(resource);

    for transform in [
        Transform {
            m11: 0.5_f32.cos(),
            m12: 0.5_f32.sin(),
            m21: -0.5_f32.sin(),
            m22: 0.5_f32.cos(),
            dx: 2.2,
            dy: 3.4,
        },
        Transform {
            m11: -1.0,
            m22: 1.0,
            dx: 20.2,
            dy: 3.4,
            ..Transform::IDENTITY
        },
    ] {
        let primitives = [
            Primitive::TransformBegin(transform),
            text(Some(id), "ignored", 7.0),
            Primitive::TransformEnd,
        ];
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(1.25),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let run = encoding.resources.glyph_runs.first().expect("glyph run");
        let glyph = encoding.resources.glyphs.first().expect("glyph");
        let mapped = run.transform.to_kurbo()
            * vello::kurbo::Point::new(f64::from(glyph.x), f64::from(glyph.y));
        let raw = root_transform(1.25)
            * Affine::new([
                f64::from(transform.m11),
                f64::from(transform.m12),
                f64::from(transform.m21),
                f64::from(transform.m22),
                f64::from(transform.dx),
                f64::from(transform.dy),
            ]);
        let logical = transform_point(raw, Point::new(glyph.x, glyph.y));
        let expected_logical = Point::new(4.3 + expected.x, 16.4 + expected.y);
        let expected_device = transform_point(raw, expected_logical);

        assert!(output.diagnostics.is_empty());
        assert!(!run.hint);
        assert_eq!(glyph.id, expected.id);
        assert_approx(glyph.x, expected_logical.x);
        assert_approx(glyph.y, expected_logical.y);
        assert_eq!(logical, expected_device);
        assert!((mapped.x - f64::from(expected_device.x)).abs() <= 0.001);
        assert!((mapped.y - f64::from(expected_device.y)).abs() <= 0.001);
        assert_eq!(renderer.cached_text_layout_count(), 0);
    }
}
