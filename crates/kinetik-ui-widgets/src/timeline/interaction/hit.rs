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
