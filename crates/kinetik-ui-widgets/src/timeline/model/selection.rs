#[allow(clippy::wildcard_imports)]
use super::*;

/// Stable timeline selection target independent from descriptor order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimelineSelectionTarget {
    /// Lane selection target.
    Lane(TimelineLaneId),
    /// Clip/item selection target.
    Item(TimelineItemId),
    /// Marker selection target.
    Marker(TimelineMarkerId),
    /// Keyframe selection target.
    Keyframe(TimelineKeyframeId),
}

/// Data-only timeline selection set keyed by stable target IDs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimelineSelection {
    targets: BTreeSet<TimelineSelectionTarget>,
}

impl TimelineSelection {
    /// Creates an empty selection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            targets: BTreeSet::new(),
        }
    }

    /// Creates a selection from stable targets.
    #[must_use]
    pub fn from_targets(targets: impl IntoIterator<Item = TimelineSelectionTarget>) -> Self {
        Self {
            targets: targets.into_iter().collect(),
        }
    }

    /// Returns selected targets in deterministic target-ID order.
    #[must_use]
    pub fn targets(&self) -> Vec<TimelineSelectionTarget> {
        self.targets.iter().copied().collect()
    }

    /// Returns true when the target is selected.
    #[must_use]
    pub fn contains(&self, target: TimelineSelectionTarget) -> bool {
        self.targets.contains(&target)
    }

    /// Returns true when no targets are selected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Applies a selection operation without relying on descriptor order.
    pub fn apply(
        &mut self,
        target: TimelineSelectionTarget,
        operation: TimelineSelectionOperation,
    ) {
        match operation {
            TimelineSelectionOperation::Replace => {
                self.targets.clear();
                self.targets.insert(target);
            }
            TimelineSelectionOperation::Toggle => {
                if !self.targets.remove(&target) {
                    self.targets.insert(target);
                }
            }
            TimelineSelectionOperation::Extend => {
                self.targets.insert(target);
            }
        }
    }

    /// Returns selected targets in the current descriptor presentation order.
    #[must_use]
    pub fn targets_in_descriptor_order(
        &self,
        descriptor: &TimelineDescriptor,
    ) -> Vec<TimelineSelectionTarget> {
        let mut ordered = Vec::new();
        ordered.extend(
            descriptor
                .lanes
                .iter()
                .map(|lane| TimelineSelectionTarget::Lane(lane.id))
                .filter(|target| self.contains(*target)),
        );
        ordered.extend(
            descriptor
                .items
                .iter()
                .map(|item| TimelineSelectionTarget::Item(item.id))
                .filter(|target| self.contains(*target)),
        );
        ordered.extend(
            descriptor
                .markers
                .iter()
                .map(|marker| TimelineSelectionTarget::Marker(marker.id))
                .filter(|target| self.contains(*target)),
        );
        ordered.extend(
            descriptor
                .keyframes
                .iter()
                .map(|keyframe| TimelineSelectionTarget::Keyframe(keyframe.id))
                .filter(|target| self.contains(*target)),
        );
        ordered
    }

    /// Returns a copy with targets that still exist in the supplied descriptor.
    #[must_use]
    pub fn retain_existing_targets(&self, descriptor: &TimelineDescriptor) -> Self {
        Self::from_targets(
            self.targets_in_descriptor_order(descriptor)
                .into_iter()
                .filter(|target| self.contains(*target)),
        )
    }
}

/// Data-only timeline viewport state used by apps to preserve interaction metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineViewportState {
    /// Horizontal timeline scale and scroll.
    pub scale: TimelineScale,
    /// Vertical lane scroll offset.
    pub lane_scroll_offset: f32,
    /// Current playhead time, when known.
    pub playhead_time: Option<TimelineTime>,
    /// Stable selection targets.
    pub selection: TimelineSelection,
    /// Current selected time range, when any.
    pub selection_range: Option<TimelineRange>,
    /// Last snap metadata associated with an interaction.
    pub snap: Option<TimelineSnapMetadata>,
}

impl TimelineViewportState {
    /// Creates viewport state from a timeline scale.
    #[must_use]
    pub fn new(scale: TimelineScale) -> Self {
        Self {
            scale: scale.sanitized(),
            lane_scroll_offset: 0.0,
            playhead_time: None,
            selection: TimelineSelection::new(),
            selection_range: None,
            snap: None,
        }
    }

    /// Assigns playhead time metadata.
    #[must_use]
    pub fn with_playhead_time(mut self, playhead_time: TimelineTime) -> Self {
        self.playhead_time = Some(playhead_time.sanitized());
        self
    }

    /// Assigns stable selection metadata.
    #[must_use]
    pub fn with_selection(mut self, selection: TimelineSelection) -> Self {
        self.selection = selection;
        self
    }

    /// Assigns range selection metadata.
    #[must_use]
    pub fn with_selection_range(mut self, selection_range: TimelineRange) -> Self {
        self.selection_range = Some(selection_range.sanitized());
        self
    }

    /// Assigns snap metadata.
    #[must_use]
    pub fn with_snap(mut self, snap: TimelineSnapMetadata) -> Self {
        self.snap = Some(sanitize_timeline_snap_metadata(snap));
        self
    }

    /// Returns a copy with clamped horizontal scroll while preserving interaction metadata.
    #[must_use]
    pub fn with_horizontal_scroll_offset(mut self, scroll_offset: f32) -> Self {
        self.scale.scroll_offset = scroll_offset;
        self.scale = self.scale.sanitized();
        self.sanitize_metadata()
    }

    /// Returns a copy with clamped lane scroll while preserving interaction metadata.
    #[must_use]
    pub fn with_lane_scroll_offset(mut self, scroll_offset: f32, max_scroll_offset: f32) -> Self {
        self.lane_scroll_offset = clamp_timeline_scroll_offset(scroll_offset, max_scroll_offset);
        self.sanitize_metadata()
    }

    /// Returns a copy zoomed around `anchor_x` while preserving interaction metadata.
    #[must_use]
    pub fn with_zoom_around_anchor(
        mut self,
        anchor_x: f32,
        zoom: TimelineZoom,
    ) -> TimelineViewportZoomResult {
        let result = self.scale.zoom_around_anchor(anchor_x, zoom);
        self.scale = result.scale;
        self = self.sanitize_metadata();
        TimelineViewportZoomResult {
            state: self,
            anchor_time: result.anchor_time,
            anchor_x: result.anchor_x,
        }
    }

    fn sanitize_metadata(mut self) -> Self {
        self.playhead_time = self.playhead_time.map(TimelineTime::sanitized);
        self.selection_range = self.selection_range.map(TimelineRange::sanitized);
        self.snap = self.snap.map(sanitize_timeline_snap_metadata);
        self
    }
}

/// Result of zooming a viewport state around an anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineViewportZoomResult {
    /// Viewport state after zoom and clamping.
    pub state: TimelineViewportState,
    /// Timeline time preserved under the anchor before zooming.
    pub anchor_time: TimelineTime,
    /// Sanitized screen-space anchor.
    pub anchor_x: f32,
}
