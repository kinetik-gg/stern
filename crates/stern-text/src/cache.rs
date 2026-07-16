use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::mem::size_of;

use stern_core::Size;

use crate::{TextLayout, TextLayoutKey};

const MAX_RETAINED_LAYOUT_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
const MAX_IDLE_LAYOUT_GENERATIONS: u64 = 120;

/// Compatibility text measurement cache with bounded retained payload.
pub struct TextLayoutCache {
    layouts: HashMap<TextLayoutKey, CachedLayout>,
    retained_payload_bytes: usize,
    generation: u64,
    touch_ordinal: u64,
    policy: CachePolicy,
}

impl TextLayoutCache {
    /// Creates an empty text layout cache.
    #[must_use]
    pub fn new() -> Self {
        Self::with_policy(CachePolicy::default())
    }

    fn with_policy(policy: CachePolicy) -> Self {
        Self {
            layouts: HashMap::new(),
            retained_payload_bytes: 0,
            generation: 0,
            touch_ordinal: 0,
            policy,
        }
    }

    /// Returns a cached layout without refreshing its lifetime.
    #[must_use]
    pub fn get(&self, key: &TextLayoutKey) -> Option<TextLayout> {
        self.layouts.get(key).map(|entry| entry.layout)
    }

    /// Inserts a cached layout when it fits the strict retained-payload budget.
    pub fn insert(&mut self, key: TextLayoutKey, layout: TextLayout) {
        if self.layouts.contains_key(&key) {
            let Some(touch_ordinal) = self.issue_touch_ordinal() else {
                return;
            };
            let Some(entry) = self.layouts.get_mut(&key) else {
                return;
            };
            entry.layout = layout;
            entry.last_generation = self.generation;
            entry.touch_ordinal = touch_ordinal;
            return;
        }

        let key = normalize_key(key);
        let Some(payload_bytes) = cache_entry_payload_bytes(&key) else {
            return;
        };
        if payload_bytes > self.policy.max_payload_bytes {
            return;
        }
        let Some(victims) = self.planned_capacity_evictions(payload_bytes) else {
            return;
        };
        let Some(next_payload) = self
            .retained_payload_bytes
            .checked_add(payload_bytes)
            .and_then(|bytes| {
                victims.iter().try_fold(bytes, |remaining, victim| {
                    remaining.checked_sub(self.layouts.get(victim)?.payload_bytes)
                })
            })
            .filter(|bytes| *bytes <= self.policy.max_payload_bytes)
        else {
            return;
        };

        let Some(touch_ordinal) = self.issue_touch_ordinal() else {
            return;
        };
        for victim in victims {
            self.remove(&victim);
        }
        self.layouts.insert(
            key,
            CachedLayout {
                layout,
                payload_bytes,
                last_generation: self.generation,
                touch_ordinal,
            },
        );
        self.retained_payload_bytes = next_payload;
    }

    /// Returns an existing layout or measures a value, touching mutable hits.
    ///
    /// Measurement is always returned even when the result cannot be retained.
    pub fn get_or_measure(&mut self, key: TextLayoutKey) -> TextLayout {
        if let Some(layout) = self.layouts.get(&key).map(|entry| entry.layout) {
            self.touch(&key);
            return layout;
        }
        let layout = fallback_measure(&key);
        self.insert(key, layout);
        layout
    }

    /// Advances one logical generation and evicts entries idle for 121 frames.
    pub fn advance_generation(&mut self) {
        let Some(generation) = self.generation.checked_add(1) else {
            self.clear();
            return;
        };
        self.generation = generation;
        let mut expired = self
            .layouts
            .iter()
            .filter_map(|(key, entry)| {
                let idle = generation.checked_sub(entry.last_generation)?;
                (idle > self.policy.max_idle_generations).then_some(key.clone())
            })
            .collect::<Vec<_>>();
        self.sort_eviction_keys(&mut expired);
        for key in expired {
            self.remove(&key);
        }
    }

