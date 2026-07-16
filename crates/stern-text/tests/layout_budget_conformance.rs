//! Long-session and public-surface conformance for retained text layout budgets.

use std::sync::Arc;

use stern_core::TextLayoutId;
use stern_text::{
    TextFeatureSet, TextLayoutCache, TextLayoutKey, TextLayoutStore, TextOverflow, TextStyle,
};

const MAX_RETAINED_BYTES: usize = 32 * 1024 * 1024;

fn style() -> TextStyle {
    TextStyle::new("Inter", 13.0, 17.0)
}

fn numeric_style() -> TextStyle {
    style().with_features(TextFeatureSet::TABULAR_NUMBERS)
}

fn key(text: impl Into<String>, width: f32, wrap: bool) -> TextLayoutKey {
    TextLayoutKey::new(text, style(), width, wrap)
}

#[test]
fn one_hundred_thousand_hits_keep_id_arc_bytes_and_journal_stable() {
    let mut store = TextLayoutStore::new();
    let request = key("hot e\u{301} 👩‍👩‍👧‍👦 العربية", 140.0, true);
    let id = store.layout_id(request.clone());
    let original = store.stored_layout(id).expect("resident layout").layout;
    let bytes = store.retained_payload_bytes();
    let cursor = store.change_cursor();

    for _ in 0..100_000 {
        assert_eq!(store.try_layout_id(request.clone()), Some(id));
    }

    let current = store.stored_layout(id).expect("resident layout").layout;
    assert!(Arc::ptr_eq(&original, &current));
    assert_eq!(store.len(), 1);
    assert_eq!(store.retained_payload_bytes(), bytes);
    assert_eq!(store.change_cursor(), cursor);
    let no_changes = store.changes_since(cursor);
    assert!(!no_changes.requires_reset());
    assert_eq!(no_changes.iter().count(), 0);
}

#[test]
fn dynamic_unicode_labels_plateau_at_the_literal_idle_boundary() {
    let mut store = TextLayoutStore::new();
    let anchor_key = key("permanent anchor", 120.0, false);
    let anchor = store.layout_id(anchor_key.clone());
    let samples = [
        "ascii",
        "e\u{301}",
        "👩‍👩‍👧‍👦",
        "العربية-עברית",
        "line one\nline two",
    ];
    let mut first_dynamic = None;

    for generation in 0..1_000 {
        store.advance_generation();
        assert_eq!(store.try_layout_id(anchor_key.clone()), Some(anchor));
        let text = format!("{}-{generation}", samples[generation % samples.len()]);
        let width = [48.0, 49.0, 50.0, 51.0][generation % 4];
        let id = store
            .try_layout_id(key(text, width, generation % 2 == 0))
            .expect("small dynamic layout fits");
        first_dynamic.get_or_insert(id);
        assert!(store.retained_payload_bytes() <= MAX_RETAINED_BYTES);
    }

    assert_eq!(store.try_layout_id(anchor_key), Some(anchor));
    assert!(store.len() <= 122, "anchor plus 121 inclusive generations");
    let first_dynamic = first_dynamic.expect("first dynamic ID");
    assert!(store.layout(first_dynamic).is_none());
    let reincarnated = store
        .try_layout_id(key("ascii-0", 48.0, true))
        .expect("evicted label can be admitted again");
    assert_ne!(reincarnated, first_dynamic);
}

#[test]
fn key_larger_than_budget_rejects_before_retained_state_changes() {
    let mut store = TextLayoutStore::new();
    let cursor = store.change_cursor();
    let huge = "x".repeat(MAX_RETAINED_BYTES + 1);
    let request = key(huge, 100.0, false);

    assert_eq!(store.try_layout_id(request.clone()), None);
    assert_eq!(store.layout_id(request), TextLayoutId::from_raw(0));
    assert!(store.is_empty());
    assert_eq!(store.retained_payload_bytes(), 0);
    assert_eq!(store.change_cursor(), cursor);
    assert!(store.layout(TextLayoutId::from_raw(0)).is_none());
}

#[test]
fn changes_are_source_bound_and_reconcile_final_presence() {
    let mut first = TextLayoutStore::new();
    let second = TextLayoutStore::new();
    let initial = first.change_cursor();
    let foreign = second.change_cursor();
    let id = first.layout_id(key("resource", 100.0, false));

    let additions = first.changes_since(initial);
    assert!(!additions.requires_reset());
    assert_eq!(
        additions
            .iter()
            .map(stern_text::TextLayoutChange::id)
            .collect::<Vec<_>>(),
        [id]
    );
    assert_eq!(
        first.stored_layout(id).expect("final presence").key.text,
        "resource"
    );
    assert!(first.changes_since(foreign).requires_reset());

    let before_clear = first.change_cursor();
    first.clear();
    assert!(first.changes_since(before_clear).requires_reset());
    assert!(first.stored_layout(id).is_none());
}

#[test]
fn transient_unicode_shaping_and_observation_do_not_retain_or_touch() {
    let mut store = TextLayoutStore::new();
    let anchor = store.layout_id(key("anchor", 100.0, false));
    let cursor = store.change_cursor();
    let bytes = store.retained_payload_bytes();

    for source in ["e\u{301}", "👩‍👩‍👧‍👦", "العربية עברית", "a\nb\nc"]
    {
        let shaped = store.shape_transient(&key(source, 36.0, true));
        assert!(shaped.line_count >= 1);
    }
    let _ = store.layout(anchor);
    let _ = store.stored_layout(anchor);
    let _ = store.layouts().collect::<Vec<_>>();

    assert_eq!(store.len(), 1);
    assert_eq!(store.retained_payload_bytes(), bytes);
    assert_eq!(store.change_cursor(), cursor);
}

