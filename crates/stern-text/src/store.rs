use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use stern_core::TextLayoutId;

use crate::{
    CosmicTextEngine, ShapedGlyph, ShapedGlyphRun, ShapedTextLayout, ShapedTextLine, TextLayoutKey,
};

const MAX_RETAINED_LAYOUT_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
const MAX_IDLE_LAYOUT_GENERATIONS: u64 = 120;
const MAX_LAYOUT_CHANGE_JOURNAL_BYTES: usize = 256 * 1024;
const REJECTED_LAYOUT_ID: TextLayoutId = TextLayoutId::from_raw(0);

static NEXT_STORE_INCARNATION: AtomicU64 = AtomicU64::new(1);

/// Opaque position in one text layout store's change history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextLayoutChangeCursor {
    incarnation: u64,
    epoch: u64,
    revision: u64,
    resync_only: bool,
}

/// One retained-layout ID whose final presence must be reconciled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextLayoutChange {
    revision: u64,
    id: TextLayoutId,
}

impl TextLayoutChange {
    /// Returns the dirty layout ID.
    #[must_use]
    pub const fn id(self) -> TextLayoutId {
        self.id
    }
}

/// Borrowed view of retained-layout changes after a cursor.
#[derive(Debug)]
pub struct TextLayoutChanges<'a> {
    requires_reset: bool,
    cursor: TextLayoutChangeCursor,
    changes: &'a [TextLayoutChange],
}

impl TextLayoutChanges<'_> {
    /// Returns whether the consumer must rebuild from a complete store snapshot.
    #[must_use]
    pub const fn requires_reset(&self) -> bool {
        self.requires_reset
    }

    /// Returns the cursor after this batch.
    #[must_use]
    pub const fn cursor(&self) -> TextLayoutChangeCursor {
        self.cursor
    }

    /// Iterates dirty IDs in deterministic mutation order.
    pub fn iter(&self) -> impl Iterator<Item = TextLayoutChange> + '_ {
        self.changes.iter().copied()
    }
}

/// Persistent shaped text layout cache.
///
/// Retained owned key/layout payload is strictly bounded. The metric excludes
/// hash-table buckets, allocator and `Arc` headers, shared font data, external
/// `Arc` owners, and shaping-engine internals.
pub struct TextLayoutStore {
    engine: CosmicTextEngine,
    keys: HashMap<Arc<TextLayoutKey>, TextLayoutId>,
    entries: HashMap<TextLayoutId, RetainedLayout>,
    retired_ids: HashSet<TextLayoutId>,
    retained_payload_bytes: usize,
    generation: u64,
    touch_ordinal: u64,
    journal: LayoutChangeJournal,
    policy: LayoutStorePolicy,
}

impl TextLayoutStore {
    /// Creates an empty shaped text layout store.
    #[must_use]
    pub fn new() -> Self {
        Self::with_policy(LayoutStorePolicy::default())
    }

    fn with_policy(policy: LayoutStorePolicy) -> Self {
        Self {
            engine: CosmicTextEngine::new(),
            keys: HashMap::new(),
            entries: HashMap::new(),
            retired_ids: HashSet::new(),
            retained_payload_bytes: 0,
            generation: 0,
            touch_ordinal: 0,
            journal: LayoutChangeJournal::new(policy.journal_bytes),
            policy,
        }
    }

    /// Returns the number of cached shaped layouts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true when no shaped layouts are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the checked owned key/layout payload retained by this store.
    #[must_use]
    pub const fn retained_payload_bytes(&self) -> usize {
        self.retained_payload_bytes
    }

