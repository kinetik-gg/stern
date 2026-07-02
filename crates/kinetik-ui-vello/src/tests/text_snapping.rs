use super::common::{assert_approx, shaped_glyph_x_positions};
use crate::{
    RenderFrameInput, RenderResources, ShapedTextCache, VelloRenderer, physical_text_layout,
    root_transform, snap_text_glyph_baseline_to_device, snap_text_glyph_position_to_device,
    snap_text_origin_to_device, snap_text_transform_origin_to_device, transform_point,
};
use kinetik_ui_core::{
    Brush, Color, Point, Primitive, ScaleFactor, Size, TextPrimitive, Transform, Vec2, ViewportInfo,
};
use kinetik_ui_text::{CosmicTextEngine, TextLayoutKey, TextStyle};
use vello::kurbo::{Affine, Point as KurboPoint};

#[test]
fn text_origin_snapping_rounds_x_and_baseline_y() {
    let origin = snap_text_origin_to_device(Point::new(5.375, 20.5));

    assert_approx(origin.x, 5.0);
    assert_approx(origin.y, 21.0);
}

#[test]
fn text_glyph_baseline_snapping_rounds_device_coordinates() {
    assert_approx(snap_text_glyph_baseline_to_device(11.49), 11.0);
    assert_approx(snap_text_glyph_baseline_to_device(11.5), 12.0);
}

#[test]
fn text_transform_origin_snapping_happens_in_device_space_for_non_uniform_scale() {
    let transform = root_transform(1.25) * Affine::scale_non_uniform(1.5, 1.0);
    let origin = Point::new(4.3, 16.4);

    let snapped = snap_text_transform_origin_to_device(transform, origin);
    let device_origin = transform_point(snapped, origin);

    assert_approx(device_origin.x, 8.0);
    assert_approx(device_origin.y, 21.0);
}

#[test]
fn text_transform_origin_snapping_happens_in_device_space_for_rotation() {
    let transform = root_transform(1.25) * Affine::rotate(0.5);
    let origin = Point::new(4.3, 16.4);

    let snapped = snap_text_transform_origin_to_device(transform, origin);
    let device_origin = transform_point(snapped, origin);

    assert!((device_origin.x - device_origin.x.round()).abs() <= 0.001);
    assert!((device_origin.y - device_origin.y.round()).abs() <= 0.001);
}

#[test]
fn physical_text_snaps_horizontal_origin_and_baseline() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
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

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph.x, 5.0);
    assert_approx(glyph.y, 21.0);
}

#[test]
fn physical_text_snaps_shaped_horizontal_glyph_positions() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let mut text_engine = CosmicTextEngine::new();
    let mut text_cache = ShapedTextCache::default();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyphs = &renderer.scene().encoding().resources.glyphs;
    let layout = physical_text_layout(
        &mut text_engine,
        &mut text_cache,
        root_transform(1.25),
        "Kinetik",
        "sans-serif",
        13.0,
        18.0,
    )
    .expect("axis-aligned physical layout");
    let expected_x = shaped_glyph_x_positions(&layout, 5.0, 1.0);

    assert!(output.diagnostics.is_empty());
    assert_eq!(glyphs.len(), expected_x.len());
    for (glyph, expected) in glyphs.iter().zip(expected_x) {
        assert_approx(glyph.x, expected.round());
    }
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "glyph x positions should stay snapped to physical pixels"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "baselines should stay snapped to physical pixels"
    );
}

#[test]
fn physical_text_policy_holds_across_common_dpi_scales() {
    let resources = RenderResources::new();
    let origin = Point::new(4.3, 16.4);
    let font_size = 13.0;
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin,
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: font_size,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    for (scale, physical_size, expected_font_size, expected_x) in [
        (1.0, 100, 13.0, 4.0),
        (1.25, 125, 16.0, 5.0),
        (1.5, 150, 20.0, 6.0),
        (2.0, 200, 26.0, 9.0),
    ] {
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(physical_size, physical_size),
                ScaleFactor::new(scale),
            ),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let glyphs = &encoding.resources.glyphs;
        let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
        let first_glyph = glyphs.first().expect("glyph");

        assert!(output.diagnostics.is_empty());
        assert_approx(glyph_run.font_size, expected_font_size);
        assert!(glyph_run.hint);
        assert_approx(first_glyph.x, expected_x);
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
            "scale {scale} should snap glyph x positions"
        );
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
            "scale {scale} should snap glyph baselines"
        );
    }
}

