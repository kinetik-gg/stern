use std::sync::Arc;

use crate::{
    RenderCommandKind, RenderDiagnostic, RenderFrameInput, RenderResources, TextLayoutResource,
    VelloRenderer, project_text_point_to_device, root_transform, snap_axis_aligned_translation,
    snap_rect_to_device, transform_point, translate_primitives,
    translation::REGISTERED_TEXT_COMPATIBILITY_METRIC_PLACEHOLDER,
};
use stern_core::{
    Brush, Color, CornerRadius, PhysicalSize, Point, Primitive, Rect, RectPrimitive, ScaleFactor,
    Size, TextLayoutId, TextPrimitive, Transform, Vec2, ViewportInfo,
};
use stern_text::{CosmicTextEngine, ShapedGlyph, ShapedTextLayout, TextLayoutKey, TextStyle};
use vello::kurbo::{Affine, Point as KurboPoint};

const ORIGIN: Point = Point::new(4.3, 16.4);
const POSITION_EPSILON: f32 = 0.001;

struct Fixture {
    id: TextLayoutId,
    key: TextLayoutKey,
    layout: Arc<ShapedTextLayout>,
}

impl Fixture {
    fn new(id: u64, text: &str, width: f32, wrap: bool) -> Self {
        let key = TextLayoutKey::new(text, TextStyle::new("Inter", 18.0, 24.0), width, wrap);
        let mut engine = CosmicTextEngine::new();
        let layout = Arc::new(engine.shape_text(&key));
        Self {
            id: TextLayoutId::from_raw(id),
            key,
            layout,
        }
    }

    fn primitive(&self) -> Primitive {
        self.primitive_with_metrics(7.0, 9.0)
    }

    fn primitive_with_metrics(&self, size: f32, line_height: f32) -> Primitive {
        Primitive::Text(TextPrimitive {
            layout: Some(self.id),
            origin: ORIGIN,
            text: "conflicting compatibility text".to_owned(),
            family: "serif".to_owned(),
            size,
            line_height,
            brush: Brush::Solid(Color::WHITE),
        })
    }

    fn resources(&self) -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_text_layout(TextLayoutResource {
            id: self.id,
            key: self.key.clone(),
            layout: Arc::clone(&self.layout),
        });
        resources
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

fn scale_f32(scale: f64) -> f32 {
    match scale {
        1.25 => 1.25,
        1.5 => 1.5,
        1.75 => 1.75,
        _ => panic!("unsupported fixture scale {scale}"),
    }
}

fn f64_affine_projection(transform: Affine, point: Point) -> KurboPoint {
    transform * KurboPoint::new(f64::from(point.x), f64::from(point.y))
}

fn all_glyphs(layout: &ShapedTextLayout) -> Vec<ShapedGlyph> {
    layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().copied())
        .collect()
}

fn invalid_compatibility_metrics() -> [f32; 5] {
    [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0]
}

fn assert_registered_text_encoding_eq(actual: &VelloRenderer, expected: &VelloRenderer) {
    let actual = actual.scene().encoding();
    let expected = expected.scene().encoding();
    assert_eq!(
        actual.resources.glyph_runs.len(),
        expected.resources.glyph_runs.len()
    );
    assert_eq!(
        actual.resources.glyphs.len(),
        expected.resources.glyphs.len()
    );

    for (actual_run, expected_run) in actual
        .resources
        .glyph_runs
        .iter()
        .zip(&expected.resources.glyph_runs)
    {
        assert_eq!(actual_run.glyphs, expected_run.glyphs);
        assert_eq!(
            actual_run.font.data.as_ref(),
            expected_run.font.data.as_ref()
        );
        assert_eq!(actual_run.font.index, expected_run.font.index);
        assert_eq!(
            &actual.resources.normalized_coords[actual_run.normalized_coords.clone()],
            &expected.resources.normalized_coords[expected_run.normalized_coords.clone()]
        );
        assert_eq!(
            actual_run.font_size.to_bits(),
            expected_run.font_size.to_bits()
        );

        let actual_glyphs = &actual.resources.glyphs[actual_run.glyphs.clone()];
        let expected_glyphs = &expected.resources.glyphs[expected_run.glyphs.clone()];
        assert_eq!(actual_glyphs.len(), expected_glyphs.len());
        for (actual_glyph, expected_glyph) in actual_glyphs.iter().zip(expected_glyphs) {
            assert_eq!(actual_glyph.id, expected_glyph.id);
            assert_eq!(actual_glyph.x.to_bits(), expected_glyph.x.to_bits());
            assert_eq!(actual_glyph.y.to_bits(), expected_glyph.y.to_bits());
        }
    }
}

