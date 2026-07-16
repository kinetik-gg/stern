//! Deterministic conformance for Stern's bounded tabular-number shaping path.

use std::mem::size_of;

use stern_core::{FontFeatureScale, FontFeatureToken, default_dark_theme};
use stern_text::{CosmicTextEngine, TextFeatureSet, TextLayoutKey, TextStyle, fonts};

const ADVANCE_TOLERANCE: f32 = 0.001;

fn shape(
    engine: &mut CosmicTextEngine,
    text: &str,
    features: TextFeatureSet,
) -> stern_text::ShapedTextLayout {
    engine.shape_text(&TextLayoutKey::new(
        text,
        TextStyle::new("Inter", 32.0, 40.0).with_features(features),
        1_000.0,
        false,
    ))
}

fn assert_uses_bundled_inter(layout: &stern_text::ShapedTextLayout) {
    assert!(!layout.runs.is_empty());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.font.data.data() == fonts::INTER_VARIABLE)
    );
}

fn spread(values: &[f32]) -> f32 {
    let minimum = values.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    maximum - minimum
}

#[test]
fn public_feature_set_is_fixed_size_and_constructor_default_is_disabled() {
    let style = TextStyle::new("Inter", 12.0, 16.0);

    assert_eq!(size_of::<TextFeatureSet>(), 1);
    assert_eq!(TextFeatureSet::default(), TextFeatureSet::NONE);
    assert_eq!(style.features, TextFeatureSet::NONE);
    assert_eq!(
        style
            .clone()
            .with_features(TextFeatureSet::TABULAR_NUMBERS)
            .features,
        TextFeatureSet::TABULAR_NUMBERS
    );
}

#[test]
fn semantic_numeric_resolution_is_exact_and_fails_soft_for_custom_values() {
    assert_eq!(
        TextFeatureSet::resolve_semantic(
            default_dark_theme().typography.features,
            FontFeatureToken::Numeric,
        ),
        Some(TextFeatureSet::TABULAR_NUMBERS)
    );
    assert_eq!(
        TextFeatureSet::resolve_semantic(
            FontFeatureScale::new("unsupported-custom-value"),
            FontFeatureToken::Numeric,
        ),
        None
    );
}

#[test]
fn bundled_inter_has_proportional_control_and_equal_tabular_digit_advances() {
    let mut engine = CosmicTextEngine::new();
    let mut default_advances = Vec::new();
    let mut tabular_advances = Vec::new();

    for digit in '0'..='9' {
        let digit = digit.to_string();
        let default = shape(&mut engine, &digit, TextFeatureSet::NONE);
        let tabular = shape(&mut engine, &digit, TextFeatureSet::TABULAR_NUMBERS);
        assert_uses_bundled_inter(&default);
        assert_uses_bundled_inter(&tabular);
        default_advances.push(default.size.width);
        tabular_advances.push(tabular.size.width);
    }

    assert!(
        spread(&default_advances) > ADVANCE_TOLERANCE,
        "bundled Inter control unexpectedly has equal default advances: {default_advances:?}"
    );
    assert!(
        spread(&tabular_advances) <= ADVANCE_TOLERANCE,
        "tnum advances diverged beyond {ADVANCE_TOLERANCE}: {tabular_advances:?}"
    );
}

#[test]
fn equal_length_changing_numeric_strings_have_equal_tabular_widths() {
    let mut engine = CosmicTextEngine::new();
    let widths = ["11111111", "20486357", "99999999"].map(|text| {
        shape(&mut engine, text, TextFeatureSet::TABULAR_NUMBERS)
            .size
            .width
    });

    assert!(
        spread(&widths) <= ADVANCE_TOLERANCE,
        "equal-length tabular strings diverged beyond {ADVANCE_TOLERANCE}: {widths:?}"
    );
}

#[test]
fn feature_shaping_preserves_inter_ranges_and_layout_topology() {
    let text = "12,345.67";
    let mut engine = CosmicTextEngine::new();
    let default = shape(&mut engine, text, TextFeatureSet::NONE);
    let tabular = shape(&mut engine, text, TextFeatureSet::TABULAR_NUMBERS);

    assert_uses_bundled_inter(&default);
    assert_uses_bundled_inter(&tabular);
    assert_eq!(default.line_count, tabular.line_count);
    assert_eq!(default.lines.len(), tabular.lines.len());
    assert_eq!(default.runs.len(), tabular.runs.len());
    assert_eq!(default.glyph_count(), tabular.glyph_count());
    assert_eq!(
        default
            .lines
            .iter()
            .map(|line| {
                (
                    line.visual_index,
                    line.source_line_index,
                    line.text_start,
                    line.text_end,
                    line.rtl,
                )
            })
            .collect::<Vec<_>>(),
        tabular
            .lines
            .iter()
            .map(|line| {
                (
                    line.visual_index,
                    line.source_line_index,
                    line.text_start,
                    line.text_end,
                    line.rtl,
                )
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        default
            .runs
            .iter()
            .map(|run| (run.line_index, run.visual_line, run.glyphs.len()))
            .collect::<Vec<_>>(),
        tabular
            .runs
            .iter()
            .map(|run| (run.line_index, run.visual_line, run.glyphs.len()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        default
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(|glyph| (glyph.start, glyph.end)))
            .collect::<Vec<_>>(),
        tabular
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(|glyph| (glyph.start, glyph.end)))
            .collect::<Vec<_>>()
    );
}
