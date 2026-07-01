#[allow(clippy::wildcard_imports)]
use super::*;

/// Timeline lane layout contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineLayout {
    /// Fixed lane row height.
    pub row_height: f32,
    /// Extra lane rows to materialize before and after the visible range.
    pub overscan: usize,
    /// Hit target width for timeline markers.
    pub marker_hit_width: f32,
    /// Hit target size for keyframes.
    pub keyframe_hit_size: f32,
}

impl TimelineLayout {
    /// Creates a timeline layout contract.
    #[must_use]
    pub const fn new(row_height: f32) -> Self {
        Self {
            row_height,
            overscan: 0,
            marker_hit_width: 9.0,
            keyframe_hit_size: 9.0,
        }
    }

    /// Assigns overscan rows for materialization.
    #[must_use]
    pub const fn with_overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Assigns marker hit target width.
    #[must_use]
    pub const fn with_marker_hit_width(mut self, marker_hit_width: f32) -> Self {
        self.marker_hit_width = marker_hit_width;
        self
    }

    /// Assigns keyframe hit target size.
    #[must_use]
    pub const fn with_keyframe_hit_size(mut self, keyframe_hit_size: f32) -> Self {
        self.keyframe_hit_size = keyframe_hit_size;
        self
    }

    /// Resolves deterministic lane rows, clip/item rectangles, markers, and keyframes.
    ///
    /// # Errors
    ///
    /// Returns [`TimelineDescriptorError`] when the supplied descriptor set does
    /// not validate.
    pub fn resolve(
        self,
        bounds: Rect,
        scale: TimelineScale,
        descriptor: &TimelineDescriptor,
        scroll_offset: f32,
    ) -> Result<TimelineLayoutResult<'_>, TimelineDescriptorError> {
        resolve_timeline_layout(self, bounds, scale, descriptor, scroll_offset)
    }
}

/// Resolved lane row metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTimelineLane<'a> {
    /// Source lane descriptor.
    pub descriptor: &'a TimelineLaneDescriptor,
    /// Descriptor index in app-owned lane order.
    pub source_index: usize,
    /// Row index in the full timeline.
    pub row_index: usize,
    /// Materialized row rectangle.
    pub rect: Rect,
}

/// Resolved timeline clip/item rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTimelineItem<'a> {
    /// Source item descriptor.
    pub descriptor: &'a TimelineItemDescriptor,
    /// Descriptor index in app-owned item order.
    pub source_index: usize,
    /// Lane row index.
    pub lane_index: usize,
    /// Source time range with finite ascending endpoints.
    pub time_range: TimelineRange,
    /// Time range represented by the clamped rectangle.
    pub visible_time_range: TimelineRange,
    /// Rectangle clamped to the visible timeline bounds.
    pub rect: Rect,
    /// Unclamped rectangle produced directly from source times and the timeline scale.
    pub unclipped_rect: Rect,
}

/// Resolved timeline marker hit target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTimelineMarker<'a> {
    /// Source marker descriptor.
    pub descriptor: &'a TimelineMarkerDescriptor,
    /// Descriptor index in app-owned marker order.
    pub source_index: usize,
    /// Sanitized marker time.
    pub time: TimelineTime,
    /// Marker center x coordinate.
    pub x: f32,
    /// Hit rectangle clamped to the visible timeline bounds.
    pub hit_rect: Rect,
}

/// Resolved timeline keyframe hit target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTimelineKeyframe<'a> {
    /// Source keyframe descriptor.
    pub descriptor: &'a TimelineKeyframeDescriptor,
    /// Descriptor index in app-owned keyframe order.
    pub source_index: usize,
    /// Containing item.
    pub item: TimelineItemId,
    /// Containing lane row index.
    pub lane_index: usize,
    /// Sanitized keyframe time.
    pub time: TimelineTime,
    /// Keyframe center x coordinate.
    pub x: f32,
    /// Hit rectangle clamped to the visible timeline bounds.
    pub hit_rect: Rect,
}

/// Resolved timeline layout output.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineLayoutResult<'a> {
    /// Sanitized viewport bounds.
    pub bounds: Rect,
    /// Total lane content height.
    pub content_height: f32,
    /// Maximum valid vertical scroll offset.
    pub max_scroll_offset: f32,
    /// Sanitized and clamped vertical scroll offset.
    pub scroll_offset: f32,
    /// Strict visible lane range before overscan.
    pub visible_lane_range: Range<usize>,
    /// Overscanned lane range to materialize.
    pub materialized_lane_range: Range<usize>,
    /// Materialized lanes in descriptor order.
    pub lanes: Vec<ResolvedTimelineLane<'a>>,
    /// Resolved clip/items in deterministic hit/paint order.
    pub items: Vec<ResolvedTimelineItem<'a>>,
    /// Resolved markers in deterministic hit/paint order.
    pub markers: Vec<ResolvedTimelineMarker<'a>>,
    /// Resolved keyframes in deterministic hit/paint order.
    pub keyframes: Vec<ResolvedTimelineKeyframe<'a>>,
}

impl TimelineLayoutResult<'_> {
    /// Returns materialized lane IDs in app descriptor order.
    #[must_use]
    pub fn materialized_lane_ids(&self) -> Vec<TimelineLaneId> {
        self.lanes.iter().map(|lane| lane.descriptor.id).collect()
    }

    /// Resolves one UI logical point to deterministic timeline hit metadata.
    #[must_use]
    pub fn hit_test(
        &self,
        point: Point,
        config: TimelineHitTestConfig,
    ) -> Option<TimelineHitMetadata> {
        hit_test_timeline(self, point, config)
    }

    /// Creates a playhead seek request from a pointer x coordinate.
    #[must_use]
    pub fn playhead_seek_request(
        &self,
        pointer_x: f32,
        frame_rate: TimelineFrameRate,
        config: TimelineHitTestConfig,
        snap: TimelineSnapMetadata,
    ) -> TimelinePlayheadSeekRequest {
        let requested_time = config.scale.screen_x_to_time(pointer_x);
        TimelinePlayheadSeekRequest::new(requested_time, frame_rate, snap)
    }
}
