//! Stable widget identity.
//!
//! `WidgetId` derivation hashes through a pinned FNV-1a 64-bit implementation
//! instead of `std`'s `DefaultHasher`, whose output is unspecified across Rust
//! releases. IDs derived from the same keys are bit-identical across compiler
//! versions, processes, and target platforms; golden conformance tests pin the
//! exact values.

use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Pinned FNV-1a 64-bit hasher for stable widget identity.
///
/// Every integer is folded through its little-endian byte encoding regardless
/// of platform endianness, so a hash depends only on the key value and the
/// fixed constants above. Stability of [`WidgetId::from_key`] additionally
/// relies on the key type's own [`Hash`] implementation routing to stable
/// inputs; if `std` ever changes how a standard type hashes, the golden-value
/// conformance tests fail loudly instead of silently shifting persisted IDs.
struct StableIdHasher(u64);

impl StableIdHasher {
    const fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    fn mix_byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(FNV_PRIME);
    }
}

impl Hasher for StableIdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.mix_byte(byte);
        }
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.mix_byte(i);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write(&(i as u64).to_le_bytes());
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i.cast_unsigned());
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.write(&i.to_le_bytes());
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write(&(i as i64).to_le_bytes());
    }
}

/// Stable identity for a stateful widget.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Creates a widget ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Creates a widget ID by hashing a stable key.
    ///
    /// The derivation is pinned: the same key yields the same ID across
    /// compiler releases, processes, and platforms, so IDs may be persisted or
    /// compared across builds. See [`StableIdHasher`] for the exact scheme and
    /// its limits.
    #[must_use]
    pub fn from_key(key: impl Hash) -> Self {
        let mut hasher = StableIdHasher::new();
        key.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Creates a child ID from this ID and a stable child key.
    ///
    /// Child derivation is equally pinned: the parent's raw bits are hashed as
    /// little-endian bytes followed by the child key's hash input.
    #[must_use]
    pub fn child(self, key: impl Hash) -> Self {
        let mut hasher = StableIdHasher::new();
        self.hash(&mut hasher);
        key.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl fmt::Debug for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetId({:#018x})", self.0)
    }
}

/// Duplicate widget ID detected during a frame or scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateWidgetId {
    /// The duplicated ID.
    pub id: WidgetId,
}

/// Scoped widget ID stack.
#[derive(Debug, Clone)]
pub struct IdStack {
    stack: Vec<WidgetId>,
    /// Widget identities proven present during this frame.
    seen: HashSet<WidgetId>,
    /// Normal registrations used for duplicate diagnostics.
    registrations: HashSet<WidgetId>,
    duplicates: Vec<DuplicateWidgetId>,
}

impl Default for IdStack {
    fn default() -> Self {
        Self::new()
    }
}

impl IdStack {
    /// Creates an empty ID stack.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stack: vec![WidgetId::from_key("root")],
            seen: HashSet::new(),
            registrations: HashSet::new(),
            duplicates: Vec::new(),
        }
    }

    /// Returns the current parent ID.
    #[must_use]
    pub fn current(&self) -> WidgetId {
        self.stack
            .last()
            .copied()
            .unwrap_or_else(|| WidgetId::from_key("root"))
    }

    /// Derives an ID from the current scope and a stable key.
    #[must_use]
    pub fn make_id(&self, key: impl Hash) -> WidgetId {
        self.current().child(key)
    }

    /// Pushes a scope and returns its ID.
    pub fn push(&mut self, key: impl Hash) -> WidgetId {
        let id = self.make_id(key);
        self.register(id);
        self.stack.push(id);
        id
    }

    /// Pops the current scope.
    ///
    /// The root scope cannot be popped.
    pub fn pop(&mut self) -> Option<WidgetId> {
        if self.stack.len() <= 1 {
            None
        } else {
            self.stack.pop()
        }
    }

    /// Runs a closure inside an ID scope and restores the previous scope.
    pub fn with_scope<T>(&mut self, key: impl Hash, f: impl FnOnce(&mut Self) -> T) -> T {
        self.push(key);
        let result = f(self);
        self.pop();
        result
    }

    /// Registers an ID as present and tracks duplicate normal registrations.
    pub fn register(&mut self, id: WidgetId) {
        self.mark_seen(id);
        if !self.registrations.insert(id) {
            self.duplicates.push(DuplicateWidgetId { id });
        }
    }

    /// Marks an ID present without creating a duplicate-registration diagnostic.
    pub(crate) fn mark_seen(&mut self, id: WidgetId) {
        self.seen.insert(id);
    }

    /// Returns whether an ID was proven present during this frame.
    pub(crate) fn was_seen(&self, id: WidgetId) -> bool {
        self.seen.contains(&id)
    }

    /// Derives and registers an ID from the current scope.
    pub fn register_key(&mut self, key: impl Hash) -> WidgetId {
        let id = self.make_id(key);
        self.register(id);
        id
    }

    /// Returns duplicates detected so far.
    #[must_use]
    pub fn duplicates(&self) -> &[DuplicateWidgetId] {
        &self.duplicates
    }

    /// Clears per-frame duplicate tracking while preserving the scope stack.
    pub fn clear_frame_tracking(&mut self) {
        self.seen.clear();
        self.registrations.clear();
        self.duplicates.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{IdStack, StableIdHasher, WidgetId};
    use std::hash::Hasher;

    #[test]
    fn fnv1a_matches_published_reference_vectors() {
        let mut empty = StableIdHasher::new();
        empty.write(b"");
        assert_eq!(empty.finish(), 0xcbf2_9ce4_8422_2325);

        let mut a = StableIdHasher::new();
        a.write(b"a");
        assert_eq!(a.finish(), 0xaf63_dc4c_8601_ec8c);

        let mut foobar = StableIdHasher::new();
        foobar.write(b"foobar");
        assert_eq!(foobar.finish(), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn widget_id_from_key_pins_golden_values() {
        assert_eq!(WidgetId::from_key("").raw(), 0xaf64_724c_8602_eb6e);
        assert_eq!(WidgetId::from_key("root").raw(), 0x4dd1_5746_1dbb_678e);
        assert_eq!(WidgetId::from_key("button").raw(), 0xed6c_366e_8b17_da4a);
        assert_eq!(WidgetId::from_key("slider").raw(), 0x23f0_7742_b581_cdb9);
    }

    #[test]
    fn widget_id_child_pins_golden_values() {
        let panel = WidgetId::from_key("panel");

        assert_eq!(
            panel.child("field").raw(),
            0x385f_66bf_8b4b_5662,
            "child derivation must stay pinned for persisted IDs"
        );
        assert_eq!(panel.child("other").raw(), 0xee47_df4a_d6b6_43b4);
    }

    #[test]
    fn integer_keys_hash_endianness_independently() {
        let from_u64 = WidgetId::from_key(0x0123_4567_89ab_cdef_u64);
        assert_eq!(from_u64.raw(), 0x37eb_3f33_4776_1c55);
    }

    #[test]
    fn widget_id_from_key_is_stable() {
        assert_eq!(WidgetId::from_key("button"), WidgetId::from_key("button"));
        assert_ne!(WidgetId::from_key("button"), WidgetId::from_key("slider"));
    }

    #[test]
    fn child_ids_are_stable_and_scoped() {
        let parent = WidgetId::from_key("panel");

        assert_eq!(parent.child("field"), parent.child("field"));
        assert_ne!(parent.child("field"), WidgetId::from_key("field"));
    }

    #[test]
    fn id_stack_restores_scope_after_closure() {
        let mut stack = IdStack::new();
        let root_child = stack.make_id("field");

        let scoped_child = stack.with_scope("panel", |stack| stack.make_id("field"));

        assert_ne!(root_child, scoped_child);
        assert_eq!(stack.make_id("field"), root_child);
    }

    #[test]
    fn pop_does_not_remove_root_scope() {
        let mut stack = IdStack::new();

        assert_eq!(stack.pop(), None);
        assert_eq!(stack.current(), IdStack::new().current());
    }

    #[test]
    fn detects_duplicate_registered_ids() {
        let mut stack = IdStack::new();
        let id = stack.register_key("search");
        stack.register(id);

        assert_eq!(stack.duplicates().len(), 1);
        assert_eq!(stack.duplicates()[0].id, id);
    }

    #[test]
    fn presence_evidence_is_distinct_from_duplicate_registration() {
        let mut stack = IdStack::new();
        let id = stack.make_id("custom");

        stack.mark_seen(id);
        stack.mark_seen(id);

        assert!(stack.was_seen(id));
        assert!(stack.duplicates().is_empty());
        stack.register(id);
        stack.register(id);
        assert_eq!(stack.duplicates().len(), 1);
    }

    #[test]
    fn detects_duplicate_scope_ids() {
        let mut stack = IdStack::new();
        let id = stack.push("panel");
        stack.pop();
        let duplicate = stack.push("panel");

        assert_eq!(duplicate, id);
        assert_eq!(stack.duplicates().len(), 1);
        assert_eq!(stack.duplicates()[0].id, id);
    }

    #[test]
    fn clear_frame_tracking_preserves_scope() {
        let mut stack = IdStack::new();
        stack.push("panel");
        let scoped_id = stack.make_id("field");

        stack.register(scoped_id);
        stack.register(scoped_id);
        stack.clear_frame_tracking();

        assert!(stack.duplicates().is_empty());
        assert_eq!(stack.make_id("field"), scoped_id);
    }
}
