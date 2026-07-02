#[allow(clippy::wildcard_imports)]
use super::*;

/// Source that produced a snapped timeline time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineSnapSource {
    /// No snap was applied.
    None,
    /// Snapped to a frame boundary.
    Frame,
    /// Snapped to the playhead.
    Playhead,
    /// Snapped to a range boundary.
    RangeBoundary,
    /// Snapped to a clip/item boundary.
    ItemBoundary,
    /// Snapped to a marker.
    Marker,
    /// Snapped to a keyframe.
    Keyframe,
}

/// Data-only snap candidate supplied by the application or descriptor projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSnapCandidate {
    /// Candidate time.
    pub time: TimelineTime,
    /// Snap source kind.
    pub source: TimelineSnapSource,
    /// Stable target identity for the snap source.
    pub target: Option<TimelineHitTarget>,
}

impl TimelineSnapCandidate {
    /// Creates snap candidate metadata.
    #[must_use]
    pub const fn new(
        time: TimelineTime,
        source: TimelineSnapSource,
        target: Option<TimelineHitTarget>,
    ) -> Self {
        Self {
            time,
            source,
            target,
        }
    }
}

/// Data-only request for collecting deterministic snap candidates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSnapCandidateRequest<'a> {
    /// Timeline identity used for playhead and range boundary targets.
    pub timeline: TimelineId,
    /// Time range in which frame candidates should be generated.
    pub range: TimelineRange,
    /// Frame grid used for frame snap candidates.
    pub frame_rate: TimelineFrameRate,
    /// App-owned timeline descriptors used for marker, keyframe, and clip edge candidates.
    pub descriptor: &'a TimelineDescriptor,
    /// Optional playhead candidate.
    pub playhead_time: Option<TimelineTime>,
    /// Optional range boundaries.
    pub selection_range: Option<TimelineRange>,
    /// Maximum frame-grid candidates to emit.
    pub max_frame_candidates: usize,
}

impl<'a> TimelineSnapCandidateRequest<'a> {
    /// Creates a snap candidate request with deterministic defaults.
    #[must_use]
    pub const fn new(
        timeline: TimelineId,
        range: TimelineRange,
        frame_rate: TimelineFrameRate,
        descriptor: &'a TimelineDescriptor,
    ) -> Self {
        Self {
            timeline,
            range,
            frame_rate,
            descriptor,
            playhead_time: None,
            selection_range: None,
            max_frame_candidates: DEFAULT_TIMELINE_RULER_MAX_TICKS,
        }
    }

    /// Assigns playhead candidate metadata.
    #[must_use]
    pub const fn with_playhead_time(mut self, playhead_time: TimelineTime) -> Self {
        self.playhead_time = Some(playhead_time);
        self
    }

    /// Assigns range boundary candidate metadata.
    #[must_use]
    pub const fn with_selection_range(mut self, selection_range: TimelineRange) -> Self {
        self.selection_range = Some(selection_range);
        self
    }

    /// Assigns the maximum frame-grid candidates to emit.
    #[must_use]
    pub const fn with_max_frame_candidates(mut self, max_frame_candidates: usize) -> Self {
        self.max_frame_candidates = max_frame_candidates;
        self
    }

    /// Collects deterministic snap candidates for the request.
    #[must_use]
    pub fn candidates(self) -> Vec<TimelineSnapCandidate> {
        timeline_snap_candidates(self)
    }
}

/// Data-only snap result metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineSnapMetadata {
    /// Time originally requested by interaction.
    pub requested_time: TimelineTime,
    /// Time after snap evaluation.
    pub snapped_time: TimelineTime,
    /// Snap source that produced `snapped_time`.
    pub source: TimelineSnapSource,
    /// Stable target identity for the snap source.
    pub target: Option<TimelineHitTarget>,
}

impl TimelineSnapMetadata {
    /// Creates unsnapped metadata.
    #[must_use]
    pub const fn unsnapped(requested_time: TimelineTime) -> Self {
        Self {
            requested_time,
            snapped_time: requested_time,
            source: TimelineSnapSource::None,
            target: None,
        }
    }

    /// Creates snapped metadata.
    #[must_use]
    pub const fn snapped(
        requested_time: TimelineTime,
        snapped_time: TimelineTime,
        source: TimelineSnapSource,
        target: Option<TimelineHitTarget>,
    ) -> Self {
        Self {
            requested_time,
            snapped_time,
            source,
            target,
        }
    }

    /// Returns true when the requested and snapped times are identical.
    #[must_use]
    pub fn is_noop(self) -> bool {
        self.source == TimelineSnapSource::None
            || self.requested_time.sanitized() == self.snapped_time.sanitized()
    }
}
