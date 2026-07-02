#[allow(clippy::wildcard_imports)]
use super::*;

/// Select operation requested by timeline interaction metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineSelectionOperation {
    /// Replace the current selection.
    Replace,
    /// Toggle the target in the current selection.
    Toggle,
    /// Extend the current selection.
    Extend,
}

/// Clip trim edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineTrimEdge {
    /// Trim the clip/item start.
    Start,
    /// Trim the clip/item end.
    End,
}

/// Data-only request metadata for a playhead seek.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelinePlayheadSeekRequest {
    /// Requested time before snap.
    pub requested_time: TimelineTime,
    /// Requested frame after snap.
    pub frame: TimelineFrame,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
}

impl TimelinePlayheadSeekRequest {
    /// Creates playhead seek metadata.
    #[must_use]
    pub fn new(
        requested_time: TimelineTime,
        frame_rate: TimelineFrameRate,
        snap: TimelineSnapMetadata,
    ) -> Self {
        Self {
            requested_time: requested_time.sanitized(),
            frame: frame_rate
                .sanitized()
                .time_to_frame(snap.snapped_time, TimelineFrameRounding::Nearest),
            snap,
        }
    }
}

/// Data-only metadata for beginning a timeline scrub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScrubBeginRequest {
    /// Hit target that initiated the scrub.
    pub source: TimelineHitTarget,
    /// Time before the scrub begins.
    pub previous_time: TimelineTime,
    /// Requested current time.
    pub current_time: TimelineTime,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

/// Data-only metadata for updating a timeline scrub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScrubUpdateRequest {
    /// Hit target that initiated the scrub.
    pub source: TimelineHitTarget,
    /// Time before this update.
    pub previous_time: TimelineTime,
    /// Requested current time.
    pub current_time: TimelineTime,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

/// Data-only metadata for ending a timeline scrub.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScrubEndRequest {
    /// Hit target that initiated the scrub.
    pub source: TimelineHitTarget,
    /// Time before the scrub began.
    pub start_time: TimelineTime,
    /// Time before the end request.
    pub previous_time: TimelineTime,
    /// Requested final time.
    pub current_time: TimelineTime,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
    /// Whether pointer capture should still be held after this request.
    pub pointer_capture_requested: bool,
}

impl TimelineScrubBeginRequest {
    /// Creates scrub-begin metadata.
    #[must_use]
    pub const fn new(
        source: TimelineHitTarget,
        previous_time: TimelineTime,
        current_time: TimelineTime,
        snap: TimelineSnapMetadata,
    ) -> Self {
        Self {
            source,
            previous_time,
            current_time,
            snap,
            pointer_capture_requested: true,
        }
    }
}

impl TimelineScrubUpdateRequest {
    /// Creates scrub-update metadata.
    #[must_use]
    pub const fn new(
        source: TimelineHitTarget,
        previous_time: TimelineTime,
        current_time: TimelineTime,
        snap: TimelineSnapMetadata,
    ) -> Self {
        Self {
            source,
            previous_time,
            current_time,
            snap,
            pointer_capture_requested: true,
        }
    }
}

impl TimelineScrubEndRequest {
    /// Creates scrub-end metadata.
    #[must_use]
    pub const fn new(
        source: TimelineHitTarget,
        start_time: TimelineTime,
        previous_time: TimelineTime,
        current_time: TimelineTime,
        snap: TimelineSnapMetadata,
    ) -> Self {
        Self {
            source,
            start_time,
            previous_time,
            current_time,
            snap,
            pointer_capture_requested: false,
        }
    }
}

/// Data-only metadata for beginning range selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRangeSelectionBeginRequest {
    /// Hit target that initiated range selection.
    pub source: TimelineHitTarget,
    /// Anchor time.
    pub anchor_time: TimelineTime,
    /// Current pointer time.
    pub current_time: TimelineTime,
    /// Normalized clamped range.
    pub range: TimelineRange,
    /// Snap metadata used by the current time.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

/// Data-only metadata for updating range selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRangeSelectionUpdateRequest {
    /// Hit target that initiated range selection.
    pub source: TimelineHitTarget,
    /// Anchor time.
    pub anchor_time: TimelineTime,
    /// Current pointer time.
    pub current_time: TimelineTime,
    /// Normalized clamped range.
    pub range: TimelineRange,
    /// Snap metadata used by the current time.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

/// Data-only metadata for ending range selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRangeSelectionEndRequest {
    /// Hit target that initiated range selection.
    pub source: TimelineHitTarget,
    /// Anchor time.
    pub anchor_time: TimelineTime,
    /// Final pointer time.
    pub current_time: TimelineTime,
    /// Normalized clamped range.
    pub range: TimelineRange,
    /// Snap metadata used by the current time.
    pub snap: TimelineSnapMetadata,
    /// Whether pointer capture should still be held after this request.
    pub pointer_capture_requested: bool,
}