#[test]
fn physical_text_uses_uniform_framebuffer_scale_when_declared_scale_is_stale() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.0),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 16.0);
    assert!(glyph_run.hint);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "framebuffer-derived scale should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "framebuffer-derived scale should snap glyph baselines"
    );
}

#[test]
fn translated_physical_text_stays_snapped_at_fractional_dpi() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.3, 16.4),
            text: "Kinetik".to_owned(),
            family: "sans-serif".to_owned(),
            size: 13.0,
            line_height: 18.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(150, 150),
            ScaleFactor::new(1.5),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 20.0);
    assert!(glyph_run.hint);
    assert!(glyphs.len() > 1);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "translated text should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "translated text should snap glyph baselines"
    );
}

#[test]
fn axis_aligned_non_uniform_text_preserves_x_scale_with_glyph_transform() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let text = "Kinetik";
    let origin = Point::new(4.3, 16.4);
    let font_size = 13.0;
    let line_height = 18.0;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.25,
            m22: 1.5,
            dx: 2.2,
            dy: 3.4,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin,
            text: text.to_owned(),
            family: "sans-serif".to_owned(),
            size: font_size,
            line_height,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
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
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 20.0);
    assert!(glyph_run.hint);
    assert_approx(glyph_run.transform.matrix[0], 1.0);
    assert_approx(glyph_run.transform.matrix[1], 0.0);
    assert_approx(glyph_run.transform.matrix[2], 0.0);
    assert_approx(glyph_run.transform.matrix[3], 1.0);
    assert_approx(glyph_run.transform.translation[0], 0.0);
    assert_approx(glyph_run.transform.translation[1], 0.0);
    let glyph_transform = glyph_run.glyph_transform.expect("x glyph transform");
    assert_approx(glyph_transform.matrix[0], 0.8125);
    assert_approx(glyph_transform.matrix[1], 0.0);
    assert_approx(glyph_transform.matrix[2], 0.0);
    assert_approx(glyph_transform.matrix[3], 1.0);
    assert!(glyphs.len() > 1);
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&TextLayoutKey::new(
        text,
        TextStyle::new("sans-serif", font_size, line_height),
        0.0,
        false,
    ));
    let logical_second_glyph = layout
        .runs
        .first()
        .and_then(|run| run.glyphs.iter().find(|glyph| glyph.x > 0.0))
        .expect("second logical glyph");
    let encoded_second_glyph = glyphs
        .iter()
        .find(|glyph| glyph.x > glyphs[0].x)
        .expect("second encoded glyph");
    let snapped_origin =
        snap_text_origin_to_device(Point::new(2.0 + origin.x * 1.25, 3.0 + origin.y * 1.5));
    assert_approx(
        encoded_second_glyph.x,
        snap_text_glyph_position_to_device(snapped_origin.x + logical_second_glyph.x * 1.25),
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "non-uniform text should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "non-uniform text should snap glyph baselines"
    );
}

#[test]
fn rotated_text_fallback_snaps_transformed_origin_to_device_pixels() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let angle = 0.5_f32;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: angle.cos(),
            m12: angle.sin(),
            m21: -angle.sin(),
            m22: angle.cos(),
            dx: 2.2,
            dy: 3.4,
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.3, 16.4),
            text: "Kinetik".to_owned(),
            family: "sans-serif".to_owned(),
            size: 13.0,
            line_height: 18.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];

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
    let glyph = encoding.resources.glyphs.first().expect("glyph");
    let mapped =
        glyph_run.transform.to_kurbo() * KurboPoint::new(f64::from(glyph.x), f64::from(glyph.y));

    assert!(output.diagnostics.is_empty());
    assert!(!glyph_run.hint);
    assert!((mapped.x - mapped.x.round()).abs() <= 0.001);
    assert!((mapped.y - mapped.y.round()).abs() <= 0.001);
}