    /// Returns the current layout generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Advances one logical frame generation and removes expired layouts.
    pub fn advance_generation(&mut self) {
        let Some(generation) = self.generation.checked_add(1) else {
            self.clear_for_counter_overflow();
            return;
        };
        self.generation = generation;

        let mut expired = self
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                let idle = generation.checked_sub(entry.last_generation)?;
                (idle > self.policy.max_idle_generations).then_some(*id)
            })
            .collect::<Vec<_>>();
        self.sort_eviction_ids(&mut expired);
        for id in expired {
            self.remove_entry(id, true);
        }
    }

    /// Clears all cached shaped layouts and establishes a full-sync boundary.
    pub fn clear(&mut self) {
        self.clear_entries();
        self.generation = 0;
        self.touch_ordinal = 0;
        self.retired_ids.clear();
        let _ = self.journal.reset_epoch();
    }

    /// Returns the backing text engine.
    #[must_use]
    pub const fn engine(&self) -> &CosmicTextEngine {
        &self.engine
    }

    /// Returns mutable access to the backing text engine.
    pub fn engine_mut(&mut self) -> &mut CosmicTextEngine {
        &mut self.engine
    }

    /// Shapes a layout without changing retained entries or change history.
    pub fn shape_transient(&mut self, key: &TextLayoutKey) -> ShapedTextLayout {
        self.engine.shape_text(key)
    }

    /// Returns a stable resident layout ID, or `None` when admission is rejected.
    pub fn try_layout_id(&mut self, key: TextLayoutKey) -> Option<TextLayoutId> {
        let preferred_id = text_layout_id(&key);
        self.try_layout_id_with_preferred_id(key, preferred_id)
    }

    /// Returns a stable layout ID for a text layout key, shaping on cache miss.
    ///
    /// When strict admission fails, this compatibility method returns the
    /// store-local sentinel ID `0`; the store never assigns or resolves it.
    pub fn layout_id(&mut self, key: TextLayoutKey) -> TextLayoutId {
        let preferred_id = text_layout_id(&key);
        self.layout_id_with_preferred_id(key, preferred_id)
    }

    pub(crate) fn layout_id_with_preferred_id(
        &mut self,
        key: TextLayoutKey,
        preferred_id: TextLayoutId,
    ) -> TextLayoutId {
        self.try_layout_id_with_preferred_id(key, preferred_id)
            .unwrap_or(REJECTED_LAYOUT_ID)
    }

    fn try_layout_id_with_preferred_id(
        &mut self,
        key: TextLayoutKey,
        preferred_id: TextLayoutId,
    ) -> Option<TextLayoutId> {
        if let Some(id) = self.keys.get(&key).copied() {
            let touched = self.touch_layout(id);
            debug_assert!(touched, "key and ID indices must remain bijective");
            return touched.then_some(id);
        }

        let key = normalize_key(key);
        let key_bytes = retained_key_payload_bytes(&key)?;
        if key_bytes > self.policy.max_payload_bytes {
            return None;
        }

        let layout = self.engine.shape_text(&key);
        let payload_bytes = retained_layout_payload_bytes(&key, &layout)?;
        if payload_bytes > self.policy.max_payload_bytes {
            return None;
        }

        let victims = self.planned_capacity_evictions(payload_bytes)?;
        let id = self.available_layout_id(preferred_id)?;
        let victim_bytes = victims.iter().try_fold(0_usize, |total, victim| {
            total.checked_add(self.entries.get(victim)?.payload_bytes)
        })?;
        let next_payload = self
            .retained_payload_bytes
            .checked_add(payload_bytes)?
            .checked_sub(victim_bytes)
            .filter(|bytes| *bytes <= self.policy.max_payload_bytes)?;
        let touch_ordinal = self.issue_touch_ordinal()?;
        for victim in victims {
            self.remove_entry(victim, true);
        }

        let key = Arc::new(key);
        let previous_key = self.keys.insert(Arc::clone(&key), id);
        debug_assert!(previous_key.is_none());
        let previous_entry = self.entries.insert(
            id,
            RetainedLayout {
                key,
                layout: Arc::new(layout),
                payload_bytes,
                last_generation: self.generation,
                touch_ordinal,
            },
        );
        debug_assert!(previous_entry.is_none());
        self.retained_payload_bytes = next_payload;
        self.record_change(id, false);
        Some(id)
    }

    /// Marks a resident held ID as used in the current generation.
    pub fn touch_layout(&mut self, id: TextLayoutId) -> bool {
        if !self.entries.contains_key(&id) {
            return false;
        }
        let Some(touch_ordinal) = self.issue_touch_ordinal() else {
            return false;
        };
        let Some(entry) = self.entries.get_mut(&id) else {
            return false;
        };
        entry.last_generation = self.generation;
        entry.touch_ordinal = touch_ordinal;
        true
    }

    /// Returns a shaped layout by ID without refreshing its lifetime.
    #[must_use]
    pub fn layout(&self, id: TextLayoutId) -> Option<&ShapedTextLayout> {
        self.entries.get(&id).map(|entry| entry.layout.as_ref())
    }

    /// Returns one stored entry by ID without refreshing its lifetime.
    #[must_use]
    pub fn stored_layout(&self, id: TextLayoutId) -> Option<StoredTextLayout<'_>> {
        self.entries.get(&id).map(|entry| StoredTextLayout {
            id,
            key: entry.key.as_ref(),
            layout: Arc::clone(&entry.layout),
        })
    }

    /// Iterates cached layouts in ascending ID order without refreshing them.
    pub fn layouts(&self) -> impl Iterator<Item = StoredTextLayout<'_>> {
        let mut ids = self.entries.keys().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        ids.into_iter().filter_map(move |id| self.stored_layout(id))
    }

    /// Returns the current position in this store's change journal.
    #[must_use]
    pub const fn change_cursor(&self) -> TextLayoutChangeCursor {
        self.journal.cursor()
    }

    /// Returns retained-layout changes after `cursor`.
    #[must_use]
    pub fn changes_since(&self, cursor: TextLayoutChangeCursor) -> TextLayoutChanges<'_> {
        self.journal.changes_since(cursor)
    }

    fn planned_capacity_evictions(&self, candidate_bytes: usize) -> Option<Vec<TextLayoutId>> {
        let mut projected = self.retained_payload_bytes.checked_add(candidate_bytes)?;
        if projected <= self.policy.max_payload_bytes {
            return Some(Vec::new());
        }

        let mut eligible = self
            .entries
            .iter()
            .filter_map(|(id, entry)| (entry.last_generation != self.generation).then_some(*id))
            .collect::<Vec<_>>();
        self.sort_eviction_ids(&mut eligible);

        let mut victims = Vec::new();
        for id in eligible {
            let entry = self.entries.get(&id)?;
            projected = projected.checked_sub(entry.payload_bytes)?;
            victims.push(id);
            if projected <= self.policy.max_payload_bytes {
                return Some(victims);
            }
        }
        None
    }

    fn sort_eviction_ids(&self, ids: &mut [TextLayoutId]) {
        ids.sort_unstable_by_key(|id| {
            self.entries
                .get(id)
                .map_or((u64::MAX, u64::MAX, id.raw()), |entry| {
                    (entry.last_generation, entry.touch_ordinal, id.raw())
                })
        });
    }

    fn available_layout_id(&self, preferred_id: TextLayoutId) -> Option<TextLayoutId> {
        let max_probes = self
            .entries
            .len()
            .checked_add(self.retired_ids.len())?
            .checked_add(1)?;
        let mut raw = preferred_id.raw().max(1);
        for _ in 0..max_probes {
            let id = TextLayoutId::from_raw(raw);
            if !self.entries.contains_key(&id) && !self.retired_ids.contains(&id) {
                return Some(id);
            }
            raw = raw.checked_add(1)?;
        }
        None
    }

    fn issue_touch_ordinal(&mut self) -> Option<u64> {
        if let Some(next) = self.touch_ordinal.checked_add(1) {
            self.touch_ordinal = next;
            return Some(next);
        }

        let mut ids = self.entries.keys().copied().collect::<Vec<_>>();
        self.sort_eviction_ids(&mut ids);
        let mut assignments = Vec::with_capacity(ids.len());
        for (index, id) in ids.into_iter().enumerate() {
            let ordinal = u64::try_from(index).ok()?.checked_add(1)?;
            assignments.push((id, ordinal));
        }
        let next = u64::try_from(assignments.len()).ok()?.checked_add(1)?;
        for (id, ordinal) in assignments {
            self.entries.get_mut(&id)?.touch_ordinal = ordinal;
        }
        self.touch_ordinal = next;
        Some(next)
    }

    fn remove_entry(&mut self, id: TextLayoutId, journal: bool) {
        let Some(entry) = self.entries.remove(&id) else {
            return;
        };
        let removed = self.keys.remove(entry.key.as_ref());
        debug_assert_eq!(removed, Some(id));
        self.retained_payload_bytes = self
            .retained_payload_bytes
            .checked_sub(entry.payload_bytes)
            .expect("retained payload accounting underflow");
        if journal {
            self.record_change(id, true);
        }
    }

    fn record_change(&mut self, id: TextLayoutId, removed: bool) {
        match self.journal.record(id) {
            JournalRecordOutcome::Recorded => {
                if removed {
                    self.retired_ids.insert(id);
                }
            }
            JournalRecordOutcome::EpochReset => {
                self.retired_ids.clear();
                if removed {
                    self.retired_ids.insert(id);
                }
            }
            JournalRecordOutcome::ResyncOnly => {
                self.retired_ids.clear();
            }
        }
    }

    fn clear_entries(&mut self) {
        self.keys.clear();
        self.entries.clear();
        self.retained_payload_bytes = 0;
    }

    fn clear_for_counter_overflow(&mut self) {
        self.clear_entries();
        self.generation = 0;
        self.touch_ordinal = 0;
        self.retired_ids.clear();
        let _ = self.journal.reset_epoch();
    }
}