    /// Returns the current cache generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns checked owned key/layout payload retained by this cache.
    ///
    /// The metric excludes lifecycle metadata, hash-table buckets, and allocator
    /// bookkeeping.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> usize {
        self.retained_payload_bytes
    }

    /// Clears all cached layouts and lifecycle state.
    pub fn clear(&mut self) {
        self.layouts.clear();
        self.retained_payload_bytes = 0;
        self.generation = 0;
        self.touch_ordinal = 0;
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Returns true when the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }

    fn touch(&mut self, key: &TextLayoutKey) {
        if !self.layouts.contains_key(key) {
            return;
        }
        let Some(touch_ordinal) = self.issue_touch_ordinal() else {
            return;
        };
        let entry = self
            .layouts
            .get_mut(key)
            .expect("resident cache key disappeared while touching it");
        entry.last_generation = self.generation;
        entry.touch_ordinal = touch_ordinal;
    }

    fn planned_capacity_evictions(&self, candidate_bytes: usize) -> Option<Vec<TextLayoutKey>> {
        let mut projected = self.retained_payload_bytes.checked_add(candidate_bytes)?;
        if projected <= self.policy.max_payload_bytes {
            return Some(Vec::new());
        }
        let mut eligible = self
            .layouts
            .iter()
            .filter_map(|(key, entry)| {
                (entry.last_generation != self.generation).then_some(key.clone())
            })
            .collect::<Vec<_>>();
        self.sort_eviction_keys(&mut eligible);

        let mut victims = Vec::new();
        for key in eligible {
            projected = projected.checked_sub(self.layouts.get(&key)?.payload_bytes)?;
            victims.push(key);
            if projected <= self.policy.max_payload_bytes {
                return Some(victims);
            }
        }
        None
    }

    fn sort_eviction_keys(&self, keys: &mut [TextLayoutKey]) {
        keys.sort_unstable_by(|left, right| {
            let left_entry = self.layouts.get(left);
            let right_entry = self.layouts.get(right);
            let left_order = left_entry.map_or((u64::MAX, u64::MAX), |entry| {
                (entry.last_generation, entry.touch_ordinal)
            });
            let right_order = right_entry.map_or((u64::MAX, u64::MAX), |entry| {
                (entry.last_generation, entry.touch_ordinal)
            });
            left_order
                .cmp(&right_order)
                .then_with(|| compare_keys(left, right))
        });
    }

    fn issue_touch_ordinal(&mut self) -> Option<u64> {
        if let Some(next) = self.touch_ordinal.checked_add(1) {
            self.touch_ordinal = next;
            return Some(next);
        }

        let mut keys = self.layouts.keys().cloned().collect::<Vec<_>>();
        self.sort_eviction_keys(&mut keys);
        let mut assignments = Vec::with_capacity(keys.len());
        for (index, key) in keys.into_iter().enumerate() {
            let ordinal = u64::try_from(index).ok()?.checked_add(1)?;
            assignments.push((key, ordinal));
        }
        let next = u64::try_from(assignments.len()).ok()?.checked_add(1)?;
        for (key, ordinal) in assignments {
            self.layouts.get_mut(&key)?.touch_ordinal = ordinal;
        }
        self.touch_ordinal = next;
        Some(next)
    }

    fn remove(&mut self, key: &TextLayoutKey) {
        let Some(entry) = self.layouts.remove(key) else {
            return;
        };
        self.retained_payload_bytes = self
            .retained_payload_bytes
            .checked_sub(entry.payload_bytes)
            .expect("cache payload accounting underflow");
    }
}

impl Clone for TextLayoutCache {
    fn clone(&self) -> Self {
        Self {
            layouts: self.layouts.clone(),
            retained_payload_bytes: self.retained_payload_bytes,
            generation: self.generation,
            touch_ordinal: self.touch_ordinal,
            policy: self.policy,
        }
    }
}

impl Default for TextLayoutCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for TextLayoutCache {
    fn eq(&self, other: &Self) -> bool {
        self.layouts.len() == other.layouts.len()
            && self.layouts.iter().all(|(key, entry)| {
                other
                    .layouts
                    .get(key)
                    .is_some_and(|other_entry| entry.layout == other_entry.layout)
            })
    }
}

impl fmt::Debug for TextLayoutCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextLayoutCache")
            .field("layouts", &VisibleLayouts(&self.layouts))
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct CachedLayout {
    layout: TextLayout,
    payload_bytes: usize,
    last_generation: u64,
    touch_ordinal: u64,
}

