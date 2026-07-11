//! Deterministic liveness tokens for retained UI targets.

use std::{
    collections::HashMap,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::WidgetId;

/// Platform-neutral identity for a UI target whose external updates need
/// deterministic liveness validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LivenessTargetId(WidgetId);

impl LivenessTargetId {
    /// Creates a liveness target from a stable widget identity.
    #[must_use]
    pub const fn new(widget_id: WidgetId) -> Self {
        Self(widget_id)
    }

    /// Returns the widget identity backing this liveness target.
    #[must_use]
    pub const fn widget_id(self) -> WidgetId {
        self.0
    }
}

impl From<WidgetId> for LivenessTargetId {
    fn from(value: WidgetId) -> Self {
        Self::new(value)
    }
}

/// Registry-wide incarnation assigned when async ownership begins or restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LivenessIncarnation(u64);

impl LivenessIncarnation {
    /// First incarnation assigned by a registry.
    pub const FIRST: Self = Self(1);

    /// Returns the numeric incarnation value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Deprecated name for [`LivenessIncarnation`].
#[deprecated(note = "renamed to LivenessIncarnation")]
pub type LivenessGeneration = LivenessIncarnation;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RegistryScope(u64);

static NEXT_REGISTRY_SCOPE: AtomicU64 = AtomicU64::new(1);

fn checked_successor(value: u64) -> Option<u64> {
    value.checked_add(1)
}

fn allocate_registry_scope() -> RegistryScope {
    let value = NEXT_REGISTRY_SCOPE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, checked_successor)
        .expect("liveness registry scope overflow");
    RegistryScope(value)
}

/// Opaque proof that an external update was issued for one registry-owned
/// target incarnation.
///
/// Tokens can only be minted by [`LivenessRegistry`]. Their private registry
/// scope prevents a token from one registry from authorizing another.
///
/// ```compile_fail
/// use kinetik_ui_core::{
///     LivenessIncarnation, LivenessTargetId, LivenessToken, WidgetId,
/// };
///
/// let _token = LivenessToken::new(
///     LivenessTargetId::new(WidgetId::from_key("preview")),
///     LivenessIncarnation::FIRST,
/// );
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LivenessToken {
    registry_scope: RegistryScope,
    target: LivenessTargetId,
    incarnation: LivenessIncarnation,
}

impl LivenessToken {
    fn new(
        registry_scope: RegistryScope,
        target: LivenessTargetId,
        incarnation: LivenessIncarnation,
    ) -> Self {
        Self {
            registry_scope,
            target,
            incarnation,
        }
    }

    /// Returns the target identity carried by this token.
    #[must_use]
    pub const fn target(self) -> LivenessTargetId {
        self.target
    }

    /// Returns the incarnation carried by this token.
    #[must_use]
    pub const fn incarnation(self) -> LivenessIncarnation {
        self.incarnation
    }

    /// Returns the incarnation using the previous generation terminology.
    #[allow(deprecated)]
    #[deprecated(note = "use incarnation")]
    #[must_use]
    pub const fn generation(self) -> LivenessGeneration {
        self.incarnation
    }

    pub(crate) fn observational_eq(self, other: Self) -> bool {
        self.target == other.target && self.incarnation == other.incarnation
    }
}

impl fmt::Debug for LivenessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LivenessToken")
            .field("target", &self.target)
            .field("incarnation", &self.incarnation)
            .finish_non_exhaustive()
    }
}

/// Result of validating or applying a liveness-gated update attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessUpdateStatus {
    /// The token matched the active target incarnation.
    Applied,
    /// The exact incarnation was explicitly cancelled.
    Cancelled {
        /// Target carried by the cancelled token.
        target: LivenessTargetId,
        /// Cancelled incarnation.
        incarnation: LivenessIncarnation,
    },
    /// The token belongs to another registry or the target has no retained
    /// incarnation record.
    StaleTarget {
        /// Target carried by the stale token.
        target: LivenessTargetId,
    },
    /// The target has a newer retained incarnation than the token.
    StaleIncarnation {
        /// Target carried by the stale token.
        target: LivenessTargetId,
        /// Incarnation carried by the stale token.
        token_incarnation: LivenessIncarnation,
        /// Latest active or tombstoned incarnation retained for the target.
        current_incarnation: LivenessIncarnation,
    },
}