impl Default for TextLayoutStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowed shaped text layout entry.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTextLayout<'a> {
    /// Text layout handle.
    pub id: TextLayoutId,
    /// Layout request used to shape the text.
    pub key: &'a TextLayoutKey,
    /// Shaped layout.
    pub layout: Arc<ShapedTextLayout>,
}

struct RetainedLayout {
    key: Arc<TextLayoutKey>,
    layout: Arc<ShapedTextLayout>,
    payload_bytes: usize,
    last_generation: u64,
    touch_ordinal: u64,
}

#[derive(Clone, Copy)]
struct LayoutStorePolicy {
    max_payload_bytes: usize,
    max_idle_generations: u64,
    journal_bytes: usize,
}

impl Default for LayoutStorePolicy {
    fn default() -> Self {
        Self {
            max_payload_bytes: MAX_RETAINED_LAYOUT_PAYLOAD_BYTES,
            max_idle_generations: MAX_IDLE_LAYOUT_GENERATIONS,
            journal_bytes: MAX_LAYOUT_CHANGE_JOURNAL_BYTES,
        }
    }
}

enum JournalRecordOutcome {
    Recorded,
    EpochReset,
    ResyncOnly,
}

struct LayoutChangeJournal {
    incarnation: u64,
    epoch: u64,
    revision: u64,
    records: Option<Box<[TextLayoutChange]>>,
    len: usize,
    byte_limit: usize,
    resync_only: bool,
}

impl LayoutChangeJournal {
    fn new(byte_limit: usize) -> Self {
        Self::with_incarnation(byte_limit, next_store_incarnation())
    }

