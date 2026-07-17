//! Deterministic variable-font weight transport conformance.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use stern_core::{FontFamilyRole, FontWeightToken, default_dark_theme};
use stern_text::{
    CosmicTextEngine, TextFeatureSet, TextLayoutCache, TextLayoutKey, TextLayoutStore,
    TextNavigationError, TextOverflow, TextStyle, fonts,
};

const SOURCE: &str = "Stern 12038475";
const SEMANTIC_WEIGHTS: [u16; 4] = [400, 500, 600, 700];

fn key(family: &str, weight: u16) -> TextLayoutKey {
    TextLayoutKey::new(
        SOURCE,
        TextStyle::new(family, 20.0, 24.0).with_weight(weight),
        400.0,
        false,
    )
}

fn shape(family: &str, weight: u16) -> stern_text::ShapedTextLayout {
    CosmicTextEngine::new().shape_text(&key(family, weight))
}

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn assert_complete_source(layout: &stern_text::ShapedTextLayout, source: &str) {
    assert!(!layout.is_empty());
    assert!(!layout.is_elided());
    assert_eq!(layout.lines.first().expect("line").text_start, 0);
    assert_eq!(layout.lines.last().expect("line").text_end, source.len());
    assert!(layout.navigation(source).is_ok());
}

#[test]
fn default_and_semantic_theme_weights_remain_exact() {
    let theme = default_dark_theme();
    let tokens = [
        FontWeightToken::Regular,
        FontWeightToken::Medium,
        FontWeightToken::Semibold,
        FontWeightToken::Bold,
    ];
    let style = TextStyle::new("Inter", 12.0, 16.0);

    assert_eq!(style.weight, 400);
    for (token, expected) in tokens.into_iter().zip(SEMANTIC_WEIGHTS) {
        let resolved = theme.typography.weights.get(token);
        assert_eq!(resolved, expected);
        assert_eq!(style.clone().with_weight(resolved).weight, expected);
    }
    assert_eq!(style.clone().with_weight(0).weight, 0);
    assert_eq!(style.with_weight(u16::MAX).weight, u16::MAX);
}

#[test]
fn raw_weight_changes_style_key_hash_and_retained_identity() {
    let regular = key("Inter", 400);
    let medium = key("Inter", 500);
    assert_ne!(regular.style, medium.style);
    assert_ne!(regular, medium);
    assert_ne!(hash(&regular.style), hash(&medium.style));
    assert_ne!(hash(&regular), hash(&medium));

    let mut store = TextLayoutStore::new();
    let regular_id = store.layout_id(regular.clone());
    let medium_id = store.layout_id(medium.clone());
    let changed_source_id = store.layout_id(TextLayoutKey::new(
        "Stern 12038476",
        regular.style.clone(),
        regular.width(),
        regular.wrap,
    ));
    assert_ne!(regular_id, medium_id);
    assert_ne!(regular_id, changed_source_id);
    assert_ne!(medium_id, changed_source_id);
    assert_eq!(store.len(), 3);

    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    for _ in 0..10_000 {
        assert_eq!(store.try_layout_id(regular.clone()), Some(regular_id));
        assert_eq!(store.try_layout_id(medium.clone()), Some(medium_id));
    }
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let mut cache = TextLayoutCache::new();
    let regular_measurement = cache.get_or_measure(regular.clone());
    let medium_measurement = cache.get_or_measure(medium.clone());
    let bytes = cache.retained_payload_bytes();
    assert_eq!(cache.len(), 2);
    for _ in 0..10_000 {
        assert_eq!(cache.get_or_measure(regular.clone()), regular_measurement);
        assert_eq!(cache.get_or_measure(medium.clone()), medium_measurement);
    }
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.retained_payload_bytes(), bytes);
}

#[test]
fn bundled_variable_faces_emit_exact_pinned_coordinate_vectors() {
    let cases = [
        (
            "Inter",
            fonts::INTER_VARIABLE,
            [vec![0, 0], vec![0, 2_949], vec![0, 5_898], vec![0, 8_848]],
        ),
        (
            "Space Grotesk",
            fonts::SPACE_GROTESK_VARIABLE,
            [vec![4_751], vec![10_650], vec![13_517], vec![16_384]],
        ),
    ];

    for (family, bytes, expected_vectors) in cases {
        for (weight, expected) in SEMANTIC_WEIGHTS.into_iter().zip(expected_vectors) {
            let layout = shape(family, weight);
            assert_complete_source(&layout, SOURCE);
            assert!(layout.runs.iter().all(|run| run.font.data.data() == bytes));
            assert!(
                layout
                    .runs
                    .iter()
                    .all(|run| run.normalized_coords == expected)
            );
        }
    }
}