#[derive(Clone, Copy)]
struct CachePolicy {
    max_payload_bytes: usize,
    max_idle_generations: u64,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            max_payload_bytes: MAX_RETAINED_LAYOUT_PAYLOAD_BYTES,
            max_idle_generations: MAX_IDLE_LAYOUT_GENERATIONS,
        }
    }
}

struct VisibleLayouts<'a>(&'a HashMap<TextLayoutKey, CachedLayout>);

impl fmt::Debug for VisibleLayouts<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_map()
            .entries(self.0.iter().map(|(key, entry)| (key, entry.layout)))
            .finish()
    }
}

fn normalize_key(mut key: TextLayoutKey) -> TextLayoutKey {
    key.text = key.text.into_boxed_str().into_string();
    key.style.family = key.style.family.into_boxed_str().into_string();
    key
}

fn cache_entry_payload_bytes(key: &TextLayoutKey) -> Option<usize> {
    checked_payload_sum([
        size_of::<TextLayoutKey>(),
        key.text.capacity(),
        key.style.family.capacity(),
        size_of::<TextLayout>(),
    ])
}

fn checked_payload_sum(parts: impl IntoIterator<Item = usize>) -> Option<usize> {
    parts.into_iter().try_fold(0_usize, usize::checked_add)
}

fn compare_keys(left: &TextLayoutKey, right: &TextLayoutKey) -> Ordering {
    left.text
        .as_bytes()
        .cmp(right.text.as_bytes())
        .then_with(|| {
            left.style
                .family
                .as_bytes()
                .cmp(right.style.family.as_bytes())
        })
        .then_with(|| left.style.size_bits.cmp(&right.style.size_bits))
        .then_with(|| {
            left.style
                .line_height_bits
                .cmp(&right.style.line_height_bits)
        })
        .then_with(|| {
            left.style
                .features
                .ordering_key()
                .cmp(&right.style.features.ordering_key())
        })
        .then_with(|| left.width_bits.cmp(&right.width_bits))
        .then_with(|| left.wrap.cmp(&right.wrap))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn fallback_measure(key: &TextLayoutKey) -> TextLayout {
    let line_height = key.style.line_height();
    let char_width = key.style.size() * 0.55;
    let wrap_width = key.width().max(0.0);
    let mut line_count = 0;
    let mut measured_width = 0.0_f32;

    for line in key.text.split('\n') {
        let raw_width = line.chars().count() as f32 * char_width;
        if key.wrap && wrap_width > 0.0 && raw_width > wrap_width {
            let wrapped_lines = (raw_width / wrap_width).ceil() as usize;
            line_count += wrapped_lines;
            measured_width = measured_width.max(wrap_width);
        } else {
            line_count += 1;
            measured_width = measured_width.max(raw_width);
        }
    }

    let line_count = line_count.max(1);
    let width = if key.wrap {
        measured_width.min(wrap_width).max(0.0)
    } else {
        measured_width
    };

    TextLayout {
        size: Size::new(width, line_height * line_count as f32),
        line_count,
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use crate::{TextFeatureSet, TextStyle};

    fn key(text: &str) -> TextLayoutKey {
        TextLayoutKey::new(text, TextStyle::new("Inter", 12.0, 16.0), 100.0, false)
    }

    fn policy(max_payload_bytes: usize, max_idle_generations: u64) -> CachePolicy {
        CachePolicy {
            max_payload_bytes,
            max_idle_generations,
        }
    }

    #[test]
    fn exact_cache_payload_is_inclusive_and_oversized_measure_is_returned() {
        let request = normalize_key(key("exact"));
        let cost = cache_entry_payload_bytes(&request).expect("small key cost");
        let mut exact = TextLayoutCache::with_policy(policy(cost, 120));
        let measured = exact.get_or_measure(request.clone());
        assert_eq!(exact.get(&request), Some(measured));
        assert_eq!(exact.retained_payload_bytes(), cost);

        let mut under = TextLayoutCache::with_policy(policy(cost - 1, 120));
        assert_eq!(under.get_or_measure(request.clone()), measured);
        assert!(under.is_empty());
        assert_eq!(under.retained_payload_bytes(), 0);
        assert!(checked_payload_sum([usize::MAX, 1]).is_none());
    }

    #[test]
    fn cache_generation_boundary_and_mutable_hit_touch_are_literal() {
        let request = key("hot");
        let mut cache = TextLayoutCache::new();
        let measured = cache.get_or_measure(request.clone());
        for _ in 0..119 {
            cache.advance_generation();
        }
        assert_eq!(cache.get_or_measure(request.clone()), measured);
        for _ in 0..120 {
            cache.advance_generation();
        }
        assert_eq!(cache.get(&request), Some(measured));
        cache.advance_generation();
        assert_eq!(cache.get(&request), None);
    }

    #[test]
    fn observational_get_does_not_extend_cache_lifetime() {
        let request = key("cold");
        let mut cache = TextLayoutCache::new();
        cache.get_or_measure(request.clone());
        for _ in 0..120 {
            cache.advance_generation();
            let _ = cache.get(&request);
        }
        assert!(cache.get(&request).is_some());
        cache.advance_generation();
        assert!(cache.get(&request).is_none());
    }

    #[test]
    fn current_cache_generation_pins_then_deterministic_lru_evicts() {
        let a = normalize_key(key("a"));
        let b = normalize_key(key("b"));
        let c = normalize_key(key("c"));
        let cost = cache_entry_payload_bytes(&a).expect("cost");
        assert_eq!(cache_entry_payload_bytes(&b), Some(cost));
        assert_eq!(cache_entry_payload_bytes(&c), Some(cost));
        let mut cache = TextLayoutCache::with_policy(policy(cost * 2, 120));
        cache.get_or_measure(a.clone());
        cache.get_or_measure(b.clone());
        cache.get_or_measure(c.clone());
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&c).is_none());

        cache.advance_generation();
        cache.get_or_measure(a.clone());
        cache.get_or_measure(c.clone());
        assert!(cache.get(&a).is_some());
        assert!(cache.get(&b).is_none());
        assert!(cache.get(&c).is_some());
        assert_eq!(cache.retained_payload_bytes(), cost * 2);
    }

    #[test]
    fn resident_insert_replaces_value_without_recounting_owned_key() {
        let request = key("same");
        let mut cache = TextLayoutCache::new();
        cache.get_or_measure(request.clone());
        let bytes = cache.retained_payload_bytes();
        let replacement = TextLayout {
            size: Size::new(7.0, 9.0),
            line_count: 3,
        };
        let mut equal_text = String::with_capacity(1024);
        equal_text.push_str("same");
        cache.insert(
            TextLayoutKey::new(
                equal_text,
                TextStyle::new("Inter", 12.0, 16.0),
                100.0,
                false,
            ),
            replacement,
        );
        assert_eq!(cache.get(&request), Some(replacement));
        assert_eq!(cache.retained_payload_bytes(), bytes);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn clone_copies_future_lifecycle_but_equality_ignores_touch_metadata() {
        let a = key("a");
        let b = key("b");
        let mut cache = TextLayoutCache::new();
        cache.get_or_measure(a.clone());
        cache.get_or_measure(b);
        let mut clone = cache.clone();
        assert_eq!(cache, clone);
        cache.advance_generation();
        cache.get_or_measure(a.clone());
        assert_eq!(cache, clone, "visible entries still define equality");
        clone.advance_generation();
        clone.get_or_measure(a);
        assert_eq!(cache.generation(), clone.generation());
        assert_eq!(
            cache.retained_payload_bytes(),
            clone.retained_payload_bytes()
        );
    }

    #[test]
    fn structural_key_order_is_literal_and_touch_overflow_renumbers() {
        let ascii = key("a");
        let utf8 = key("é");
        assert_eq!(compare_keys(&ascii, &utf8), Ordering::Less);
        let mut cache = TextLayoutCache::new();
        cache.get_or_measure(ascii.clone());
        cache.get_or_measure(utf8);
        cache.touch_ordinal = u64::MAX;
        cache.get_or_measure(ascii);
        assert!(cache.touch_ordinal < u64::MAX);
    }

    #[test]
    fn touch_overflow_supports_empty_and_pressure_admission() {
        let a = normalize_key(key("a"));
        let b = normalize_key(key("b"));
        let entry_cost = cache_entry_payload_bytes(&a).expect("cost");
        assert_eq!(cache_entry_payload_bytes(&b), Some(entry_cost));

        let mut empty = TextLayoutCache::with_policy(policy(entry_cost, 120));
        empty.touch_ordinal = u64::MAX;
        empty.get_or_measure(a.clone());
        assert_eq!(empty.touch_ordinal, 1);
        assert_eq!(empty.layouts[&a].touch_ordinal, 1);
        assert_eq!(empty.retained_payload_bytes(), entry_cost);

        let mut pressure = TextLayoutCache::with_policy(policy(entry_cost, 120));
        pressure.get_or_measure(a.clone());
        pressure.advance_generation();
        pressure.touch_ordinal = u64::MAX;
        pressure.get_or_measure(b.clone());
        assert_eq!(pressure.touch_ordinal, 2);
        assert_eq!(pressure.len(), 1);
        assert_eq!(pressure.retained_payload_bytes(), entry_cost);
        assert!(pressure.get(&a).is_none());
        assert!(pressure.get(&b).is_some());
        assert_eq!(pressure.layouts[&b].touch_ordinal, 2);
    }

    #[test]
    fn equal_lru_metadata_uses_structural_key_order_as_the_final_cache_tie_break() {
        let a = normalize_key(key("aa"));
        let b = normalize_key(key("bb"));
        let c = normalize_key(key("cc"));
        let entry_cost = cache_entry_payload_bytes(&a).expect("cost");
        assert_eq!(cache_entry_payload_bytes(&b), Some(entry_cost));
        assert_eq!(cache_entry_payload_bytes(&c), Some(entry_cost));
        let mut cache = TextLayoutCache::with_policy(policy(entry_cost * 2, 120));
        cache.get_or_measure(a.clone());
        cache.get_or_measure(b.clone());
        cache.advance_generation();
        for request in [&a, &b] {
            let entry = cache.layouts.get_mut(request).expect("resident");
            entry.last_generation = 0;
            entry.touch_ordinal = 7;
        }

        cache.get_or_measure(c.clone());
        assert!(cache.get(&a).is_none(), "lexically first key loses the tie");
        assert!(cache.get(&b).is_some());
        assert!(cache.get(&c).is_some());
        assert_eq!(cache.retained_payload_bytes(), entry_cost * 2);
    }

    #[test]
    fn equal_lru_metadata_orders_features_before_selecting_the_cache_victim() {
        let none = normalize_key(key("00000000"));
        let tabular = normalize_key(TextLayoutKey::new(
            "00000000",
            TextStyle::new("Inter", 12.0, 16.0).with_features(TextFeatureSet::TABULAR_NUMBERS),
            100.0,
            false,
        ));
        let newcomer = normalize_key(key("zzzzzzzz"));
        let entry_cost = cache_entry_payload_bytes(&none).expect("cost");
        assert_eq!(cache_entry_payload_bytes(&tabular), Some(entry_cost));
        assert_eq!(cache_entry_payload_bytes(&newcomer), Some(entry_cost));
        assert_eq!(compare_keys(&none, &tabular), Ordering::Less);

        for insertion_order in [
            [none.clone(), tabular.clone()],
            [tabular.clone(), none.clone()],
        ] {
            let mut cache = TextLayoutCache::with_policy(policy(entry_cost * 2, 120));
            for request in insertion_order {
                cache.get_or_measure(request);
            }
            cache.advance_generation();
            for request in [&none, &tabular] {
                let entry = cache.layouts.get_mut(request).expect("resident");
                entry.last_generation = 0;
                entry.touch_ordinal = 7;
            }

            cache.get_or_measure(newcomer.clone());

            assert!(
                cache.get(&none).is_none(),
                "feature-disabled key is the literal first victim"
            );
            assert!(cache.get(&tabular).is_some());
            assert!(cache.get(&newcomer).is_some());
            assert_eq!(cache.retained_payload_bytes(), entry_cost * 2);
        }
    }
}