    fn with_incarnation(byte_limit: usize, incarnation: Option<u64>) -> Self {
        Self {
            incarnation: incarnation.unwrap_or(0),
            epoch: 0,
            revision: 0,
            records: None,
            len: 0,
            byte_limit,
            resync_only: incarnation.is_none(),
        }
    }

    const fn cursor(&self) -> TextLayoutChangeCursor {
        TextLayoutChangeCursor {
            incarnation: self.incarnation,
            epoch: self.epoch,
            revision: self.revision,
            resync_only: self.resync_only,
        }
    }

    fn changes_since(&self, cursor: TextLayoutChangeCursor) -> TextLayoutChanges<'_> {
        let current = self.cursor();
        if self.resync_only
            || cursor.resync_only
            || cursor.incarnation != self.incarnation
            || cursor.epoch != self.epoch
            || cursor.revision > self.revision
        {
            return TextLayoutChanges {
                requires_reset: true,
                cursor: current,
                changes: &[],
            };
        }

        let Some(start) = usize::try_from(cursor.revision).ok() else {
            return TextLayoutChanges {
                requires_reset: true,
                cursor: current,
                changes: &[],
            };
        };
        let Some(records) = self.records.as_deref() else {
            return TextLayoutChanges {
                requires_reset: false,
                cursor: current,
                changes: &[],
            };
        };
        let Some(changes) = records.get(start..self.len) else {
            return TextLayoutChanges {
                requires_reset: true,
                cursor: current,
                changes: &[],
            };
        };
        TextLayoutChanges {
            requires_reset: false,
            cursor: current,
            changes,
        }
    }

    fn record(&mut self, id: TextLayoutId) -> JournalRecordOutcome {
        if self.resync_only {
            return JournalRecordOutcome::ResyncOnly;
        }
        if self.ensure_records().is_none() {
            self.enter_resync_only();
            return JournalRecordOutcome::ResyncOnly;
        }

        let capacity = self.records.as_ref().map_or(0, |records| records.len());
        let revision = self.revision.checked_add(1);
        let mut outcome = JournalRecordOutcome::Recorded;
        if self.len == capacity || revision.is_none() {
            outcome = self.reset_epoch();
            if matches!(outcome, JournalRecordOutcome::ResyncOnly) {
                return outcome;
            }
        }

        let Some(revision) = self.revision.checked_add(1) else {
            self.enter_resync_only();
            return JournalRecordOutcome::ResyncOnly;
        };
        let Some(records) = self.records.as_deref_mut() else {
            self.enter_resync_only();
            return JournalRecordOutcome::ResyncOnly;
        };
        let Some(slot) = records.get_mut(self.len) else {
            self.enter_resync_only();
            return JournalRecordOutcome::ResyncOnly;
        };
        *slot = TextLayoutChange { revision, id };
        self.len += 1;
        self.revision = revision;
        outcome
    }

    fn reset_epoch(&mut self) -> JournalRecordOutcome {
        if self.resync_only {
            return JournalRecordOutcome::ResyncOnly;
        }
        let Some(epoch) = self.epoch.checked_add(1) else {
            self.enter_resync_only();
            return JournalRecordOutcome::ResyncOnly;
        };
        self.epoch = epoch;
        self.revision = 0;
        self.len = 0;
        JournalRecordOutcome::EpochReset
    }

    fn ensure_records(&mut self) -> Option<()> {
        if self.records.is_some() {
            return Some(());
        }
        let record_size = size_of::<TextLayoutChange>();
        let capacity = self.byte_limit.checked_div(record_size)?;
        if capacity == 0 {
            return None;
        }
        let empty = TextLayoutChange {
            revision: 0,
            id: REJECTED_LAYOUT_ID,
        };
        self.records = Some(vec![empty; capacity].into_boxed_slice());
        Some(())
    }

    fn enter_resync_only(&mut self) {
        self.resync_only = true;
        self.records = None;
        self.len = 0;
        self.revision = 0;
    }

    #[cfg(test)]
    fn retained_bytes(&self) -> usize {
        self.records
            .as_ref()
            .map_or(0, |records| records.len() * size_of::<TextLayoutChange>())
    }
}

fn next_store_incarnation() -> Option<u64> {
    NEXT_STORE_INCARNATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(1)
        })
        .ok()
        .filter(|incarnation| *incarnation != 0)
}

fn normalize_key(mut key: TextLayoutKey) -> TextLayoutKey {
    key.text = key.text.into_boxed_str().into_string();
    key.style.family = key.style.family.into_boxed_str().into_string();
    key
}

fn retained_key_payload_bytes(key: &TextLayoutKey) -> Option<usize> {
    checked_payload_sum([
        size_of::<TextLayoutKey>(),
        key.text.capacity(),
        key.style.family.capacity(),
    ])
}