#[test]
fn selected_face_mapping_clamps_coordinates_without_rewriting_raw_identity() {
    let cases = [
        ("Inter", 0, 100, vec![0, -16_384]),
        ("Inter", 1_000, u16::MAX, vec![0, 16_384]),
        ("Space Grotesk", 0, 100, vec![0]),
        ("Space Grotesk", 900, u16::MAX, vec![16_384]),
    ];

    for (family, first_weight, second_weight, endpoint) in cases {
        let first_key = key(family, first_weight);
        let second_key = key(family, second_weight);
        let first = shape(family, first_weight);
        let second = shape(family, second_weight);
        assert_ne!(first_key, second_key);
        assert_eq!(first_key.style.weight, first_weight);
        assert_eq!(second_key.style.weight, second_weight);
        assert!(
            first
                .runs
                .iter()
                .chain(&second.runs)
                .all(|run| run.normalized_coords == endpoint)
        );

        let mut store = TextLayoutStore::new();
        assert_ne!(store.layout_id(first_key), store.layout_id(second_key));
    }
}

#[test]
fn static_space_mono_keeps_exact_bytes_and_empty_coordinates() {
    let theme = default_dark_theme();
    let family = theme.font_family(FontFamilyRole::Mono);
    assert_eq!(family, "Space Mono");
    for weight in SEMANTIC_WEIGHTS {
        let layout = shape(family, weight);
        assert_complete_source(&layout, SOURCE);
        assert!(
            layout
                .runs
                .iter()
                .all(|run| run.font.data.data() == fonts::SPACE_MONO_REGULAR)
        );
        assert!(
            layout
                .runs
                .iter()
                .all(|run| run.normalized_coords.is_empty())
        );
    }
}

#[test]
fn features_and_end_ellipsis_preserve_weight_source_and_navigation_policy() {
    let source = "The complete weighted numeric source remains retained after ellipsis 12038475";
    let request = TextLayoutKey::new(
        source,
        TextStyle::new("Inter", 18.0, 24.0)
            .with_weight(600)
            .with_features(TextFeatureSet::TABULAR_NUMBERS),
        96.0,
        false,
    )
    .with_overflow(TextOverflow::EndEllipsis);
    let layout = CosmicTextEngine::new().shape_text(&request);
    let markers = layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .collect::<Vec<_>>();

    assert_eq!(request.text, source);
    assert_eq!(request.style.weight, 600);
    assert_eq!(request.style.features, TextFeatureSet::TABULAR_NUMBERS);
    assert_eq!(request.overflow, TextOverflow::EndEllipsis);
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.normalized_coords == [0, 5_898])
    );
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].start, markers[0].end);
    assert_eq!(
        layout.navigation(source),
        Err(TextNavigationError::ElidedLayout)
    );
}

#[test]
fn explicit_regular_is_exactly_equivalent_to_constructor_default() {
    let default = TextLayoutKey::new(SOURCE, TextStyle::new("Inter", 20.0, 24.0), 400.0, false);
    let explicit = TextLayoutKey::new(
        SOURCE,
        TextStyle::new("Inter", 20.0, 24.0).with_weight(400),
        400.0,
        false,
    );
    assert_eq!(default, explicit);
    assert_eq!(hash(&default), hash(&explicit));

    let mut engine = CosmicTextEngine::new();
    let default_layout = engine.shape_text(&default);
    let explicit_layout = engine.shape_text(&explicit);
    assert_eq!(default_layout, explicit_layout);

    let mut store = TextLayoutStore::new();
    let default_id = store.layout_id(default.clone());
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    assert_eq!(store.layout_id(explicit.clone()), default_id);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let mut cache = TextLayoutCache::new();
    assert_eq!(
        cache.get_or_measure(default),
        cache.get_or_measure(explicit)
    );
    assert_eq!(cache.len(), 1);
}