fn compatibility_text(layout: Option<TextLayoutId>, size: f32, line_height: f32) -> Primitive {
    Primitive::Text(TextPrimitive {
        layout,
        origin: ORIGIN,
        text: "fallback compatibility text".to_owned(),
        family: "serif".to_owned(),
        size,
        line_height,
        brush: Brush::Solid(Color::WHITE),
    })
}

fn assert_fixture_properties(fixtures: &[Fixture]) {
    let proportional = &fixtures[0].layout;
    let mut positive_widths = all_glyphs(proportional)
        .into_iter()
        .map(|glyph| glyph.width)
        .filter(|width| *width > 0.0)
        .map(f32::to_bits)
        .collect::<Vec<_>>();
    positive_widths.sort_unstable();
    positive_widths.dedup();
    assert!(
        positive_widths.len() >= 2,
        "proportional fixture must expose unequal positive advances"
    );

    let ligature = &fixtures[1];
    assert!(
        all_glyphs(&ligature.layout)
            .iter()
            .any(|glyph| glyph.start == 0 && glyph.end == ligature.key.text.len()),
        "the pinned Inter arrow fixture must contain a multi-grapheme cluster"
    );
    let ligature_navigation = ligature
        .layout
        .navigation(&ligature.key.text)
        .expect("ligature navigation");
    for offset in 0..=ligature.key.text.len() {
        assert!(
            ligature_navigation
                .caret_stops()
                .iter()
                .any(|stop| stop.caret.offset == offset),
            "ligature fixture is missing navigation offset {offset}"
        );
    }

    let unicode = &fixtures[2];
    let unicode_navigation = unicode
        .layout
        .navigation(&unicode.key.text)
        .expect("Unicode navigation");
    let mut unicode_offsets = unicode_navigation
        .caret_stops()
        .iter()
        .map(|stop| stop.caret.offset)
        .collect::<Vec<_>>();
    unicode_offsets.sort_unstable();
    unicode_offsets.dedup();
    assert_eq!(
        unicode_offsets,
        vec![0, 1, 4, 5, 6, 14, 15, 23, 24, 25, 36, 37]
    );
    for grapheme in ["e\u{301}", "👍🏽", "🇮🇩", "👩‍🚀"] {
        let start = unicode.key.text.find(grapheme).expect("fixture grapheme");
        let end = start + grapheme.len();
        assert!(
            unicode_navigation
                .caret_stops()
                .iter()
                .all(|stop| stop.caret.offset <= start || stop.caret.offset >= end),
            "extended grapheme {grapheme:?} must remain atomic"
        );
    }

    let bidi = &fixtures[3].layout;
    assert_eq!(bidi.line_count, 1);
    let bidi_glyphs = all_glyphs(bidi);
    assert!(bidi_glyphs.iter().any(|glyph| glyph.rtl));
    assert!(bidi_glyphs.iter().any(|glyph| !glyph.rtl));
    let mut rtl = bidi_glyphs
        .iter()
        .filter(|glyph| glyph.rtl)
        .copied()
        .collect::<Vec<_>>();
    rtl.sort_by(|left, right| left.x.total_cmp(&right.x));
    assert!(rtl.len() >= 2);
    assert!(
        rtl.windows(2).all(|pair| pair[0].start > pair[1].start),
        "RTL glyphs ordered by increasing visual x must descend in source offsets"
    );

    let wrapped = &fixtures[4];
    assert!(wrapped.layout.line_count > 1);
    let navigation = wrapped
        .layout
        .navigation(&wrapped.key.text)
        .expect("wrapped navigation");
    assert!(
        navigation
            .caret_stops()
            .iter()
            .enumerate()
            .any(|(index, left)| {
                navigation.caret_stops()[index + 1..].iter().any(|right| {
                    left.caret.offset == right.caret.offset
                        && left.caret.affinity != right.caret.affinity
                        && left.visual_line != right.visual_line
                })
            }),
        "wrapped fixture must expose an affinity-distinct visual seam"
    );
}

