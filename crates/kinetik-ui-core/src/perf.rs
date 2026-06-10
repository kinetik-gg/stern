//! Lightweight performance and budget tracking models.

use std::time::Duration;

/// Timings collected for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameTimings {
    /// Time spent converting input and preparing frame context.
    pub input: Duration,
    /// Time spent measuring and laying out UI.
    pub layout: Duration,
    /// Time spent running widget/update code.
    pub update: Duration,
    /// Time spent producing render primitives.
    pub render: Duration,
    /// Time spent submitting work to the renderer/platform.
    pub present: Duration,
}

impl FrameTimings {
    /// Returns the sum of all recorded stage timings.
    #[must_use]
    pub fn total(self) -> Duration {
        self.input + self.layout + self.update + self.render + self.present
    }
}

/// Counts collected for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameCounters {
    /// Emitted render primitive count.
    pub primitives: usize,
    /// Emitted semantic node count.
    pub semantic_nodes: usize,
    /// Routed or emitted action count.
    pub actions: usize,
    /// Uploaded or touched texture count.
    pub textures: usize,
}

/// Performance snapshot for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameMetrics {
    /// Sequential frame number.
    pub frame_index: u64,
    /// Stage timings.
    pub timings: FrameTimings,
    /// Frame counters.
    pub counters: FrameCounters,
}

impl FrameMetrics {
    /// Creates a metrics snapshot.
    #[must_use]
    pub const fn new(frame_index: u64, timings: FrameTimings, counters: FrameCounters) -> Self {
        Self {
            frame_index,
            timings,
            counters,
        }
    }

    /// Returns true when the total frame time is within the target duration.
    #[must_use]
    pub fn within_frame_time(self, target: Duration) -> bool {
        self.timings.total() <= target
    }
}

/// Per-frame allocation budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationBudget {
    /// Allowed allocation count.
    pub allocations: usize,
    /// Allowed allocated bytes.
    pub bytes: usize,
}

impl AllocationBudget {
    /// Creates an allocation budget.
    #[must_use]
    pub const fn new(allocations: usize, bytes: usize) -> Self {
        Self { allocations, bytes }
    }

    /// Evaluates allocation usage against the budget.
    #[must_use]
    pub fn evaluate(self, usage: AllocationUsage) -> BudgetStatus {
        BudgetStatus {
            allocation_budget: self.allocations,
            allocation_usage: usage.allocations,
            byte_budget: self.bytes,
            byte_usage: usage.bytes,
        }
    }
}

/// Per-frame allocation usage sample.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllocationUsage {
    /// Allocation count.
    pub allocations: usize,
    /// Allocated bytes.
    pub bytes: usize,
}

impl AllocationUsage {
    /// Creates an allocation usage sample.
    #[must_use]
    pub const fn new(allocations: usize, bytes: usize) -> Self {
        Self { allocations, bytes }
    }
}

/// Result of comparing allocation usage with a budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetStatus {
    /// Allowed allocation count.
    pub allocation_budget: usize,
    /// Actual allocation count.
    pub allocation_usage: usize,
    /// Allowed allocated bytes.
    pub byte_budget: usize,
    /// Actual allocated bytes.
    pub byte_usage: usize,
}

impl BudgetStatus {
    /// Returns true when all budget dimensions are within bounds.
    #[must_use]
    pub const fn within_budget(self) -> bool {
        self.allocation_usage <= self.allocation_budget && self.byte_usage <= self.byte_budget
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AllocationBudget, AllocationUsage, FrameCounters, FrameMetrics, FrameTimings};

    #[test]
    fn frame_metrics_sum_timings_and_check_target() {
        let timings = FrameTimings {
            input: Duration::from_millis(1),
            layout: Duration::from_millis(2),
            update: Duration::from_millis(3),
            render: Duration::from_millis(4),
            present: Duration::from_millis(5),
        };
        let metrics = FrameMetrics::new(7, timings, FrameCounters::default());

        assert_eq!(metrics.timings.total(), Duration::from_millis(15));
        assert!(metrics.within_frame_time(Duration::from_millis(16)));
        assert!(!metrics.within_frame_time(Duration::from_millis(14)));
    }

    #[test]
    fn allocation_budget_reports_status() {
        let budget = AllocationBudget::new(10, 1024);

        assert!(
            budget
                .evaluate(AllocationUsage::new(8, 512))
                .within_budget()
        );
        assert!(
            !budget
                .evaluate(AllocationUsage::new(11, 512))
                .within_budget()
        );
    }
}
