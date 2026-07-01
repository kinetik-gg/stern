#[allow(clippy::wildcard_imports)]
use super::*;

/// Stable backend-independent timeline hit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimelineHitTarget {
    /// Timeline background inside the content surface.
    Background(TimelineId),
    /// Timeline ruler surface.
    Ruler(TimelineRulerId),
    /// Playhead handle or line.
    Playhead(TimelineId),
    /// Start range-selection handle.
    RangeStartHandle(TimelineId),
    /// End range-selection handle.
    RangeEndHandle(TimelineId),
    /// Lane header row.
    LaneHeader(TimelineLaneId),
    /// Clip/item body.
    Item(TimelineItemId),
    /// Clip/item start trim handle.
    ItemTrimStartHandle(TimelineItemId),
    /// Clip/item end trim handle.
    ItemTrimEndHandle(TimelineItemId),
    /// Timeline marker.
    Marker(TimelineMarkerId),
    /// Timeline keyframe.
    Keyframe(TimelineKeyframeId),
}

/// Deterministic metadata for one timeline hit test result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineHitMetadata {
    /// Stable target identity.
    pub target: TimelineHitTarget,
    /// Hit rectangle that accepted the point.
    pub rect: Rect,
    /// Timeline time represented by the hit point's x coordinate.
    pub time: TimelineTime,
    /// Descriptor state for descriptor-backed targets.
    pub state: TimelineDescriptorState,
}

impl TimelineHitMetadata {
    /// Returns true when the hit target cannot emit interaction requests.
    #[must_use]
    pub const fn disabled(self) -> bool {
        self.state.disabled
    }

    /// Returns true when the hit target is visible but not editable.
    #[must_use]
    pub const fn read_only(self) -> bool {
        self.state.read_only
    }
}

/// Timeline hit-test geometry and identity context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineHitTestConfig {
    /// Timeline surface identity used by background, playhead, and range handles.
    pub timeline: TimelineId,
    /// Ruler identity.
    pub ruler: TimelineRulerId,
    /// Timeline scale used for pointer x to time conversion.
    pub scale: TimelineScale,
    /// Optional ruler rectangle.
    pub ruler_rect: Option<Rect>,
    /// Optional playhead time.
    pub playhead_time: Option<TimelineTime>,
    /// Optional selected time range with editable handles.
    pub selection_range: Option<TimelineRange>,
    /// Width of the lane-header hit strip from the left edge of timeline bounds.
    pub lane_header_width: f32,
    /// Logical hit width for the playhead line.
    pub playhead_hit_width: f32,
    /// Logical hit width for range handles.
    pub range_handle_hit_width: f32,
    /// Logical width reserved for clip trim handles.
    pub item_trim_handle_width: f32,
}

impl TimelineHitTestConfig {
    /// Creates timeline hit-test configuration with deterministic defaults.
    #[must_use]
    pub const fn new(timeline: TimelineId, ruler: TimelineRulerId, scale: TimelineScale) -> Self {
        Self {
            timeline,
            ruler,
            scale,
            ruler_rect: None,
            playhead_time: None,
            selection_range: None,
            lane_header_width: 0.0,
            playhead_hit_width: 7.0,
            range_handle_hit_width: 7.0,
            item_trim_handle_width: 6.0,
        }
    }

    /// Assigns the ruler hit rectangle.
    #[must_use]
    pub const fn with_ruler_rect(mut self, ruler_rect: Rect) -> Self {
        self.ruler_rect = Some(ruler_rect);
        self
    }

    /// Assigns the playhead time used for hit testing.
    #[must_use]
    pub const fn with_playhead_time(mut self, playhead_time: TimelineTime) -> Self {
        self.playhead_time = Some(playhead_time);
        self
    }

    /// Assigns the selected range used for range-handle hit testing.
    #[must_use]
    pub const fn with_selection_range(mut self, selection_range: TimelineRange) -> Self {
        self.selection_range = Some(selection_range);
        self
    }

    /// Assigns the lane header strip width.
    #[must_use]
    pub const fn with_lane_header_width(mut self, lane_header_width: f32) -> Self {
        self.lane_header_width = lane_header_width;
        self
    }

    /// Assigns playhead hit width.
    #[must_use]
    pub const fn with_playhead_hit_width(mut self, playhead_hit_width: f32) -> Self {
        self.playhead_hit_width = playhead_hit_width;
        self
    }

    /// Assigns range handle hit width.
    #[must_use]
    pub const fn with_range_handle_hit_width(mut self, range_handle_hit_width: f32) -> Self {
        self.range_handle_hit_width = range_handle_hit_width;
        self
    }

    /// Assigns clip trim handle hit width.
    #[must_use]
    pub const fn with_item_trim_handle_width(mut self, item_trim_handle_width: f32) -> Self {
        self.item_trim_handle_width = item_trim_handle_width;
        self
    }