fn retained_layout_payload_bytes(key: &TextLayoutKey, layout: &ShapedTextLayout) -> Option<usize> {
    let mut bytes = retained_key_payload_bytes(key)?;
    bytes = bytes.checked_add(size_of::<ShapedTextLayout>())?;
    bytes = bytes.checked_add(
        layout
            .lines
            .capacity()
            .checked_mul(size_of::<ShapedTextLine>())?,
    )?;
    bytes = bytes.checked_add(
        layout
            .runs
            .capacity()
            .checked_mul(size_of::<ShapedGlyphRun>())?,
    )?;
    for run in &layout.runs {
        bytes = bytes.checked_add(
            run.normalized_coords
                .capacity()
                .checked_mul(size_of::<i16>())?,
        )?;
        bytes = bytes.checked_add(
            run.glyphs
                .capacity()
                .checked_mul(size_of::<ShapedGlyph>())?,
        )?;
    }
    Some(bytes)
}

fn checked_payload_sum(parts: impl IntoIterator<Item = usize>) -> Option<usize> {
    parts.into_iter().try_fold(0_usize, usize::checked_add)
}

fn text_layout_id(key: &TextLayoutKey) -> TextLayoutId {
    let mut hasher = StableHasher::new();
    key.hash(&mut hasher);
    TextLayoutId::from_raw(hasher.finish().max(1))
}

struct StableHasher(u64);

impl StableHasher {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    const fn new() -> Self {
        Self(Self::OFFSET)
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(u64::try_from(i).unwrap_or(u64::MAX));
    }
}

#[cfg(test)]
mod budget_tests {
    use std::mem::size_of;
    use std::sync::Arc;

    use super::*;
    use crate::TextStyle;

    fn key(text: &str) -> TextLayoutKey {
        TextLayoutKey::new(text, TextStyle::new("Inter", 12.0, 16.0), 100.0, false)
    }

    fn cost(key: &TextLayoutKey) -> usize {
        let key = normalize_key(key.clone());
        let mut engine = CosmicTextEngine::new();
        let layout = engine.shape_text(&key);
        retained_layout_payload_bytes(&key, &layout).expect("small layout cost")
    }

    fn policy(max_payload_bytes: usize, max_idle_generations: u64) -> LayoutStorePolicy {
        LayoutStorePolicy {
            max_payload_bytes,
            max_idle_generations,
            journal_bytes: MAX_LAYOUT_CHANGE_JOURNAL_BYTES,
        }
    }

    #[test]
    fn exact_payload_limit_is_inclusive_and_one_byte_under_rejects() {
        let request = key("exact");
        let exact_cost = cost(&request);
        let mut exact = TextLayoutStore::with_policy(policy(exact_cost, 120));
        assert!(exact.try_layout_id(request.clone()).is_some());
        assert_eq!(exact.retained_payload_bytes(), exact_cost);

        let mut under = TextLayoutStore::with_policy(policy(exact_cost - 1, 120));
        let cursor = under.change_cursor();
        assert_eq!(under.try_layout_id(request), None);
        assert!(under.is_empty());
        assert_eq!(under.retained_payload_bytes(), 0);
        assert_eq!(under.change_cursor(), cursor);
        assert!(checked_payload_sum([usize::MAX, 1]).is_none());
    }

    #[test]
    fn generation_120_is_inclusive_and_121_evicts() {
        let mut store = TextLayoutStore::new();
        let id = store.layout_id(key("age"));
        let cursor = store.change_cursor();
        for _ in 0..120 {
            store.advance_generation();
        }
        assert!(store.layout(id).is_some());
        assert!(store.changes_since(cursor).iter().next().is_none());

        store.advance_generation();
        assert!(store.layout(id).is_none());
        let changes = store.changes_since(cursor).iter().collect::<Vec<_>>();
        assert_eq!(
            changes.iter().map(|change| change.id()).collect::<Vec<_>>(),
            [id]
        );
    }

    #[test]
    fn hit_at_generation_119_restarts_the_idle_window_without_journaling() {
        let mut store = TextLayoutStore::new();
        let request = key("hot");
        let id = store.layout_id(request.clone());
        for _ in 0..119 {
            store.advance_generation();
        }
        let cursor = store.change_cursor();
        assert_eq!(store.try_layout_id(request), Some(id));
        assert_eq!(store.change_cursor(), cursor);
        for _ in 0..120 {
            store.advance_generation();
        }
        assert!(store.layout(id).is_some());
        store.advance_generation();
        assert!(store.layout(id).is_none());
    }

    #[test]
    fn current_generation_pins_reject_transactionally_then_lru_evicts() {
        let a = key("a");
        let b = key("b");
        let c = key("c");
        let entry_cost = cost(&a);
        assert_eq!(cost(&b), entry_cost);
        assert_eq!(cost(&c), entry_cost);
        let mut store = TextLayoutStore::with_policy(policy(entry_cost * 2, 120));
        let a_id = store.layout_id(a.clone());
        let b_id = store.layout_id(b);
        let bytes = store.retained_payload_bytes();
        let cursor = store.change_cursor();

        assert_eq!(store.try_layout_id(c.clone()), None);
        assert_eq!(store.len(), 2);
        assert_eq!(store.retained_payload_bytes(), bytes);
        assert_eq!(store.change_cursor(), cursor);

        store.advance_generation();
        assert_eq!(store.try_layout_id(a), Some(a_id));
        let before_pressure = store.change_cursor();
        let c_id = store
            .try_layout_id(c)
            .expect("older cold entry can be evicted");
        assert!(store.layout(a_id).is_some());
        assert!(store.layout(b_id).is_none());
        assert!(store.layout(c_id).is_some());
        assert_eq!(store.retained_payload_bytes(), bytes);
        assert_eq!(
            store
                .changes_since(before_pressure)
                .iter()
                .map(TextLayoutChange::id)
                .collect::<Vec<_>>(),
            [b_id, c_id]
        );
    }