impl TimelineRangeSelectionBeginRequest {
    /// Creates range-selection begin metadata.
    #[must_use]
    pub fn new(
        source: TimelineHitTarget,
        anchor_time: TimelineTime,
        current_time: TimelineTime,
        bounds: TimelineRange,
        snap: TimelineSnapMetadata,
    ) -> Self {
        let range = clamped_timeline_drag_range(anchor_time, snap.snapped_time, bounds);
        Self {
            source,
            anchor_time: clamp_timeline_time(anchor_time, bounds),
            current_time,
            range,
            snap,
            pointer_capture_requested: true,
        }
    }
}

impl TimelineRangeSelectionUpdateRequest {
    /// Creates range-selection update metadata.
    #[must_use]
    pub fn new(
        source: TimelineHitTarget,
        anchor_time: TimelineTime,
        current_time: TimelineTime,
        bounds: TimelineRange,
        snap: TimelineSnapMetadata,
    ) -> Self {
        let range = clamped_timeline_drag_range(anchor_time, snap.snapped_time, bounds);
        Self {
            source,
            anchor_time: clamp_timeline_time(anchor_time, bounds),
            current_time,
            range,
            snap,
            pointer_capture_requested: true,
        }
    }
}

impl TimelineRangeSelectionEndRequest {
    /// Creates range-selection end metadata.
    #[must_use]
    pub fn new(
        source: TimelineHitTarget,
        anchor_time: TimelineTime,
        current_time: TimelineTime,
        bounds: TimelineRange,
        snap: TimelineSnapMetadata,
    ) -> Self {
        let range = clamped_timeline_drag_range(anchor_time, snap.snapped_time, bounds);
        Self {
            source,
            anchor_time: clamp_timeline_time(anchor_time, bounds),
            current_time,
            range,
            snap,
            pointer_capture_requested: false,
        }
    }
}

/// Data-only marker selection metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineMarkerSelectionRequest {
    /// Target marker.
    pub target: TimelineMarkerId,
    /// Requested selection operation.
    pub operation: TimelineSelectionOperation,
}

/// Data-only marker context metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineMarkerContextRequest {
    /// Target marker.
    pub target: TimelineMarkerId,
    /// Marker time.
    pub time: TimelineTime,
    /// Descriptor state at request time.
    pub state: TimelineDescriptorState,
}

/// Data-only clip/item selection metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineClipSelectionRequest {
    /// Target clip/item.
    pub target: TimelineItemId,
    /// Requested selection operation.
    pub operation: TimelineSelectionOperation,
}

/// Data-only keyframe selection metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineKeyframeSelectionRequest {
    /// Target keyframe.
    pub target: TimelineKeyframeId,
    /// Containing clip/item.
    pub item: TimelineItemId,
    /// Requested selection operation.
    pub operation: TimelineSelectionOperation,
}

/// Data-only clip/item move metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineClipMoveRequest {
    /// Target clip/item.
    pub target: TimelineItemId,
    /// Containing lane.
    pub lane: TimelineLaneId,
    /// Source range before movement.
    pub original_range: TimelineRange,
    /// Requested movement delta in timeline seconds.
    pub requested_delta: TimelineTime,
    /// Movement delta after snap.
    pub snapped_delta: TimelineTime,
    /// Range requested before snap.
    pub requested_range: TimelineRange,
    /// Range requested after snap.
    pub snapped_range: TimelineRange,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

impl TimelineClipMoveRequest {
    /// Returns true when this request would not move the clip/item.
    #[must_use]
    pub fn is_noop(self) -> bool {
        self.snapped_delta.sanitized() == TimelineTime::ZERO
    }
}

/// Data-only clip/item trim metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineClipTrimRequest {
    /// Target clip/item.
    pub target: TimelineItemId,
    /// Trimmed edge.
    pub edge: TimelineTrimEdge,
    /// Source range before trim.
    pub original_range: TimelineRange,
    /// Requested trim time before snap or clamping.
    pub requested_time: TimelineTime,
    /// Requested trim time after snap and clamping.
    pub clamped_time: TimelineTime,
    /// Range requested after snap and clamping.
    pub clamped_range: TimelineRange,
    /// Snap metadata used by the request.
    pub snap: TimelineSnapMetadata,
    /// Whether the UI should preserve pointer-capture intent for this drag.
    pub pointer_capture_requested: bool,
}

impl TimelineClipTrimRequest {
    /// Returns true when this request would not change the clip/item range.
    #[must_use]
    pub fn is_noop(self) -> bool {
        self.clamped_range == self.original_range
    }
}