fn assert_registered_encoding(fixture: &Fixture, scale: f64) {
    let resources = fixture.resources();
    assert!(Arc::ptr_eq(
        &resources
            .text_layout_resource(fixture.id)
            .expect("registered layout")
            .layout,
        &fixture.layout
    ));
    let primitives = [fixture.primitive()];
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(scale),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 0);
    assert_eq!(
        encoding.resources.glyph_runs.len(),
        fixture.layout.runs.len()
    );

    let effective = snap_axis_aligned_translation(root_transform(scale));
    let mut glyph_cursor = 0;
    for (encoded_run, logical_run) in encoding
        .resources
        .glyph_runs
        .iter()
        .zip(&fixture.layout.runs)
    {
        assert_eq!(encoded_run.glyphs.start, glyph_cursor);
        glyph_cursor += logical_run.glyphs.len();
        assert_eq!(encoded_run.glyphs.end, glyph_cursor);
        assert_eq!(encoded_run.font.data.as_ref(), logical_run.font.data.data());
        assert_eq!(encoded_run.font.index, logical_run.font.index);
        assert_eq!(
            encoded_run.font_size.to_bits(),
            (logical_run.font_size * scale_f32(scale)).to_bits()
        );
        let encoded_glyphs = &encoding.resources.glyphs[encoded_run.glyphs.clone()];
        assert_eq!(encoded_glyphs.len(), logical_run.glyphs.len());
        for (encoded, logical) in encoded_glyphs.iter().zip(&logical_run.glyphs) {
            let logical_point = Point::new(ORIGIN.x + logical.x, ORIGIN.y + logical.y);
            let unrounded = f64_affine_projection(effective, logical_point);
            assert_eq!(encoded.id, logical.id);
            assert!(
                (f64::from(encoded.x) - unrounded.x.round()).abs() <= f64::from(POSITION_EPSILON)
            );
            assert!(
                (f64::from(encoded.y) - unrounded.y.round()).abs() <= f64::from(POSITION_EPSILON)
            );
            assert!((unrounded.x - f64::from(encoded.x)).abs() <= 0.500_1);
            assert!((unrounded.y - f64::from(encoded.y)).abs() <= 0.500_1);
        }
    }
    assert_eq!(glyph_cursor, encoding.resources.glyphs.len());
}

#[test]
fn text_authority_registered_fixture_matrix_preserves_topology_at_fractional_scales() {
    let fixtures = [
        Fixture::new(801, "Hamburgefontsiv AVATAR", 400.0, false),
        Fixture::new(802, "->", 400.0, false),
        Fixture::new(803, "Ae\u{301}B 👍🏽 🇮🇩 A👩‍🚀B", 400.0, false),
        Fixture::new(804, "abc אבג def", 400.0, false),
        Fixture::new(805, "abc אבג", 40.0, true),
    ];
    assert_fixture_properties(&fixtures);

    for fixture in &fixtures {
        for scale in [1.25, 1.5, 1.75] {
            assert_registered_encoding(fixture, scale);
        }
    }
}