/// Result of explicitly removing an async owner target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessRemovalStatus {
    /// An active incarnation was removed.
    Removed,
    /// The target had no active incarnation.
    AlreadyAbsent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActiveEntry {
    incarnation: LivenessIncarnation,
    present_this_frame: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TombstoneReason {
    Cancelled,
    Removed,
    Omitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TombstoneEntry {
    incarnation: LivenessIncarnation,
    reason: TombstoneReason,
    retired_epoch: u64,
}

/// Retained async-owner registry owned by [`crate::UiMemory`].
///
/// Presence is frame-local, while an active incarnation remains valid until
/// omission is finalized. Retired incarnations remain as bounded tombstones
/// through one complete following frame.
///
/// Registry equality is observational and deliberately ignores the private
/// authority scope. Equal registries do not accept one another's tokens.
///
/// ```compile_fail
/// use kinetik_ui_core::LivenessRegistry;
///
/// let registry = LivenessRegistry::new();
/// let _authority_copy = registry.clone();
/// ```
pub struct LivenessRegistry {
    registry_scope: RegistryScope,
    last_incarnation: u64,
    epoch: u64,
    active: HashMap<LivenessTargetId, ActiveEntry>,
    tombstones: HashMap<LivenessTargetId, TombstoneEntry>,
}

impl Default for LivenessRegistry {
    fn default() -> Self {
        Self {
            registry_scope: allocate_registry_scope(),
            last_incarnation: 0,
            epoch: 0,
            active: HashMap::new(),
            tombstones: HashMap::new(),
        }
    }
}

impl fmt::Debug for LivenessRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LivenessRegistry")
            .field("last_incarnation", &self.last_incarnation)
            .field("epoch", &self.epoch)
            .field("active", &self.active)
            .field("tombstones", &self.tombstones)
            .finish_non_exhaustive()
    }
}

impl PartialEq for LivenessRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.last_incarnation == other.last_incarnation
            && self.epoch == other.epoch
            && self.active == other.active
            && self.tombstones == other.tombstones
    }
}

impl Eq for LivenessRegistry {}