    #[test]
    fn collision_ids_are_retired_until_reset() {
        let mut store = TextLayoutStore::with_policy(policy(usize::MAX, 1));
        let preferred = TextLayoutId::from_raw(42);
        let a = key("first");
        let b = key("second");
        assert_eq!(
            store.layout_id_with_preferred_id(a.clone(), preferred),
            preferred
        );
        let b_id = store.layout_id_with_preferred_id(b.clone(), preferred);
        assert_eq!(b_id, TextLayoutId::from_raw(43));

        store.advance_generation();
        assert!(store.touch_layout(b_id));
        store.advance_generation();
        assert!(store.layout(preferred).is_none());
        assert_eq!(
            store.layout_id_with_preferred_id(a.clone(), preferred),
            TextLayoutId::from_raw(44)
        );

        store.clear();
        assert_eq!(store.layout_id_with_preferred_id(a, preferred), preferred);
    }

    #[test]
    fn journal_is_fixed_bounded_and_overflow_requires_reset() {
        let record_bytes = size_of::<TextLayoutChange>();
        let mut store = TextLayoutStore::with_policy(LayoutStorePolicy {
            max_payload_bytes: usize::MAX,
            max_idle_generations: 120,
            journal_bytes: record_bytes * 2,
        });
        let initial = store.change_cursor();
        store.layout_id(key("a"));
        store.layout_id(key("b"));
        assert_eq!(store.changes_since(initial).iter().count(), 2);
        assert_eq!(store.journal.retained_bytes(), record_bytes * 2);
        let full = store.change_cursor();

        store.layout_id(key("c"));
        assert!(store.changes_since(full).requires_reset());
        assert!(store.journal.retained_bytes() <= record_bytes * 2);
        let current = store.change_cursor();
        let no_op = store.changes_since(current);
        assert!(!no_op.requires_reset());
        assert_eq!(no_op.iter().count(), 0);
    }

    #[test]
    fn foreign_store_cursors_require_reset_at_equal_revision() {
        let mut first = TextLayoutStore::new();
        let mut second = TextLayoutStore::new();
        first.layout_id(key("same"));
        second.layout_id(key("same"));
        let first_cursor = first.change_cursor();
        let second_cursor = second.change_cursor();
        assert_ne!(first_cursor, second_cursor);
        assert!(first.changes_since(second_cursor).requires_reset());
        assert!(second.changes_since(first_cursor).requires_reset());
        assert!(!first.changes_since(first_cursor).requires_reset());
    }

    #[test]
    fn terminal_journal_exhaustion_preserves_residents_and_tombstones_nothing() {
        let mut store = TextLayoutStore::with_policy(LayoutStorePolicy {
            max_payload_bytes: usize::MAX,
            max_idle_generations: 120,
            journal_bytes: size_of::<TextLayoutChange>(),
        });
        let first = store.layout_id_with_preferred_id(key("a"), TextLayoutId::from_raw(42));
        store.journal.epoch = u64::MAX;
        let second = store.layout_id_with_preferred_id(key("b"), TextLayoutId::from_raw(43));
        assert!(store.journal.resync_only);
        assert!(store.layout(first).is_some());
        assert!(store.layout(second).is_some());
        assert!(store.retired_ids.is_empty());
        assert!(store.changes_since(store.change_cursor()).requires_reset());
        assert_eq!(store.journal.retained_bytes(), 0);

        store.remove_entry(first, true);
        assert!(store.retired_ids.is_empty());
        assert_eq!(
            store.layout_id_with_preferred_id(key("c"), first),
            first,
            "terminal full-sync mode may reuse removed IDs"
        );
    }

    #[test]
    fn transient_shape_and_observations_do_not_touch_or_journal() {
        let mut store = TextLayoutStore::new();
        let request = key("observe");
        let id = store.layout_id(request.clone());
        let cursor = store.change_cursor();
        let bytes = store.retained_payload_bytes();
        let _ = store.shape_transient(&key("transient"));
        let _ = store.layout(id);
        let _ = store.stored_layout(id);
        let _ = store.layouts().collect::<Vec<_>>();
        assert_eq!(store.len(), 1);
        assert_eq!(store.retained_payload_bytes(), bytes);
        assert_eq!(store.change_cursor(), cursor);

        store.advance_generation();
        assert!(store.touch_layout(id));
        assert_eq!(store.change_cursor(), cursor);
        assert_eq!(store.try_layout_id(request), Some(id));
    }