#[test]
fn compatibility_cache_dynamic_workload_is_bounded_and_hot_entry_survives() {
    let mut cache = TextLayoutCache::new();
    let anchor = key("cache anchor", 100.0, false);
    let anchor_layout = cache.get_or_measure(anchor.clone());

    for generation in 0..1_000 {
        cache.advance_generation();
        assert_eq!(cache.get_or_measure(anchor.clone()), anchor_layout);
        let source = match generation % 4 {
            0 => format!("ascii-{generation}"),
            1 => format!("e\u{301}-{generation}"),
            2 => format!("👩‍👩‍👧‍👦-{generation}"),
            _ => format!("العربية\n{generation}"),
        };
        let measured = cache.get_or_measure(key(source, 44.0, true));
        assert!(measured.line_count >= 1);
        assert!(cache.retained_payload_bytes() <= MAX_RETAINED_BYTES);
    }

    assert_eq!(cache.get(&anchor), Some(anchor_layout));
    assert!(cache.len() <= 122);
    let clone = cache.clone();
    assert_eq!(clone, cache);
    assert_eq!(
        clone.retained_payload_bytes(),
        cache.retained_payload_bytes()
    );
}

#[test]
fn feature_aware_store_and_cache_identity_stay_bounded_on_hot_hits() {
    let default_key = key("12038475", 140.0, false);
    let numeric_key = TextLayoutKey::new("12038475", numeric_style(), 140.0, false);
    let mut store = TextLayoutStore::new();

    let default_id = store.layout_id(default_key.clone());
    let numeric_id = store.layout_id(numeric_key.clone());
    let store_bytes = store.retained_payload_bytes();
    let store_cursor = store.change_cursor();

    assert_ne!(default_id, numeric_id);
    assert_eq!(store.len(), 2);
    assert!(store_bytes <= MAX_RETAINED_BYTES);
    for _ in 0..10_000 {
        assert_eq!(store.try_layout_id(default_key.clone()), Some(default_id));
        assert_eq!(store.try_layout_id(numeric_key.clone()), Some(numeric_id));
    }
    assert_eq!(store.retained_payload_bytes(), store_bytes);
    assert_eq!(store.change_cursor(), store_cursor);
    assert_eq!(
        store.stored_layout(default_id).unwrap().key.style.features,
        TextFeatureSet::NONE
    );
    assert_eq!(
        store.stored_layout(numeric_id).unwrap().key.style.features,
        TextFeatureSet::TABULAR_NUMBERS
    );

    let mut cache = TextLayoutCache::new();
    let default_layout = cache.get_or_measure(default_key.clone());
    let numeric_layout = cache.get_or_measure(numeric_key.clone());
    let cache_bytes = cache.retained_payload_bytes();

    assert_eq!(cache.len(), 2);
    assert!(cache_bytes <= MAX_RETAINED_BYTES);
    for _ in 0..10_000 {
        assert_eq!(cache.get_or_measure(default_key.clone()), default_layout);
        assert_eq!(cache.get_or_measure(numeric_key.clone()), numeric_layout);
    }
    assert_eq!(cache.retained_payload_bytes(), cache_bytes);
    assert_eq!(cache.get(&default_key), Some(default_layout));
    assert_eq!(cache.get(&numeric_key), Some(numeric_layout));
}

#[test]
fn overflow_policy_has_distinct_stable_store_and_cache_identity() {
    let source = "The complete caller-owned source remains retained after presentation elision 👩‍🚀";
    let visible = key(source, 96.0, false);
    let ellipsized = visible.clone().with_overflow(TextOverflow::EndEllipsis);
    let mut store = TextLayoutStore::new();

    let visible_id = store.layout_id(visible.clone());
    let ellipsized_id = store.layout_id(ellipsized.clone());
    let bytes = store.retained_payload_bytes();
    let cursor = store.change_cursor();

    assert_ne!(visible_id, ellipsized_id);
    assert!(!store.layout(visible_id).unwrap().is_elided());
    assert!(store.layout(ellipsized_id).unwrap().is_elided());
    for id in [visible_id, ellipsized_id] {
        let retained = store.stored_layout(id).expect("retained layout");
        assert_eq!(retained.key.text, source);
    }
    assert_eq!(
        store.stored_layout(visible_id).unwrap().key.overflow,
        TextOverflow::Visible
    );
    assert_eq!(
        store.stored_layout(ellipsized_id).unwrap().key.overflow,
        TextOverflow::EndEllipsis
    );

    for _ in 0..10_000 {
        assert_eq!(store.try_layout_id(visible.clone()), Some(visible_id));
        assert_eq!(store.try_layout_id(ellipsized.clone()), Some(ellipsized_id));
    }
    assert_eq!(store.retained_payload_bytes(), bytes);
    assert_eq!(store.change_cursor(), cursor);

    let mut cache = TextLayoutCache::new();
    let visible_measurement = cache.get_or_measure(visible.clone());
    let ellipsized_measurement = cache.get_or_measure(ellipsized.clone());
    let cache_bytes = cache.retained_payload_bytes();
    assert_eq!(visible_measurement, ellipsized_measurement);
    assert_eq!(cache.len(), 2);
    for _ in 0..10_000 {
        assert_eq!(cache.get_or_measure(visible.clone()), visible_measurement);
        assert_eq!(
            cache.get_or_measure(ellipsized.clone()),
            ellipsized_measurement
        );
    }
    assert_eq!(cache.retained_payload_bytes(), cache_bytes);
    assert_eq!(cache.get(&visible), Some(visible_measurement));
    assert_eq!(cache.get(&ellipsized), Some(ellipsized_measurement));
}