impl LivenessRegistry {
    /// Creates an empty registry with a unique private authority scope.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when no active incarnations or tombstones remain.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.active.is_empty() && self.tombstones.is_empty()
    }

    /// Returns the total retained active and tombstone record count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.active_count() + self.tombstone_count()
    }

    /// Returns the number of active owner incarnations.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Returns the number of retained tombstones.
    #[must_use]
    pub fn tombstone_count(&self) -> usize {
        self.tombstones.len()
    }

    /// Starts a new presence epoch without invalidating active incarnations.
    pub(crate) fn begin_frame(&mut self) {
        self.epoch = checked_successor(self.epoch).expect("liveness frame epoch overflow");
        for entry in self.active.values_mut() {
            entry.present_this_frame = false;
        }
    }

    /// Retires omitted incarnations and prunes tombstones after one complete
    /// following frame.
    pub(crate) fn end_frame(&mut self) {
        let omitted = self
            .active
            .iter()
            .filter_map(|(target, entry)| {
                (!entry.present_this_frame).then_some((*target, entry.incarnation))
            })
            .collect::<Vec<_>>();

        for (target, incarnation) in omitted {
            self.active.remove(&target);
            self.tombstones.insert(
                target,
                TombstoneEntry {
                    incarnation,
                    reason: TombstoneReason::Omitted,
                    retired_epoch: self.epoch,
                },
            );
        }

        self.tombstones
            .retain(|_, entry| entry.retired_epoch >= self.epoch);
    }

    /// Marks an owner present in the current frame.
    ///
    /// Repeated marks in one or many continuously present frames return the
    /// same token. A missing or retired target starts a new incarnation.
    pub fn mark_present(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        let target = target.into();
        if let Some(entry) = self.active.get_mut(&target) {
            entry.present_this_frame = true;
            return LivenessToken::new(self.registry_scope, target, entry.incarnation);
        }

        let incarnation = self.allocate_incarnation();
        self.active.insert(
            target,
            ActiveEntry {
                incarnation,
                present_this_frame: true,
            },
        );
        LivenessToken::new(self.registry_scope, target, incarnation)
    }

    /// Starts a replacement incarnation and marks it present.
    pub fn restart(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        let target = target.into();
        let incarnation = self.allocate_incarnation();
        self.active.insert(
            target,
            ActiveEntry {
                incarnation,
                present_this_frame: true,
            },
        );
        LivenessToken::new(self.registry_scope, target, incarnation)
    }

    /// Cancels the exact active token incarnation.
    ///
    /// Repeating cancellation for the latest cancelled tombstone is
    /// idempotent and does not extend its retention epoch.
    pub fn cancel(&mut self, token: LivenessToken) -> LivenessUpdateStatus {
        let status = self.validate(token);
        if status != LivenessUpdateStatus::Applied {
            return status;
        }

        self.active.remove(&token.target);
        self.tombstones.insert(
            token.target,
            TombstoneEntry {
                incarnation: token.incarnation,
                reason: TombstoneReason::Cancelled,
                retired_epoch: self.epoch,
            },
        );
        LivenessUpdateStatus::Cancelled {
            target: token.target,
            incarnation: token.incarnation,
        }
    }

    /// Explicitly removes the target's active incarnation.
    pub fn remove(&mut self, target: impl Into<LivenessTargetId>) -> LivenessRemovalStatus {
        let target = target.into();
        let Some(entry) = self.active.remove(&target) else {
            return LivenessRemovalStatus::AlreadyAbsent;
        };

        self.tombstones.insert(
            target,
            TombstoneEntry {
                incarnation: entry.incarnation,
                reason: TombstoneReason::Removed,
                retired_epoch: self.epoch,
            },
        );
        LivenessRemovalStatus::Removed
    }

    /// Returns true when the target was explicitly marked in the current
    /// frame.
    #[must_use]
    pub fn is_present(&self, target: impl Into<LivenessTargetId>) -> bool {
        self.active
            .get(&target.into())
            .is_some_and(|entry| entry.present_this_frame)
    }

    /// Returns true while the target has an active incarnation, including
    /// between frame begin and omission finalization.
    #[must_use]
    pub fn is_active(&self, target: impl Into<LivenessTargetId>) -> bool {
        self.active.contains_key(&target.into())
    }

    /// Returns the target's active incarnation.
    #[must_use]
    pub fn current_incarnation(
        &self,
        target: impl Into<LivenessTargetId>,
    ) -> Option<LivenessIncarnation> {
        self.active
            .get(&target.into())
            .map(|entry| entry.incarnation)
    }

    /// Marks presence using the previous live terminology.
    #[deprecated(note = "use mark_present")]
    pub fn mark_live(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.mark_present(target)
    }

    /// Tests active-incarnation state using the previous live terminology.
    #[deprecated(note = "use is_active")]
    #[must_use]
    pub fn is_live(&self, target: impl Into<LivenessTargetId>) -> bool {
        self.is_active(target)
    }

    /// Returns the active incarnation using the previous generation
    /// terminology.
    #[allow(deprecated)]
    #[deprecated(note = "use current_incarnation")]
    #[must_use]
    pub fn current_generation(
        &self,
        target: impl Into<LivenessTargetId>,
    ) -> Option<LivenessGeneration> {
        self.current_incarnation(target)
    }

    /// Validates a token against the latest active or tombstoned target
    /// incarnation without running work.
    #[must_use]
    pub fn validate(&self, token: LivenessToken) -> LivenessUpdateStatus {
        if token.registry_scope != self.registry_scope {
            return LivenessUpdateStatus::StaleTarget {
                target: token.target,
            };
        }

        let active = self.active.get(&token.target);
        let tombstone = self.tombstones.get(&token.target);
        let current_incarnation = active
            .map(|entry| entry.incarnation)
            .or_else(|| tombstone.map(|entry| entry.incarnation));
        let Some(current_incarnation) = current_incarnation else {
            return LivenessUpdateStatus::StaleTarget {
                target: token.target,
            };
        };

        if token.incarnation != current_incarnation {
            return LivenessUpdateStatus::StaleIncarnation {
                target: token.target,
                token_incarnation: token.incarnation,
                current_incarnation,
            };
        }

        if active.is_some() {
            return LivenessUpdateStatus::Applied;
        }

        match tombstone.map(|entry| entry.reason) {
            Some(TombstoneReason::Cancelled) => LivenessUpdateStatus::Cancelled {
                target: token.target,
                incarnation: token.incarnation,
            },
            Some(TombstoneReason::Removed | TombstoneReason::Omitted) | None => {
                LivenessUpdateStatus::StaleTarget {
                    target: token.target,
                }
            }
        }
    }

    /// Runs `update` once for this call only when the token matches the active
    /// incarnation.
    pub fn apply_update(
        &self,
        token: LivenessToken,
        update: impl FnOnce(),
    ) -> LivenessUpdateStatus {
        let status = self.validate(token);
        if status == LivenessUpdateStatus::Applied {
            update();
        }
        status
    }

    fn allocate_incarnation(&mut self) -> LivenessIncarnation {
        self.last_incarnation =
            checked_successor(self.last_incarnation).expect("liveness incarnation overflow");
        LivenessIncarnation(self.last_incarnation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_counter_successor_never_wraps() {
        assert_eq!(checked_successor(0), Some(1));
        assert_eq!(checked_successor(u64::MAX), None);
    }

    #[test]
    #[should_panic(expected = "liveness incarnation overflow")]
    fn incarnation_overflow_panics_before_wrapping() {
        let mut registry = LivenessRegistry::new();
        registry.last_incarnation = u64::MAX;
        registry.restart(WidgetId::from_key("overflow"));
    }

    #[test]
    #[should_panic(expected = "liveness frame epoch overflow")]
    fn epoch_overflow_panics_before_wrapping() {
        let mut registry = LivenessRegistry::new();
        registry.epoch = u64::MAX;
        registry.begin_frame();
    }
}
