//! Public conformance for non-destructive single-line end ellipsis.

use stern_text::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextOverflow, TextStyle};

fn key(text: impl Into<String>, width: f32, wrap: bool) -> TextLayoutKey {
    TextLayoutKey::new(text, TextStyle::new("Inter", 18.0, 24.0), width, wrap)
}

fn shape(request: &TextLayoutKey) -> ShapedTextLayout {
    CosmicTextEngine::new().shape_text(request)
}

fn shape_pair(
    first: &TextLayoutKey,
    second: &TextLayoutKey,
) -> (ShapedTextLayout, ShapedTextLayout) {
    let mut engine = CosmicTextEngine::new();
    (engine.shape_text(first), engine.shape_text(second))
}

#[test]
fn visible_is_the_default_and_preserves_existing_topology() {
    let default = key("Default visible e\u{301} 👩‍🚀 العربية", 72.0, false);
    let explicit = default.clone().with_overflow(TextOverflow::Visible);

    assert_eq!(default.overflow, TextOverflow::Visible);
    let (default_layout, explicit_layout) = shape_pair(&default, &explicit);
    assert_eq!(default_layout, explicit_layout);
    assert!(!shape(&default).is_elided());
}

#[test]
fn eligible_long_source_is_elided_without_changing_the_request() {
    let source = "A deliberately long single-line label with a Unicode astronaut 👩‍🚀";
    assert!(!source.contains('\u{2026}'));
    let request = key(source, 128.0, false).with_overflow(TextOverflow::EndEllipsis);

    let layout = shape(&request);

    assert!(layout.is_elided());
    assert_eq!(layout.line_count, 1);
    assert!(layout.size.width <= request.width());
    assert_eq!(request.text, source);
    let markers = layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .collect::<Vec<_>>();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].start, markers[0].end);
}

#[test]
fn generated_marker_uses_the_bundled_ellipsis_glyph_identity() {
    let request = key(
        "This source contains no ellipsis character and must overflow",
        96.0,
        false,
    )
    .with_overflow(TextOverflow::EndEllipsis);
    let direct_request = key("\u{2026}", 96.0, false);
    let (generated, direct) = shape_pair(&request, &direct_request);

    let (generated_run, generated_glyph) = generated
        .runs
        .iter()
        .find_map(|run| {
            run.glyphs
                .iter()
                .find(|glyph| glyph.elided)
                .map(|glyph| (run, glyph))
        })
        .expect("generated ellipsis glyph");
    let (direct_run, direct_glyph) = direct
        .runs
        .iter()
        .find_map(|run| run.glyphs.first().map(|glyph| (run, glyph)))
        .expect("independently shaped U+2026");

    assert_eq!(generated_glyph.id, direct_glyph.id);
    assert_eq!(generated_run.font, direct_run.font);
    assert_eq!(
        generated_run.font_size.to_bits(),
        direct_run.font_size.to_bits()
    );
    assert!(!direct_glyph.elided);
    assert_eq!(
        (direct_glyph.start, direct_glyph.end),
        (0, '\u{2026}'.len_utf8())
    );
}

#[test]
fn full_fit_request_does_not_report_elision() {
    let request = key("Fits", 400.0, false).with_overflow(TextOverflow::EndEllipsis);
    let layout = shape(&request);

    assert!(!layout.is_elided());
    assert!(
        layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .all(|glyph| !glyph.elided && glyph.start < glyph.end)
    );
}

#[test]
fn ineligible_widths_preserve_visible_layout() {
    for width in [0.0, -12.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let visible = key("A long source that would otherwise be elided", width, false);
        let requested = visible.clone().with_overflow(TextOverflow::EndEllipsis);

        let (requested_layout, visible_layout) = shape_pair(&requested, &visible);
        assert_eq!(
            requested_layout,
            visible_layout,
            "width bits {:#x}",
            width.to_bits()
        );
        assert!(!requested_layout.is_elided());
    }
}

#[test]
fn wrapping_and_multiline_requests_preserve_existing_behavior() {
    let wrapping = key("alpha beta gamma delta epsilon", 52.0, true);
    let requested_wrap = wrapping.clone().with_overflow(TextOverflow::EndEllipsis);
    let (requested_layout, visible_layout) = shape_pair(&requested_wrap, &wrapping);
    assert_eq!(requested_layout, visible_layout);
    assert!(requested_layout.line_count > 1);

    for (separator, source) in [
        ("LF", "line one\nline two"),
        ("CR", "line one\rline two"),
        ("FS", "line one\u{001c}line two"),
        ("GS", "line one\u{001d}line two"),
        ("RS", "line one\u{001e}line two"),
        ("NEL", "line one\u{0085}line two"),
        ("PS", "line one\u{2029}line two"),
    ] {
        let visible = key(source, 48.0, false);
        let requested = visible.clone().with_overflow(TextOverflow::EndEllipsis);
        let (requested_layout, visible_layout) = shape_pair(&requested, &visible);
        assert_eq!(
            requested_layout, visible_layout,
            "{separator} source {source:?}"
        );
        assert!(
            !requested_layout.is_elided(),
            "{separator} source {source:?}"
        );
    }
}
