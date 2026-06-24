//! Deterministic liveness tokens for retained UI targets.

use std::collections::HashMap;

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

/// Monotonic generation assigned whenever a liveness target is renewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LivenessGeneration(u64);

impl LivenessGeneration {
    /// First generation assigned to a newly seen target.
    pub const FIRST: Self = Self(1);

    /// Returns the numeric generation value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0.checked_add(1).expect("liveness generation overflow"))
    }
}

/// Opaque proof that an external update was issued for a target generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LivenessToken {
    target: LivenessTargetId,
    generation: LivenessGeneration,
}

impl LivenessToken {
    /// Creates a liveness token from explicit parts.
    #[must_use]
    pub const fn new(target: LivenessTargetId, generation: LivenessGeneration) -> Self {
        Self { target, generation }
    }

    /// Returns the target identity carried by this token.
    #[must_use]
    pub const fn target(self) -> LivenessTargetId {
        self.target
    }

    /// Returns the generation carried by this token.
    #[must_use]
    pub const fn generation(self) -> LivenessGeneration {
        self.generation
    }
}

/// Result of validating a liveness-gated update attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessUpdateStatus {
    /// The token matched the currently live target generation.
    Applied,
    /// The token target is not live in the retained registry.
    StaleTarget {
        /// Target carried by the stale token.
        target: LivenessTargetId,
    },
    /// The target is live, but the token carries an older generation.
    StaleGeneration {
        /// Target carried by the stale token.
        target: LivenessTargetId,
        /// Generation carried by the stale token.
        token_generation: LivenessGeneration,
        /// Current retained generation for the target.
        current_generation: LivenessGeneration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LivenessEntry {
    generation: LivenessGeneration,
    seen_this_frame: bool,
}

/// Retained target registry owned by [`crate::UiMemory`].
///
/// The registry keeps tombstoned generation entries for targets that disappeared
/// so a later same-ID re-entry cannot make old tokens valid again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LivenessRegistry {
    targets: HashMap<LivenessTargetId, LivenessEntry>,
}

impl LivenessRegistry {
    /// Creates an empty liveness registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when no target generations have been retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Returns the number of retained target generation records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Marks all retained targets unseen before a new frame is built.
    pub(crate) fn begin_frame(&mut self) {
        for entry in self.targets.values_mut() {
            entry.seen_this_frame = false;
        }
    }

    /// Preserves unseen target generations as tombstones at frame end.
    #[allow(clippy::unused_self)]
    pub(crate) fn end_frame(&mut self) {
        // Absence must validate as `StaleTarget`, but the generation record
        // must survive so same-ID re-entry renews past previously issued tokens.
    }

    /// Marks a target live and returns a token for its renewed generation.
    ///
    /// Re-marking the same target renews its generation. Older tokens for that
    /// target then validate as [`LivenessUpdateStatus::StaleGeneration`].
    pub fn mark_live(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        let target = target.into();
        let generation = if let Some(entry) = self.targets.get_mut(&target) {
            entry.generation = entry.generation.next();
            entry.seen_this_frame = true;
            entry.generation
        } else {
            self.targets.insert(
                target,
                LivenessEntry {
                    generation: LivenessGeneration::FIRST,
                    seen_this_frame: true,
                },
            );
            LivenessGeneration::FIRST
        };

        LivenessToken::new(target, generation)
    }

    /// Marks a target absent while preserving its generation history.
    pub fn remove(&mut self, target: impl Into<LivenessTargetId>) -> bool {
        let target = target.into();
        let Some(entry) = self.targets.get_mut(&target) else {
            return false;
        };

        entry.seen_this_frame = false;
        true
    }

    /// Returns the current generation for a live target.
    #[must_use]
    pub fn current_generation(
        &self,
        target: impl Into<LivenessTargetId>,
    ) -> Option<LivenessGeneration> {
        let target = target.into();
        self.targets
            .get(&target)
            .filter(|entry| entry.seen_this_frame)
            .map(|entry| entry.generation)
    }

    /// Returns true when a target is currently live.
    #[must_use]
    pub fn is_live(&self, target: impl Into<LivenessTargetId>) -> bool {
        self.current_generation(target).is_some()
    }

    /// Validates a token against the retained registry without running work.
    #[must_use]
    pub fn validate(&self, token: LivenessToken) -> LivenessUpdateStatus {
        let Some(entry) = self
            .targets
            .get(&token.target)
            .filter(|entry| entry.seen_this_frame)
        else {
            return LivenessUpdateStatus::StaleTarget {
                target: token.target,
            };
        };

        if entry.generation == token.generation {
            LivenessUpdateStatus::Applied
        } else {
            LivenessUpdateStatus::StaleGeneration {
                target: token.target,
                token_generation: token.generation,
                current_generation: entry.generation,
            }
        }
    }

    /// Runs `update` only when the token still matches the live target.
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
}