    #[test]
    fn external_arc_survives_eviction_and_full_iteration_is_id_sorted() {
        let mut store = TextLayoutStore::with_policy(policy(usize::MAX, 0));
        let high = store.layout_id_with_preferred_id(key("high"), TextLayoutId::from_raw(90));
        let low = store.layout_id_with_preferred_id(key("low"), TextLayoutId::from_raw(10));
        let exported = store.stored_layout(high).expect("exported").layout;
        assert_eq!(
            store.layouts().map(|entry| entry.id).collect::<Vec<_>>(),
            [low, high]
        );
        let glyphs = exported.glyph_count();
        store.advance_generation();
        assert!(store.is_empty());
        assert_eq!(store.retained_payload_bytes(), 0);
        assert_eq!(exported.glyph_count(), glyphs);
        assert_eq!(Arc::strong_count(&exported), 1);
    }

    #[test]
    fn generation_and_touch_overflow_paths_never_wrap() {
        let mut store = TextLayoutStore::new();
        let id = store.layout_id(key("overflow"));
        store.touch_ordinal = u64::MAX;
        assert!(store.touch_layout(id));
        assert!(store.touch_ordinal < u64::MAX);

        let old_cursor = store.change_cursor();
        store.generation = u64::MAX;
        store.advance_generation();
        assert_eq!(store.generation(), 0);
        assert!(store.is_empty());
        assert!(store.changes_since(old_cursor).requires_reset());
    }

    #[test]
    fn id_probe_at_u64_max_rejects_without_wrapping_or_mutation() {
        let mut store = TextLayoutStore::with_policy(policy(usize::MAX, 0));
        let max_id = TextLayoutId::from_raw(u64::MAX);
        let resident = key("resident");
        assert_eq!(store.layout_id_with_preferred_id(resident, max_id), max_id);

        let occupied_cursor = store.change_cursor();
        let occupied_bytes = store.retained_payload_bytes();
        assert_eq!(
            store.try_layout_id_with_preferred_id(key("occupied"), max_id),
            None
        );
        assert_eq!(
            store.layout_id_with_preferred_id(key("occupied sentinel"), max_id),
            REJECTED_LAYOUT_ID
        );
        assert_eq!(store.len(), 1);
        assert_eq!(store.retained_payload_bytes(), occupied_bytes);
        assert_eq!(store.change_cursor(), occupied_cursor);
        assert!(store.layout(max_id).is_some());

        store.advance_generation();
        assert!(store.is_empty());
        assert!(store.retired_ids.contains(&max_id));
        let retired_cursor = store.change_cursor();
        assert_eq!(
            store.try_layout_id_with_preferred_id(key("retired"), max_id),
            None
        );
        assert_eq!(
            store.layout_id_with_preferred_id(key("retired sentinel"), max_id),
            REJECTED_LAYOUT_ID
        );
        assert!(store.is_empty());
        assert_eq!(store.retained_payload_bytes(), 0);
        assert_eq!(store.change_cursor(), retired_cursor);
        assert!(store.retired_ids.contains(&max_id));
    }

    #[test]
    fn touch_overflow_empty_and_pressure_admission_are_transactional() {
        let request = key("a");
        let entry_cost = cost(&request);
        let mut empty = TextLayoutStore::with_policy(policy(entry_cost, 120));
        empty.touch_ordinal = u64::MAX;
        let admitted = empty.try_layout_id(request).expect("empty admission");
        assert_eq!(empty.touch_ordinal, 1);
        assert_eq!(empty.entries[&admitted].touch_ordinal, 1);
        assert_eq!(empty.retained_payload_bytes(), entry_cost);

        let a = key("a");
        let b = key("b");
        assert_eq!(cost(&b), entry_cost);
        let mut pressure = TextLayoutStore::with_policy(policy(entry_cost, 120));
        let a_id = pressure.layout_id(a);
        pressure.advance_generation();
        pressure.touch_ordinal = u64::MAX;
        let cursor = pressure.change_cursor();
        let b_id = pressure.try_layout_id(b).expect("replacement admission");
        assert_eq!(pressure.touch_ordinal, 2);
        assert_eq!(pressure.len(), 1);
        assert_eq!(pressure.retained_payload_bytes(), entry_cost);
        assert!(pressure.layout(a_id).is_none());
        assert!(pressure.layout(b_id).is_some());
        assert_eq!(pressure.entries[&b_id].touch_ordinal, 2);
        assert_eq!(
            pressure
                .changes_since(cursor)
                .iter()
                .map(TextLayoutChange::id)
                .collect::<Vec<_>>(),
            [a_id, b_id]
        );
    }