    pub(crate) fn sanitized(self) -> Self {
        Self {
            timeline: self.timeline,
            ruler: self.ruler,
            scale: self.scale.sanitized(),
            ruler_rect: self.ruler_rect.map(finite_rect),
            playhead_time: self.playhead_time.map(TimelineTime::sanitized),
            selection_range: self.selection_range.map(TimelineRange::sanitized),
            lane_header_width: finite_f32_non_negative(self.lane_header_width),
            playhead_hit_width: finite_f32_non_negative(self.playhead_hit_width),
            range_handle_hit_width: finite_f32_non_negative(self.range_handle_hit_width),
            item_trim_handle_width: finite_f32_non_negative(self.item_trim_handle_width),
        }
    }
}

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

impl ResolvedTimelineItem<'_> {
    /// Creates clip/item selection metadata when the descriptor is enabled.
    #[must_use]
    pub const fn selection_request(
        &self,
        operation: TimelineSelectionOperation,
    ) -> Option<TimelineClipSelectionRequest> {
        if self.descriptor.state.disabled {
            None
        } else {
            Some(TimelineClipSelectionRequest {
                target: self.descriptor.id,
                operation,
            })
        }
    }

    /// Creates clip/item move metadata without mutating the descriptor.
    #[must_use]
    pub fn move_request(
        &self,
        requested_delta: TimelineTime,
        snap: TimelineSnapMetadata,
    ) -> Option<TimelineClipMoveRequest> {
        if self.descriptor.state.disabled || self.descriptor.state.read_only {
            return None;
        }

        let original_range = self.time_range.sanitized();
        let requested_delta = requested_delta.sanitized();
        let requested_range = offset_timeline_range(original_range, requested_delta);
        let snapped_delta = TimelineTime::from_seconds(
            snap.snapped_time.sanitized().seconds() - original_range.start.seconds(),
        )
        .sanitized();
        let snapped_range = offset_timeline_range(original_range, snapped_delta);

        Some(TimelineClipMoveRequest {
            target: self.descriptor.id,
            lane: self.descriptor.lane,
            original_range,
            requested_delta,
            snapped_delta,
            requested_range,
            snapped_range,
            snap,
            pointer_capture_requested: true,
        })
    }

    /// Creates clip/item trim metadata without mutating the descriptor.
    #[must_use]
    pub fn trim_request(
        &self,
        edge: TimelineTrimEdge,
        requested_time: TimelineTime,
        bounds: TimelineRange,
        snap: TimelineSnapMetadata,
    ) -> Option<TimelineClipTrimRequest> {
        if self.descriptor.state.disabled || self.descriptor.state.read_only {
            return None;
        }

        let original_range = self.time_range.sanitized();
        let bounds = bounds.sanitized();
        let snapped_time = clamp_timeline_time(snap.snapped_time, bounds);
        let clamped_time = match edge {
            TimelineTrimEdge::Start => TimelineTime::from_seconds(
                snapped_time
                    .seconds()
                    .clamp(bounds.start.seconds(), original_range.end.seconds()),
            ),
            TimelineTrimEdge::End => TimelineTime::from_seconds(
                snapped_time
                    .seconds()
                    .clamp(original_range.start.seconds(), bounds.end.seconds()),
            ),
        }
        .sanitized();
        let clamped_range = match edge {
            TimelineTrimEdge::Start => TimelineRange::new(clamped_time, original_range.end),
            TimelineTrimEdge::End => TimelineRange::new(original_range.start, clamped_time),
        }
        .sanitized();

        Some(TimelineClipTrimRequest {
            target: self.descriptor.id,
            edge,
            original_range,
            requested_time: requested_time.sanitized(),
            clamped_time,
            clamped_range,
            snap,
            pointer_capture_requested: true,
        })
    }
}

impl ResolvedTimelineMarker<'_> {
    /// Creates marker selection metadata when the descriptor is enabled.
    #[must_use]
    pub const fn selection_request(
        &self,
        operation: TimelineSelectionOperation,
    ) -> Option<TimelineMarkerSelectionRequest> {
        if self.descriptor.state.disabled {
            None
        } else {
            Some(TimelineMarkerSelectionRequest {
                target: self.descriptor.id,
                operation,
            })
        }
    }

    /// Creates marker context metadata when the descriptor is enabled.
    #[must_use]
    pub const fn context_request(&self) -> Option<TimelineMarkerContextRequest> {
        if self.descriptor.state.disabled {
            None
        } else {
            Some(TimelineMarkerContextRequest {
                target: self.descriptor.id,
                time: self.time,
                state: self.descriptor.state,
            })
        }
    }
}

impl ResolvedTimelineKeyframe<'_> {
    /// Creates keyframe selection metadata when the descriptor is enabled.
    #[must_use]
    pub const fn selection_request(
        &self,
        operation: TimelineSelectionOperation,
    ) -> Option<TimelineKeyframeSelectionRequest> {
        if self.descriptor.state.disabled {
            None
        } else {
            Some(TimelineKeyframeSelectionRequest {
                target: self.descriptor.id,
                item: self.item,
                operation,
            })
        }
    }
}
