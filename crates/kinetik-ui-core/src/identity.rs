//! Stable widget identity.

use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

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
    #[must_use]
    pub fn from_key(key: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Creates a child ID from this ID and a stable child key.
    #[must_use]
    pub fn child(self, key: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
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
    seen: HashSet<WidgetId>,
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

    /// Registers an ID for duplicate detection.
    pub fn register(&mut self, id: WidgetId) {
        if !self.seen.insert(id) {
            self.duplicates.push(DuplicateWidgetId { id });
        }
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
        self.duplicates.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{IdStack, WidgetId};

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