#[test]
fn explicit_regular_weight_matches_constructor_default_encoding() {
    let default_key = TextLayoutKey::new(
        "Regular weight equivalence",
        TextStyle::new("Inter", 18.0, 24.0),
        400.0,
        false,
    );
    let explicit_key = TextLayoutKey::new(
        "Regular weight equivalence",
        TextStyle::new("Inter", 18.0, 24.0).with_weight(400),
        400.0,
        false,
    );
    let mut engine = CosmicTextEngine::new();
    let default = Fixture {
        id: TextLayoutId::from_raw(726),
        layout: Arc::new(engine.shape_text(&default_key)),
        key: default_key,
    };
    let explicit = Fixture {
        id: default.id,
        layout: Arc::new(engine.shape_text(&explicit_key)),
        key: explicit_key,
    };
    assert_eq!(default.key, explicit.key);
    assert_eq!(default.layout, explicit.layout);

    let default_resources = default.resources();
    let explicit_resources = explicit.resources();
    let primitives = [default.primitive()];
    let mut default_renderer = VelloRenderer::new();
    let default_output = default_renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &default_resources,
    });
    let mut explicit_renderer = VelloRenderer::new();
    let explicit_output = explicit_renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &explicit_resources,
    });

    assert!(default_output.diagnostics.is_empty());
    assert!(explicit_output.diagnostics.is_empty());
    assert_registered_text_encoding_eq(&default_renderer, &explicit_renderer);
    assert_eq!(default_renderer.cached_text_layout_count(), 0);
    assert_eq!(explicit_renderer.cached_text_layout_count(), 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn text_authority_registered_layout_ignores_invalid_compatibility_metrics() {
    const VALID_SIZE: f32 = 7.0;
    const VALID_LINE_HEIGHT: f32 = 9.0;

    let fixture = Fixture::new(808, "Registered authority", 400.0, false);
    let resources = fixture.resources();
    let control = [fixture.primitive_with_metrics(VALID_SIZE, VALID_LINE_HEIGHT)];
    let mut control_renderer = VelloRenderer::new();
    let control_output = control_renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.75),
        primitives: &control,
        resources: &resources,
    });
    assert!(control_output.diagnostics.is_empty());
    assert_eq!(control_renderer.cached_text_layout_count(), 0);
    assert_eq!(control_renderer.cached_text_layout_payload_bytes(), 0);

    for invalid in invalid_compatibility_metrics() {
        for (size, line_height, expected_size, expected_line_height) in [
            (
                invalid,
                VALID_LINE_HEIGHT,
                REGISTERED_TEXT_COMPATIBILITY_METRIC_PLACEHOLDER,
                VALID_LINE_HEIGHT,
            ),
            (
                VALID_SIZE,
                invalid,
                VALID_SIZE,
                REGISTERED_TEXT_COMPATIBILITY_METRIC_PLACEHOLDER,
            ),
        ] {
            let primitives = [fixture.primitive_with_metrics(size, line_height)];
            let translation = translate_primitives(&primitives, &resources);
            assert!(translation.diagnostics.is_empty(), "invalid={invalid:?}");
            let [command] = translation.commands.as_slice() else {
                panic!("registered invalid metadata must emit exactly one command");
            };
            let RenderCommandKind::Text {
                layout,
                size: command_size,
                line_height: command_line_height,
                ..
            } = &command.kind
            else {
                panic!("registered invalid metadata must emit a text command");
            };
            assert_eq!(*layout, Some(fixture.id));
            assert!(command_size.is_finite() && *command_size > 0.0);
            assert!(command_line_height.is_finite() && *command_line_height > 0.0);
            assert_eq!(command_size.to_bits(), expected_size.to_bits());
            assert_eq!(
                command_line_height.to_bits(),
                expected_line_height.to_bits()
            );

            let mut renderer = VelloRenderer::new();
            let output = renderer.submit_frame(RenderFrameInput {
                viewport: viewport(1.75),
                primitives: &primitives,
                resources: &resources,
            });
            assert!(output.diagnostics.is_empty(), "invalid={invalid:?}");
            assert_registered_text_encoding_eq(&renderer, &control_renderer);
            assert_eq!(renderer.cached_text_layout_count(), 0);
            assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
        }
    }
}