    #[test]
    fn equal_lru_metadata_uses_layout_id_as_the_final_store_tie_break() {
        let a = key("aa");
        let b = key("bb");
        let c = key("cc");
        let entry_cost = cost(&a);
        assert_eq!(cost(&b), entry_cost);
        assert_eq!(cost(&c), entry_cost);
        let mut store = TextLayoutStore::with_policy(policy(entry_cost * 2, 120));
        let high = store.layout_id_with_preferred_id(a, TextLayoutId::from_raw(90));
        let low = store.layout_id_with_preferred_id(b, TextLayoutId::from_raw(10));
        store.advance_generation();
        for id in [high, low] {
            let entry = store.entries.get_mut(&id).expect("resident");
            entry.last_generation = 0;
            entry.touch_ordinal = 7;
        }

        let replacement = store
            .try_layout_id_with_preferred_id(c, TextLayoutId::from_raw(50))
            .expect("pressure admission");
        assert!(store.layout(low).is_none(), "lower raw ID loses the tie");
        assert!(store.layout(high).is_some());
        assert!(store.layout(replacement).is_some());
        assert_eq!(store.retained_payload_bytes(), entry_cost * 2);
    }

    #[test]
    fn same_store_cursors_cover_incremental_future_and_revision_overflow_boundaries() {
        let mut store = TextLayoutStore::new();
        let initial = store.change_cursor();
        let a = store.layout_id(key("a"));
        let after_a = store.change_cursor();
        let b = store.layout_id(key("b"));
        assert_eq!(
            store
                .changes_since(initial)
                .iter()
                .map(TextLayoutChange::id)
                .collect::<Vec<_>>(),
            [a, b]
        );
        assert_eq!(
            store
                .changes_since(after_a)
                .iter()
                .map(TextLayoutChange::id)
                .collect::<Vec<_>>(),
            [b]
        );

        let mut future = store.change_cursor();
        future.revision = future.revision.checked_add(1).expect("small revision");
        assert!(store.changes_since(future).requires_reset());

        let before_overflow = store.change_cursor();
        store.journal.revision = u64::MAX;
        let c = store.layout_id(key("c"));
        assert!(store.changes_since(before_overflow).requires_reset());
        let current = store.changes_since(store.change_cursor());
        assert!(!current.requires_reset());
        assert_eq!(current.iter().count(), 0);
        assert_eq!(store.journal.epoch, before_overflow.epoch + 1);
        assert_eq!(store.journal.revision, 1);
        assert_eq!(
            store.journal.records.as_deref().expect("journal")[0].id(),
            c
        );
    }

    #[test]
    fn injected_incarnation_exhaustion_enters_bounded_resync_only_mode() {
        let request = key("resident");
        let entry_cost = cost(&request);
        let mut store = TextLayoutStore::with_policy(policy(entry_cost, 0));
        store.journal =
            LayoutChangeJournal::with_incarnation(MAX_LAYOUT_CHANGE_JOURNAL_BYTES, None);
        let cursor = store.change_cursor();
        let id = store
            .try_layout_id(request)
            .expect("resident still admitted");
        assert!(store.layout(id).is_some());
        assert_eq!(store.retained_payload_bytes(), entry_cost);
        assert_eq!(store.journal.retained_bytes(), 0);
        assert!(store.changes_since(cursor).requires_reset());
        assert!(store.retired_ids.is_empty());

        store.advance_generation();
        assert!(store.is_empty());
        assert_eq!(store.retained_payload_bytes(), 0);
        assert!(store.retired_ids.is_empty());
        assert_eq!(store.journal.retained_bytes(), 0);
    }

    #[test]
    fn production_journal_capacity_is_exact_and_overflow_starts_a_new_epoch() {
        let record_bytes = size_of::<TextLayoutChange>();
        let capacity = MAX_LAYOUT_CHANGE_JOURNAL_BYTES / record_bytes;
        let mut journal = LayoutChangeJournal::new(MAX_LAYOUT_CHANGE_JOURNAL_BYTES);
        let initial = journal.cursor();
        for raw in 1..=capacity {
            let id = TextLayoutId::from_raw(u64::try_from(raw).expect("capacity fits u64"));
            assert!(matches!(journal.record(id), JournalRecordOutcome::Recorded));
        }
        assert_eq!(journal.len, capacity);
        assert_eq!(journal.retained_bytes(), capacity * record_bytes);
        assert!(journal.retained_bytes() <= MAX_LAYOUT_CHANGE_JOURNAL_BYTES);
        assert_eq!(journal.changes_since(initial).iter().count(), capacity);

        let full = journal.cursor();
        assert!(matches!(
            journal.record(TextLayoutId::from_raw(u64::MAX)),
            JournalRecordOutcome::EpochReset
        ));
        assert!(journal.changes_since(full).requires_reset());
        assert_eq!(journal.len, 1);
        assert_eq!(journal.revision, 1);
        assert_eq!(journal.retained_bytes(), capacity * record_bytes);
    }

    #[test]
    fn retained_key_payload_counts_exact_utf8_bytes() {
        let cases = [("a", 1_usize), ("e\u{301}", 3), ("😀", 4), ("👩‍👩‍👧‍👦", 25)];
        for (text, expected_utf8_bytes) in cases {
            assert_eq!(text.len(), expected_utf8_bytes);
            let request = normalize_key(key(text));
            assert_eq!(request.text.capacity(), request.text.len());
            assert_eq!(request.style.family.capacity(), request.style.family.len());
            assert_eq!(
                retained_key_payload_bytes(&request),
                Some(size_of::<TextLayoutKey>() + expected_utf8_bytes + request.style.family.len())
            );
        }
    }
}