#[test]
fn text_authority_fallback_paths_reject_invalid_compatibility_metrics_in_order() {
    const VALID_SIZE: f32 = 7.0;
    const VALID_LINE_HEIGHT: f32 = 9.0;

    let resources = RenderResources::new();
    let missing = TextLayoutId::from_raw(809);
    for invalid in invalid_compatibility_metrics() {
        for (size, line_height, diagnostic) in [
            (
                invalid,
                VALID_LINE_HEIGHT,
                RenderDiagnostic::InvalidGeometry("text_size"),
            ),
            (
                VALID_SIZE,
                invalid,
                RenderDiagnostic::InvalidGeometry("text_line_height"),
            ),
        ] {
            let layoutless = [compatibility_text(None, size, line_height)];
            let translation = translate_primitives(&layoutless, &resources);
            assert!(translation.commands.is_empty(), "invalid={invalid:?}");
            assert_eq!(translation.diagnostics, vec![diagnostic.clone()]);

            let missing_resource = [compatibility_text(Some(missing), size, line_height)];
            let translation = translate_primitives(&missing_resource, &resources);
            assert!(translation.commands.is_empty(), "invalid={invalid:?}");
            assert_eq!(
                translation.diagnostics,
                vec![RenderDiagnostic::MissingTextLayout(missing), diagnostic]
            );
        }
    }
}

#[test]
fn text_authority_projection_rounds_in_f64_before_narrowing_to_vello_storage() {
    let logical_x = 2.8_f32;
    let unrounded = f64::from(logical_x) * 1.25;
    assert!(unrounded < 3.5);
    assert_eq!(unrounded.round().to_bits(), 3.0_f64.to_bits());

    let encoded = project_text_point_to_device(Affine::scale(1.25), Point::new(logical_x, 0.0));
    assert_eq!(encoded.x.to_bits(), 3.0_f32.to_bits());
}

fn translated_rect(rect: Rect) -> Rect {
    Rect::new(
        ORIGIN.x + rect.x,
        ORIGIN.y + rect.y,
        rect.width,
        rect.height,
    )
}

fn production_rect_device_edges(rect: Rect, scale: f64) -> (Point, Point) {
    let prepared = snap_rect_to_device(translated_rect(rect), scale);
    let effective = snap_axis_aligned_translation(root_transform(scale));
    (
        transform_point(effective, Point::new(prepared.min_x(), prepared.min_y())),
        transform_point(effective, Point::new(prepared.max_x(), prepared.max_y())),
    )
}

fn encoded_fill_rect_device_bounds(rect: Rect, scale: f64, command_transform: Transform) -> Rect {
    let primitives = [
        Primitive::TransformBegin(command_transform),
        Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::TransformEnd,
    ];
    let resources = RenderResources::new();
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(scale),
        primitives: &primitives,
        resources: &resources,
    });
    assert!(output.diagnostics.is_empty());

    let encoding = renderer.scene().encoding();
    assert_eq!(encoding.n_paths, 1);
    assert_eq!(encoding.transforms.len(), 1);
    assert!(!encoding.path_data.is_empty());
    let transform = encoding.transforms[0];
    let mut min = Point::new(f32::INFINITY, f32::INFINITY);
    let mut max = Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut words = encoding.path_data.chunks_exact(2);
    for point_words in &mut words {
        let x = f32::from_bits(point_words[0]);
        let y = f32::from_bits(point_words[1]);
        let point = Point::new(
            transform.matrix[0]
                .mul_add(x, transform.matrix[2].mul_add(y, transform.translation[0])),
            transform.matrix[1]
                .mul_add(x, transform.matrix[3].mul_add(y, transform.translation[1])),
        );
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    assert!(words.remainder().is_empty());
    assert!(min.x.is_finite() && min.y.is_finite());
    assert!(max.x.is_finite() && max.y.is_finite());
    Rect::from_min_max(min, max)
}

fn assert_rect_matches_points(rect: Rect, min: Point, max: Point, tolerance: f32) {
    assert!((rect.min_x() - min.x).abs() <= tolerance);
    assert!((rect.min_y() - min.y).abs() <= tolerance);
    assert!((rect.max_x() - max.x).abs() <= tolerance);
    assert!((rect.max_y() - max.y).abs() <= tolerance);
}

fn production_translated_rect_device_edges(
    rect: Rect,
    scale: f64,
    translation: Vec2,
) -> (Point, Point) {
    let prepared = snap_rect_to_device(translated_rect(rect), scale);
    let raw = root_transform(scale)
        * Affine::translate((f64::from(translation.x), f64::from(translation.y)));
    let effective = snap_axis_aligned_translation(raw);
    (
        transform_point(effective, Point::new(prepared.min_x(), prepared.min_y())),
        transform_point(effective, Point::new(prepared.max_x(), prepared.max_y())),
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn text_authority_navigation_rects_and_hits_share_fractional_device_edges() {
    let fixture = Fixture::new(806, "Alpha beta", 400.0, false);
    let navigation = fixture
        .layout
        .navigation(&fixture.key.text)
        .expect("navigation");
    let first_run = fixture.layout.runs.first().expect("glyph run");
    let first = first_run.glyphs.first().expect("first glyph");
    let second = first_run.glyphs.get(1).expect("second glyph");
    assert_eq!(first.end, second.start);
    let first_stop = navigation
        .caret_stops()
        .iter()
        .find(|stop| stop.caret.offset == first.start)
        .expect("first caret stop");
    let second_stop = navigation
        .caret_stops()
        .iter()
        .find(|stop| stop.caret.offset == second.start)
        .expect("second caret stop");
    assert!((first_stop.x - first.x).abs() <= POSITION_EPSILON);
    assert!((second_stop.x - second.x).abs() <= POSITION_EPSILON);

    for scale in [1.25, 1.5, 1.75] {
        let resources = fixture.resources();
        let primitives = [fixture.primitive()];
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_run = encoding.resources.glyph_runs.first().expect("encoded run");
        let encoded = &encoding.resources.glyphs[encoded_run.glyphs.clone()];
        let left_local = navigation.caret_rect(first_stop.caret);
        let right_local = navigation.caret_rect(second_stop.caret);
        let selection = navigation
            .selection_rects(first.start..first.end)
            .into_iter()
            .next()
            .expect("selection rectangle");
        let left_bounds = encoded_fill_rect_device_bounds(
            translated_rect(left_local),
            scale,
            Transform::IDENTITY,
        );
        let right_bounds = encoded_fill_rect_device_bounds(
            translated_rect(right_local),
            scale,
            Transform::IDENTITY,
        );
        let selection_bounds =
            encoded_fill_rect_device_bounds(translated_rect(selection), scale, Transform::IDENTITY);
        let (left_expected_min, left_expected_max) =
            production_rect_device_edges(left_local, scale);
        let (right_expected_min, right_expected_max) =
            production_rect_device_edges(right_local, scale);
        let (selection_expected_min, selection_expected_max) =
            production_rect_device_edges(selection, scale);

        assert!(output.diagnostics.is_empty());
        assert_rect_matches_points(
            left_bounds,
            left_expected_min,
            left_expected_max,
            POSITION_EPSILON,
        );
        assert_rect_matches_points(
            right_bounds,
            right_expected_min,
            right_expected_max,
            POSITION_EPSILON,
        );
        assert_rect_matches_points(
            selection_bounds,
            selection_expected_min,
            selection_expected_max,
            POSITION_EPSILON,
        );
        assert!((selection_bounds.min_x() - left_bounds.min_x()).abs() <= POSITION_EPSILON);
        assert!((selection_bounds.max_x() - right_bounds.min_x()).abs() <= POSITION_EPSILON);
        assert!((encoded[0].x - left_bounds.min_x()).abs() <= POSITION_EPSILON);
        assert!((encoded[0].x - selection_bounds.min_x()).abs() <= POSITION_EPSILON);
        assert!((encoded[1].x - right_bounds.min_x()).abs() <= POSITION_EPSILON);
        assert!((encoded[1].x - selection_bounds.max_x()).abs() <= POSITION_EPSILON);
        assert_eq!(renderer.cached_text_layout_count(), 0);

        let left = *first_stop;
        let right = *second_stop;
        let threshold = (left_bounds.min_x() + right_bounds.min_x()) * 0.5;
        let delta = (right_bounds.min_x() - left_bounds.min_x()) * 0.25;
        assert!(delta * 4.0 >= 4.0);
        let physical_y = selection_bounds.center().y;
        let effective = snap_axis_aligned_translation(root_transform(scale));
        let left_ui = transform_point(
            effective.inverse(),
            Point::new(threshold - delta, physical_y),
        );
        let right_ui = transform_point(
            effective.inverse(),
            Point::new(threshold + delta, physical_y),
        );
        assert_eq!(
            navigation.hit_test_caret(left_ui.x - ORIGIN.x, left_ui.y - ORIGIN.y),
            left.caret
        );
        assert_eq!(
            navigation.hit_test_caret(right_ui.x - ORIGIN.x, right_ui.y - ORIGIN.y),
            right.caret
        );
    }
}

#[test]
fn text_authority_fractional_translation_stays_within_generic_rect_quantization_band() {
    let fixture = Fixture::new(807, "Alpha beta", 400.0, false);
    let navigation = fixture
        .layout
        .navigation(&fixture.key.text)
        .expect("navigation");
    let glyphs = &fixture.layout.runs.first().expect("glyph run").glyphs;
    let first = glyphs.first().expect("first glyph");
    let second = glyphs.get(1).expect("second glyph");
    let second_stop = navigation
        .caret_stops()
        .iter()
        .find(|stop| stop.caret.offset == second.start)
        .expect("second caret stop");
    let selection = navigation
        .selection_rects(first.start..first.end)
        .into_iter()
        .next()
        .expect("selection rectangle");
    let translation = Vec2::new(0.37, 0.29);
    let command = Transform::translation(translation);
    let expected_ids = all_glyphs(&fixture.layout)
        .into_iter()
        .map(|glyph| glyph.id)
        .collect::<Vec<_>>();

    for scale in [1.25, 1.5, 1.75] {
        let resources = fixture.resources();
        let primitives = [
            Primitive::TransformBegin(command),
            fixture.primitive(),
            Primitive::TransformEnd,
        ];
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded = &encoding.resources.glyphs;
        let caret = navigation.caret_rect(second_stop.caret);
        let caret_bounds = encoded_fill_rect_device_bounds(translated_rect(caret), scale, command);
        let selection_bounds =
            encoded_fill_rect_device_bounds(translated_rect(selection), scale, command);
        let (caret_expected_min, caret_expected_max) =
            production_translated_rect_device_edges(caret, scale, translation);
        let (selection_expected_min, selection_expected_max) =
            production_translated_rect_device_edges(selection, scale, translation);

        assert!(output.diagnostics.is_empty());
        assert_eq!(
            encoded.iter().map(|glyph| glyph.id).collect::<Vec<_>>(),
            expected_ids
        );
        assert_rect_matches_points(
            caret_bounds,
            caret_expected_min,
            caret_expected_max,
            POSITION_EPSILON,
        );
        assert_rect_matches_points(
            selection_bounds,
            selection_expected_min,
            selection_expected_max,
            POSITION_EPSILON,
        );
        assert!((encoded[0].x - selection_bounds.min_x()).abs() <= 1.000_1);
        assert!((encoded[1].x - caret_bounds.min_x()).abs() <= 1.000_1);
        assert!((encoded[1].x - selection_bounds.max_x()).abs() <= 1.000_1);
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert!(Arc::ptr_eq(
            &resources
                .text_layout_resource(fixture.id)
                .expect("registered layout")
                .layout,
            &fixture.layout
        ));
    }
}
