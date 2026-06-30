//! Data-only timeline ruler, frame-rate, lane, item, and coordinate contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use kinetik_ui_core::{
    Rect, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, WidgetId,
};

/// Default timeline ruler scale in logical pixels per second.
pub const DEFAULT_TIMELINE_PIXELS_PER_SECOND: f32 = 100.0;
/// Minimum timeline zoom in logical pixels per second.
pub const MIN_TIMELINE_PIXELS_PER_SECOND: f32 = 0.001;
/// Maximum timeline zoom in logical pixels per second.
pub const MAX_TIMELINE_PIXELS_PER_SECOND: f32 = 1_000_000.0;
/// Maximum number of ruler ticks emitted by the convenience tick generator.
pub const DEFAULT_TIMELINE_RULER_MAX_TICKS: usize = 4096;

macro_rules! timeline_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// Creates an ID from raw bits.
            #[must_use]
            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            /// Returns raw ID bits.
            #[must_use]
            pub const fn raw(self) -> u64 {
                self.0
            }
        }
    };
}

timeline_id!(TimelineId, "Stable identity for a timeline surface.");
timeline_id!(
    TimelineRulerId,
    "Stable identity for a timeline ruler surface."
);
timeline_id!(
    TransportControlId,
    "Stable identity for a timeline transport control."
);
timeline_id!(TimelineLaneId, "Stable identity for a timeline lane.");
timeline_id!(
    TimelineItemId,
    "Stable identity for a timeline clip or item."
);
timeline_id!(TimelineMarkerId, "Stable identity for a timeline marker.");
timeline_id!(
    TimelineKeyframeId,
    "Stable identity for a timeline keyframe target."
);

/// Compatibility name for timeline lanes used as tracks.
pub type TimelineTrackId = TimelineLaneId;
/// Compatibility name for timeline items used as clips.
pub type TimelineClipId = TimelineItemId;

/// Timeline time in seconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct TimelineTime {
    seconds: f64,
}

impl TimelineTime {
    /// The timeline origin.
    pub const ZERO: Self = Self::from_seconds(0.0);

    /// Creates timeline time from seconds.
    #[must_use]
    pub const fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }

    /// Returns raw seconds.
    #[must_use]
    pub const fn seconds(self) -> f64 {
        self.seconds
    }

    /// Returns a copy with non-finite seconds replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::from_seconds(finite_f64_or_zero(self.seconds))
    }
}

/// Integer frame position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimelineFrame(i64);

impl TimelineFrame {
    /// Creates a frame position from raw frame bits.
    #[must_use]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Returns the raw frame index.
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

/// Frame rounding policy for converting continuous time to integer frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineFrameRounding {
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round to the nearest frame, with half values away from zero.
    Nearest,
    /// Round toward zero.
    Truncate,
}

/// Rational frame-rate metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineFrameRate {
    /// Frames-per-second numerator.
    pub numerator: u32,
    /// Frames-per-second denominator.
    pub denominator: u32,
}

impl TimelineFrameRate {
    /// Creates rational frame-rate metadata.
    #[must_use]
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Creates an integer frame rate.
    #[must_use]
    pub const fn integer(frames_per_second: u32) -> Self {
        Self::new(frames_per_second, 1)
    }

    /// Returns deterministic non-zero frame-rate metadata.
    #[must_use]
    pub const fn sanitized(self) -> Self {
        Self {
            numerator: if self.numerator == 0 {
                24
            } else {
                self.numerator
            },
            denominator: if self.denominator == 0 {
                1
            } else {
                self.denominator
            },
        }
    }

    /// Returns frames per second as a finite number.
    #[must_use]
    pub fn frames_per_second(self) -> f64 {
        let rate = self.sanitized();
        f64::from(rate.numerator) / f64::from(rate.denominator)
    }

    /// Returns seconds per frame.
    #[must_use]
    pub fn seconds_per_frame(self) -> f64 {
        1.0 / self.frames_per_second()
    }

    /// Converts a frame position to timeline time.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn frame_to_time(self, frame: TimelineFrame) -> TimelineTime {
        TimelineTime::from_seconds(frame.raw() as f64 * self.seconds_per_frame()).sanitized()
    }

    /// Converts timeline time to an integer frame with the requested rounding policy.
    #[must_use]
    pub fn time_to_frame(
        self,
        time: TimelineTime,
        rounding: TimelineFrameRounding,
    ) -> TimelineFrame {
        let frame = time.sanitized().seconds() * self.frames_per_second();
        TimelineFrame::from_raw(round_frame(frame, rounding))
    }

    fn rounded_display_fps(self) -> i64 {
        let fps = self.frames_per_second().round();
        f64_to_i64_saturating(fps).max(1)
    }
}

impl Default for TimelineFrameRate {
    fn default() -> Self {
        Self::integer(24)
    }
}

/// Finite normalized timeline time range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRange {
    /// Start time.
    pub start: TimelineTime,
    /// End time.
    pub end: TimelineTime,
}

impl TimelineRange {
    /// Creates a timeline range.
    #[must_use]
    pub const fn new(start: TimelineTime, end: TimelineTime) -> Self {
        Self { start, end }
    }

    /// Creates a timeline range from seconds.
    #[must_use]
    pub const fn seconds(start: f64, end: f64) -> Self {
        Self::new(
            TimelineTime::from_seconds(start),
            TimelineTime::from_seconds(end),
        )
    }

    /// Returns a finite range with ascending endpoints.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let start = self.start.sanitized().seconds();
        let end = self.end.sanitized().seconds();
        Self::seconds(start.min(end), start.max(end))
    }

    /// Returns range duration in seconds.
    #[must_use]
    pub fn duration_seconds(self) -> f64 {
        let range = self.sanitized();
        (range.end.seconds() - range.start.seconds()).max(0.0)
    }

    /// Returns true when the range has no positive duration.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.duration_seconds() <= 0.0
    }

    /// Computes content width in logical pixels at the supplied zoom.
    #[must_use]
    pub fn content_width(self, zoom: TimelineZoom) -> f32 {
        finite_f64_to_f32(self.duration_seconds() * f64::from(zoom.sanitized().pixels_per_second))
    }
}

/// Timeline zoom in logical pixels per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineZoom {
    /// Logical pixels per timeline second.
    pub pixels_per_second: f32,
}

impl TimelineZoom {
    /// Creates timeline zoom metadata.
    #[must_use]
    pub const fn new(pixels_per_second: f32) -> Self {
        Self { pixels_per_second }
    }

    /// Returns a deterministic clamped zoom.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            pixels_per_second: sanitize_timeline_zoom(self.pixels_per_second),
        }
    }

    /// Sets zoom with deterministic clamping.
    pub fn set_pixels_per_second(&mut self, pixels_per_second: f32) {
        self.pixels_per_second = sanitize_timeline_zoom(pixels_per_second);
    }
}

impl Default for TimelineZoom {
    fn default() -> Self {
        Self::new(DEFAULT_TIMELINE_PIXELS_PER_SECOND)
    }
}

/// Timeline viewport scale and horizontal scroll state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScale {
    /// UI logical x coordinate of the viewport/ruler origin.
    pub origin_x: f32,
    /// Viewport width in logical units.
    pub viewport_width: f32,
    /// Timeline content range.
    pub content_range: TimelineRange,
    /// Logical pixels per second.
    pub zoom: TimelineZoom,
    /// Horizontal scroll offset in logical pixels from `content_range.start`.
    pub scroll_offset: f32,
}

impl TimelineScale {
    /// Creates timeline scale state.
    #[must_use]
    pub const fn new(
        origin_x: f32,
        viewport_width: f32,
        content_range: TimelineRange,
        zoom: TimelineZoom,
        scroll_offset: f32,
    ) -> Self {
        Self {
            origin_x,
            viewport_width,
            content_range,
            zoom,
            scroll_offset,
        }
    }

    /// Returns a copy with finite coordinates, clamped zoom, and clamped scroll.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let content_range = self.content_range.sanitized();
        let zoom = self.zoom.sanitized();
        let viewport_width = finite_f32_non_negative(self.viewport_width);
        let max_scroll_offset = max_timeline_scroll_offset(content_range, zoom, viewport_width);
        Self {
            origin_x: finite_f32_or_zero(self.origin_x),
            viewport_width,
            content_range,
            zoom,
            scroll_offset: clamp_timeline_scroll_offset(self.scroll_offset, max_scroll_offset),
        }
    }

    /// Returns the maximum valid scroll offset in logical pixels.
    #[must_use]
    pub fn max_scroll_offset(self) -> f32 {
        let scale = self.sanitized();
        max_timeline_scroll_offset(scale.content_range, scale.zoom, scale.viewport_width)
    }

    /// Returns the visible time range represented by this scale.
    #[must_use]
    pub fn visible_range(self) -> TimelineRange {
        let scale = self.sanitized();
        let seconds_per_pixel = 1.0 / f64::from(scale.zoom.pixels_per_second);
        let start = scale.content_range.start.seconds()
            + f64::from(scale.scroll_offset) * seconds_per_pixel;
        let end = start + f64::from(scale.viewport_width) * seconds_per_pixel;
        TimelineRange::seconds(start, end.min(scale.content_range.end.seconds())).sanitized()
    }

    /// Converts timeline time to UI logical screen x.
    #[must_use]
    pub fn time_to_screen_x(self, time: TimelineTime) -> f32 {
        let scale = self.sanitized();
        let content_seconds = time.sanitized().seconds() - scale.content_range.start.seconds();
        finite_f64_to_f32(
            f64::from(scale.origin_x - scale.scroll_offset)
                + content_seconds * f64::from(scale.zoom.pixels_per_second),
        )
    }

    /// Converts UI logical screen x to timeline time.
    #[must_use]
    pub fn screen_x_to_time(self, x: f32) -> TimelineTime {
        let scale = self.sanitized();
        let content_x = finite_f32_or_zero(x) - scale.origin_x + scale.scroll_offset;
        TimelineTime::from_seconds(
            scale.content_range.start.seconds()
                + f64::from(content_x) / f64::from(scale.zoom.pixels_per_second),
        )
        .sanitized()
    }

    /// Converts a frame position to UI logical screen x.
    #[must_use]
    pub fn frame_to_screen_x(self, frame_rate: TimelineFrameRate, frame: TimelineFrame) -> f32 {
        self.time_to_screen_x(frame_rate.frame_to_time(frame))
    }

    /// Converts UI logical screen x to a frame position.
    #[must_use]
    pub fn screen_x_to_frame(
        self,
        frame_rate: TimelineFrameRate,
        x: f32,
        rounding: TimelineFrameRounding,
    ) -> TimelineFrame {
        frame_rate.time_to_frame(self.screen_x_to_time(x), rounding)
    }
}

/// Shared timeline descriptor state exposed by app-owned lane and item metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimelineDescriptorState {
    /// Descriptor is currently selected.
    pub selected: bool,
    /// Descriptor cannot currently be operated.
    pub disabled: bool,
    /// Descriptor is visible but not editable.
    pub read_only: bool,
}

impl TimelineDescriptorState {
    /// Marks this state as selected.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Marks this state as disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marks this state as read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
}

/// App-owned lane or track descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineLaneDescriptor {
    /// Stable lane identity.
    pub id: TimelineLaneId,
    /// Human-readable lane label.
    pub label: String,
    /// Generic descriptor state.
    pub state: TimelineDescriptorState,
}

impl TimelineLaneDescriptor {
    /// Creates a lane descriptor.
    #[must_use]
    pub fn new(id: TimelineLaneId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            state: TimelineDescriptorState::default(),
        }
    }

    /// Sets descriptor state.
    #[must_use]
    pub const fn with_state(mut self, state: TimelineDescriptorState) -> Self {
        self.state = state;
        self
    }
}

/// Compatibility name for timeline lanes used as tracks.
pub type TimelineTrackDescriptor = TimelineLaneDescriptor;

/// App-owned clip or item descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItemDescriptor {
    /// Stable item identity.
    pub id: TimelineItemId,
    /// Lane containing this item.
    pub lane: TimelineLaneId,
    /// Source timeline range. Resolution sanitizes ordering and finiteness.
    pub time_range: TimelineRange,
    /// Human-readable item label.
    pub label: String,
    /// Generic descriptor state.
    pub state: TimelineDescriptorState,
}

impl TimelineItemDescriptor {
    /// Creates an item descriptor.
    #[must_use]
    pub fn new(
        id: TimelineItemId,
        lane: TimelineLaneId,
        time_range: TimelineRange,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id,
            lane,
            time_range,
            label: label.into(),
            state: TimelineDescriptorState::default(),
        }
    }

    /// Sets descriptor state.
    #[must_use]
    pub const fn with_state(mut self, state: TimelineDescriptorState) -> Self {
        self.state = state;
        self
    }
}

/// Compatibility name for timeline items used as clips.
pub type TimelineClipDescriptor = TimelineItemDescriptor;

/// App-owned marker descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineMarkerDescriptor {
    /// Stable marker identity.
    pub id: TimelineMarkerId,
    /// Marker time.
    pub time: TimelineTime,
    /// Human-readable marker label.
    pub label: String,
    /// Generic descriptor state.
    pub state: TimelineDescriptorState,
}

impl TimelineMarkerDescriptor {
    /// Creates a marker descriptor.
    #[must_use]
    pub fn new(id: TimelineMarkerId, time: TimelineTime, label: impl Into<String>) -> Self {
        Self {
            id,
            time,
            label: label.into(),
            state: TimelineDescriptorState::default(),
        }
    }

    /// Sets descriptor state.
    #[must_use]
    pub const fn with_state(mut self, state: TimelineDescriptorState) -> Self {
        self.state = state;
        self
    }
}

/// App-owned keyframe descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineKeyframeDescriptor {
    /// Stable keyframe identity.
    pub id: TimelineKeyframeId,
    /// Item containing this keyframe.
    pub item: TimelineItemId,
    /// Keyframe time.
    pub time: TimelineTime,
    /// Human-readable keyframe label.
    pub label: String,
    /// Generic descriptor state.
    pub state: TimelineDescriptorState,
}

impl TimelineKeyframeDescriptor {
    /// Creates a keyframe descriptor.
    #[must_use]
    pub fn new(id: TimelineKeyframeId, item: TimelineItemId, time: TimelineTime) -> Self {
        Self {
            id,
            item,
            time,
            label: format!("Keyframe {}", id.raw()),
            state: TimelineDescriptorState::default(),
        }
    }

    /// Sets the keyframe label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets descriptor state.
    #[must_use]
    pub const fn with_state(mut self, state: TimelineDescriptorState) -> Self {
        self.state = state;
        self
    }
}

/// App-owned timeline descriptor set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimelineDescriptor {
    /// Lane descriptors in application presentation order.
    pub lanes: Vec<TimelineLaneDescriptor>,
    /// Clip/item descriptors.
    pub items: Vec<TimelineItemDescriptor>,
    /// Marker descriptors.
    pub markers: Vec<TimelineMarkerDescriptor>,
    /// Keyframe descriptors.
    pub keyframes: Vec<TimelineKeyframeDescriptor>,
}

impl TimelineDescriptor {
    /// Creates a timeline descriptor set.
    #[must_use]
    pub fn new(
        lanes: impl Into<Vec<TimelineLaneDescriptor>>,
        items: impl Into<Vec<TimelineItemDescriptor>>,
        markers: impl Into<Vec<TimelineMarkerDescriptor>>,
        keyframes: impl Into<Vec<TimelineKeyframeDescriptor>>,
    ) -> Self {
        Self {
            lanes: lanes.into(),
            items: items.into(),
            markers: markers.into(),
            keyframes: keyframes.into(),
        }
    }

    /// Validates stable IDs and descriptor references.
    ///
    /// # Errors
    ///
    /// Returns [`TimelineDescriptorError`] when descriptor IDs are duplicated or
    /// when an item/keyframe references a missing parent descriptor.
    pub fn validate(&self) -> Result<(), TimelineDescriptorError> {
        validate_timeline_descriptor(self)
    }
}

/// Structured timeline descriptor validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineDescriptorError {
    /// The descriptor contains a duplicate lane ID.
    DuplicateLaneId {
        /// Duplicated lane.
        id: TimelineLaneId,
    },
    /// The descriptor contains a duplicate clip/item ID.
    DuplicateItemId {
        /// Duplicated item.
        id: TimelineItemId,
    },
    /// The descriptor contains a duplicate marker ID.
    DuplicateMarkerId {
        /// Duplicated marker.
        id: TimelineMarkerId,
    },
    /// The descriptor contains a duplicate keyframe ID.
    DuplicateKeyframeId {
        /// Duplicated keyframe.
        id: TimelineKeyframeId,
    },
    /// An item references an unknown lane.
    UnknownItemLane {
        /// Item with the invalid lane reference.
        item: TimelineItemId,
        /// Missing lane.
        lane: TimelineLaneId,
    },
    /// A keyframe references an unknown item.
    UnknownKeyframeItem {
        /// Keyframe with the invalid item reference.
        keyframe: TimelineKeyframeId,
        /// Missing item.
        item: TimelineItemId,
    },
}

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
}

/// Builds backend-neutral semantic nodes for a resolved timeline layout.
#[must_use]
pub fn timeline_semantics(
    id: WidgetId,
    bounds: Rect,
    result: &TimelineLayoutResult<'_>,
    label: impl Into<String>,
) -> Vec<SemanticNode> {
    let root = timeline_root_semantics(id, bounds, result, label);
    let mut semantics = Vec::with_capacity(
        1 + result.lanes.len() + result.items.len() + result.markers.len() + result.keyframes.len(),
    );
    semantics.push(root);
    semantics.extend(
        result
            .lanes
            .iter()
            .map(|lane| timeline_lane_semantics(id, lane, result)),
    );
    semantics.extend(
        result
            .items
            .iter()
            .map(|item| timeline_item_semantics(id, item, result)),
    );
    semantics.extend(
        result
            .markers
            .iter()
            .map(|marker| timeline_marker_semantics(id, marker)),
    );
    semantics.extend(
        result
            .keyframes
            .iter()
            .map(|keyframe| timeline_keyframe_semantics(id, keyframe)),
    );
    semantics
}

/// Builds the timeline root semantic node.
#[must_use]
pub fn timeline_root_semantics(
    id: WidgetId,
    bounds: Rect,
    result: &TimelineLayoutResult<'_>,
    label: impl Into<String>,
) -> SemanticNode {
    let lane_ids = result
        .lanes
        .iter()
        .map(|lane| timeline_lane_widget_id(id, lane.descriptor.id));
    let marker_ids = result
        .markers
        .iter()
        .map(|marker| timeline_marker_widget_id(id, marker.descriptor.id));
    let mut node = SemanticNode::new(
        id,
        SemanticRole::Custom("timeline".to_owned()),
        finite_rect(bounds),
    )
    .with_label(label)
    .with_children(lane_ids.chain(marker_ids));
    node.state.value = Some(SemanticValue::Text(format!(
        "{} lanes, {} items, {} markers, {} keyframes",
        result.lanes.len(),
        result.items.len(),
        result.markers.len(),
        result.keyframes.len()
    )));
    node
}

/// Builds a timeline lane semantic node.
#[must_use]
pub fn timeline_lane_semantics(
    root: WidgetId,
    lane: &ResolvedTimelineLane<'_>,
    result: &TimelineLayoutResult<'_>,
) -> SemanticNode {
    let children = result
        .items
        .iter()
        .filter(|item| item.descriptor.lane == lane.descriptor.id)
        .map(|item| timeline_item_widget_id(root, item.descriptor.id))
        .collect::<Vec<_>>();
    let mut node = SemanticNode::new(
        timeline_lane_widget_id(root, lane.descriptor.id),
        SemanticRole::Custom("timeline-lane".to_owned()),
        lane.rect,
    )
    .with_label(lane.descriptor.label.clone())
    .with_children(children)
    .focusable(!lane.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, lane.descriptor.state);
    node.state.value = Some(SemanticValue::Text(lane.descriptor.label.clone()));
    node
}

/// Builds a timeline clip/item semantic node.
#[must_use]
pub fn timeline_item_semantics(
    root: WidgetId,
    item: &ResolvedTimelineItem<'_>,
    result: &TimelineLayoutResult<'_>,
) -> SemanticNode {
    let children = result
        .keyframes
        .iter()
        .filter(|keyframe| keyframe.item == item.descriptor.id)
        .map(|keyframe| timeline_keyframe_widget_id(root, keyframe.descriptor.id))
        .collect::<Vec<_>>();
    let mut node = SemanticNode::new(
        timeline_item_widget_id(root, item.descriptor.id),
        SemanticRole::Custom("timeline-item".to_owned()),
        item.rect,
    )
    .with_label(item.descriptor.label.clone())
    .with_children(children)
    .focusable(!item.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, item.descriptor.state);
    node.state.value = Some(SemanticValue::Text(item.descriptor.label.clone()));
    if !item.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select item",
        ));
    }
    node
}

/// Builds a timeline marker semantic node.
#[must_use]
pub fn timeline_marker_semantics(
    root: WidgetId,
    marker: &ResolvedTimelineMarker<'_>,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        timeline_marker_widget_id(root, marker.descriptor.id),
        SemanticRole::Custom("timeline-marker".to_owned()),
        marker.hit_rect,
    )
    .with_label(marker.descriptor.label.clone())
    .focusable(!marker.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, marker.descriptor.state);
    node.state.value = Some(SemanticValue::Text(marker.descriptor.label.clone()));
    if !marker.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select marker",
        ));
    }
    node
}

/// Builds a timeline keyframe semantic node.
#[must_use]
pub fn timeline_keyframe_semantics(
    root: WidgetId,
    keyframe: &ResolvedTimelineKeyframe<'_>,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        timeline_keyframe_widget_id(root, keyframe.descriptor.id),
        SemanticRole::Custom("timeline-keyframe".to_owned()),
        keyframe.hit_rect,
    )
    .with_label(keyframe.descriptor.label.clone())
    .focusable(!keyframe.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, keyframe.descriptor.state);
    node.state.value = Some(SemanticValue::Text(keyframe.descriptor.label.clone()));
    if !keyframe.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select keyframe",
        ));
    }
    node
}

/// Derives a stable semantic widget ID for a timeline lane.
#[must_use]
pub fn timeline_lane_widget_id(root: WidgetId, lane: TimelineLaneId) -> WidgetId {
    root.child(("timeline-lane", lane.raw()))
}

/// Derives a stable semantic widget ID for a timeline clip/item.
#[must_use]
pub fn timeline_item_widget_id(root: WidgetId, item: TimelineItemId) -> WidgetId {
    root.child(("timeline-item", item.raw()))
}

/// Derives a stable semantic widget ID for a timeline clip.
#[must_use]
pub fn timeline_clip_widget_id(root: WidgetId, clip: TimelineClipId) -> WidgetId {
    timeline_item_widget_id(root, clip)
}

/// Derives a stable semantic widget ID for a timeline marker.
#[must_use]
pub fn timeline_marker_widget_id(root: WidgetId, marker: TimelineMarkerId) -> WidgetId {
    root.child(("timeline-marker", marker.raw()))
}

/// Derives a stable semantic widget ID for a timeline keyframe.
#[must_use]
pub fn timeline_keyframe_widget_id(root: WidgetId, keyframe: TimelineKeyframeId) -> WidgetId {
    root.child(("timeline-keyframe", keyframe.raw()))
}

/// Ruler tick role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineRulerTickKind {
    /// Labeled primary tick.
    Major,
    /// Unlabeled subdivision tick.
    Minor,
}

/// Stable ruler tick metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRulerTick {
    /// Tick kind.
    pub kind: TimelineRulerTickKind,
    /// Tick frame.
    pub frame: TimelineFrame,
    /// Deterministic label. Minor ticks use an empty label.
    pub label: String,
}

impl TimelineRulerTick {
    /// Returns tick time for a frame rate.
    #[must_use]
    pub fn time(&self, frame_rate: TimelineFrameRate) -> TimelineTime {
        frame_rate.frame_to_time(self.frame)
    }
}

/// Ruler tick generation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRulerTickRequest {
    /// Visible time range.
    pub visible_range: TimelineRange,
    /// Frame-rate metadata.
    pub frame_rate: TimelineFrameRate,
    /// Timeline zoom.
    pub zoom: TimelineZoom,
    /// Upper bound for emitted ticks.
    pub max_ticks: usize,
}

impl TimelineRulerTickRequest {
    /// Creates a ruler tick request.
    #[must_use]
    pub const fn new(
        visible_range: TimelineRange,
        frame_rate: TimelineFrameRate,
        zoom: TimelineZoom,
    ) -> Self {
        Self {
            visible_range,
            frame_rate,
            zoom,
            max_ticks: DEFAULT_TIMELINE_RULER_MAX_TICKS,
        }
    }

    /// Sets a maximum tick count.
    #[must_use]
    pub const fn with_max_ticks(mut self, max_ticks: usize) -> Self {
        self.max_ticks = max_ticks;
        self
    }

    /// Emits deterministic finite ruler ticks.
    #[must_use]
    pub fn ticks(self) -> Vec<TimelineRulerTick> {
        timeline_ruler_ticks(self)
    }
}

/// Computes maximum horizontal scroll offset in logical pixels.
#[must_use]
pub fn max_timeline_scroll_offset(
    content_range: TimelineRange,
    zoom: TimelineZoom,
    viewport_width: f32,
) -> f32 {
    (content_range.content_width(zoom) - finite_f32_non_negative(viewport_width)).max(0.0)
}

/// Clamps a scroll offset between zero and the supplied maximum offset.
#[must_use]
pub fn clamp_timeline_scroll_offset(scroll_offset: f32, max_scroll_offset: f32) -> f32 {
    finite_f32_non_negative(scroll_offset).min(finite_f32_non_negative(max_scroll_offset))
}

/// Clamps a pixels-per-second zoom value.
#[must_use]
pub fn sanitize_timeline_zoom(pixels_per_second: f32) -> f32 {
    if pixels_per_second.is_finite() && pixels_per_second > 0.0 {
        pixels_per_second.clamp(
            MIN_TIMELINE_PIXELS_PER_SECOND,
            MAX_TIMELINE_PIXELS_PER_SECOND,
        )
    } else {
        DEFAULT_TIMELINE_PIXELS_PER_SECOND
    }
}

/// Emits deterministic finite ruler ticks for the requested visible range.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn timeline_ruler_ticks(request: TimelineRulerTickRequest) -> Vec<TimelineRulerTick> {
    let visible = request.visible_range.sanitized();
    if visible.is_empty() || request.max_ticks == 0 {
        return Vec::new();
    }

    let frame_rate = request.frame_rate.sanitized();
    let zoom = request.zoom.sanitized();
    let min_major_frames = ((80.0 / f64::from(zoom.pixels_per_second))
        * frame_rate.frames_per_second())
    .ceil()
    .max(1.0);
    let mut major_step = nice_frame_step(f64_to_i64_saturating(min_major_frames).max(1));
    let mut minor_step = (major_step / 5).max(1);

    let start_frame = frame_rate
        .time_to_frame(visible.start, TimelineFrameRounding::Floor)
        .raw();
    let end_frame = frame_rate
        .time_to_frame(visible.end, TimelineFrameRounding::Ceil)
        .raw();

    while tick_count(start_frame, end_frame, minor_step) > request.max_ticks {
        minor_step = major_step;
        if tick_count(start_frame, end_frame, minor_step) > request.max_ticks {
            let next_major_step = nice_frame_step(major_step.saturating_mul(2));
            if next_major_step <= major_step {
                break;
            }
            major_step = next_major_step;
            minor_step = major_step;
        }
    }

    let first = floor_to_step(start_frame, minor_step);
    let last = ceil_to_step(end_frame, minor_step);
    let mut ticks = Vec::new();
    let mut frame = first;
    while frame <= last && ticks.len() < request.max_ticks {
        let kind = if frame.rem_euclid(major_step) == 0 {
            TimelineRulerTickKind::Major
        } else {
            TimelineRulerTickKind::Minor
        };
        ticks.push(TimelineRulerTick {
            kind,
            frame: TimelineFrame::from_raw(frame),
            label: if kind == TimelineRulerTickKind::Major {
                timeline_timecode_label(TimelineFrame::from_raw(frame), frame_rate)
            } else {
                String::new()
            },
        });
        if minor_step <= 0 {
            break;
        }
        let Some(next_frame) = frame.checked_add(minor_step) else {
            break;
        };
        if next_frame <= frame {
            break;
        }
        frame = next_frame;
    }
    ticks
}

/// Returns a deterministic timecode-style label for a frame.
#[must_use]
pub fn timeline_timecode_label(frame: TimelineFrame, frame_rate: TimelineFrameRate) -> String {
    let display_fps = frame_rate.rounded_display_fps();
    let raw = frame.raw();
    let sign = if raw < 0 { "-" } else { "" };
    let frames = raw.saturating_abs();
    let frames_per_hour = display_fps.saturating_mul(3600);
    let frames_per_minute = display_fps.saturating_mul(60);
    let hours = frames / frames_per_hour;
    let minutes = (frames % frames_per_hour) / frames_per_minute;
    let seconds = (frames % frames_per_minute) / display_fps;
    let frame = frames % display_fps;

    format!("{sign}{hours:02}:{minutes:02}:{seconds:02}:{frame:02}")
}

fn validate_timeline_descriptor(
    descriptor: &TimelineDescriptor,
) -> Result<(), TimelineDescriptorError> {
    let mut lane_ids = BTreeSet::new();
    for lane in &descriptor.lanes {
        if !lane_ids.insert(lane.id) {
            return Err(TimelineDescriptorError::DuplicateLaneId { id: lane.id });
        }
    }

    let mut item_ids = BTreeSet::new();
    for item in &descriptor.items {
        if !item_ids.insert(item.id) {
            return Err(TimelineDescriptorError::DuplicateItemId { id: item.id });
        }
    }

    let mut marker_ids = BTreeSet::new();
    for marker in &descriptor.markers {
        if !marker_ids.insert(marker.id) {
            return Err(TimelineDescriptorError::DuplicateMarkerId { id: marker.id });
        }
    }

    let mut keyframe_ids = BTreeSet::new();
    for keyframe in &descriptor.keyframes {
        if !keyframe_ids.insert(keyframe.id) {
            return Err(TimelineDescriptorError::DuplicateKeyframeId { id: keyframe.id });
        }
    }

    for item in &descriptor.items {
        if !lane_ids.contains(&item.lane) {
            return Err(TimelineDescriptorError::UnknownItemLane {
                item: item.id,
                lane: item.lane,
            });
        }
    }

    for keyframe in &descriptor.keyframes {
        if !item_ids.contains(&keyframe.item) {
            return Err(TimelineDescriptorError::UnknownKeyframeItem {
                keyframe: keyframe.id,
                item: keyframe.item,
            });
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn resolve_timeline_layout(
    layout: TimelineLayout,
    bounds: Rect,
    scale: TimelineScale,
    descriptor: &TimelineDescriptor,
    scroll_offset: f32,
) -> Result<TimelineLayoutResult<'_>, TimelineDescriptorError> {
    descriptor.validate()?;

    let bounds = finite_rect(bounds);
    let window = timeline_lane_window(
        descriptor.lanes.len(),
        layout.row_height,
        bounds.height,
        scroll_offset,
        layout.overscan,
    );
    let row_height = finite_positive(layout.row_height).unwrap_or(0.0);
    let scale = scale.sanitized();
    let lane_indices = descriptor
        .lanes
        .iter()
        .enumerate()
        .map(|(index, lane)| (lane.id, index))
        .collect::<BTreeMap<_, _>>();
    let item_indices = descriptor
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<BTreeMap<_, _>>();

    let lanes = descriptor
        .lanes
        .iter()
        .enumerate()
        .skip(window.materialized_range.start)
        .take(
            window
                .materialized_range
                .end
                .saturating_sub(window.materialized_range.start),
        )
        .map(|(row_index, lane)| ResolvedTimelineLane {
            descriptor: lane,
            source_index: row_index,
            row_index,
            rect: timeline_lane_rect(bounds, row_height, window.clamped_scroll_offset, row_index),
        })
        .collect::<Vec<_>>();

    let materialized_lanes = window
        .materialized_range
        .clone()
        .collect::<BTreeSet<usize>>();
    let mut items = descriptor
        .items
        .iter()
        .enumerate()
        .filter_map(|(source_index, item)| {
            let lane_index = *lane_indices.get(&item.lane)?;
            if !materialized_lanes.contains(&lane_index) {
                return None;
            }
            resolve_timeline_item(
                source_index,
                item,
                lane_index,
                bounds,
                row_height,
                window.clamped_scroll_offset,
                scale,
            )
        })
        .collect::<Vec<_>>();
    items.sort_by(compare_resolved_timeline_items);

    let mut markers = descriptor
        .markers
        .iter()
        .enumerate()
        .filter_map(|(source_index, marker)| {
            resolve_timeline_marker(source_index, marker, bounds, scale, layout.marker_hit_width)
        })
        .collect::<Vec<_>>();
    markers.sort_by(compare_resolved_timeline_markers);

    let mut keyframes = descriptor
        .keyframes
        .iter()
        .enumerate()
        .filter_map(|(source_index, keyframe)| {
            let item_index = *item_indices.get(&keyframe.item)?;
            let item = descriptor.items.get(item_index)?;
            let lane_index = *lane_indices.get(&item.lane)?;
            if !materialized_lanes.contains(&lane_index) {
                return None;
            }
            resolve_timeline_keyframe(
                source_index,
                keyframe,
                lane_index,
                bounds,
                row_height,
                window.clamped_scroll_offset,
                scale,
                layout.keyframe_hit_size,
            )
        })
        .collect::<Vec<_>>();
    keyframes.sort_by(compare_resolved_timeline_keyframes);

    Ok(TimelineLayoutResult {
        bounds,
        content_height: window.content_extent,
        max_scroll_offset: window.max_scroll_offset,
        scroll_offset: window.clamped_scroll_offset,
        visible_lane_range: window.visible_range,
        materialized_lane_range: window.materialized_range,
        lanes,
        items,
        markers,
        keyframes,
    })
}

fn resolve_timeline_item(
    source_index: usize,
    item: &TimelineItemDescriptor,
    lane_index: usize,
    bounds: Rect,
    row_height: f32,
    scroll_offset: f32,
    scale: TimelineScale,
) -> Option<ResolvedTimelineItem<'_>> {
    let time_range = item.time_range.sanitized();
    let row = timeline_lane_rect(bounds, row_height, scroll_offset, lane_index);
    let start_x = scale.time_to_screen_x(time_range.start);
    let end_x = scale.time_to_screen_x(time_range.end);
    let left = start_x.min(end_x);
    let right = start_x.max(end_x);
    let unclipped_rect = finite_rect(Rect::new(left, row.y, right - left, row.height));
    let rect = intersect_rect(unclipped_rect, bounds)?;
    let visible_time_range = TimelineRange::new(
        scale.screen_x_to_time(rect.x),
        scale.screen_x_to_time(rect_max_x(rect)),
    )
    .sanitized();

    Some(ResolvedTimelineItem {
        descriptor: item,
        source_index,
        lane_index,
        time_range,
        visible_time_range,
        rect,
        unclipped_rect,
    })
}

fn resolve_timeline_marker(
    source_index: usize,
    marker: &TimelineMarkerDescriptor,
    bounds: Rect,
    scale: TimelineScale,
    hit_width: f32,
) -> Option<ResolvedTimelineMarker<'_>> {
    let time = marker.time.sanitized();
    let x = scale.time_to_screen_x(time);
    let width = finite_positive(hit_width).unwrap_or(1.0);
    let hit_rect = centered_rect(x, bounds.y + bounds.height * 0.5, width, bounds.height);
    let hit_rect = intersect_rect(hit_rect, bounds)?;

    Some(ResolvedTimelineMarker {
        descriptor: marker,
        source_index,
        time,
        x,
        hit_rect,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_timeline_keyframe(
    source_index: usize,
    keyframe: &TimelineKeyframeDescriptor,
    lane_index: usize,
    bounds: Rect,
    row_height: f32,
    scroll_offset: f32,
    scale: TimelineScale,
    hit_size: f32,
) -> Option<ResolvedTimelineKeyframe<'_>> {
    let time = keyframe.time.sanitized();
    let x = scale.time_to_screen_x(time);
    let row = timeline_lane_rect(bounds, row_height, scroll_offset, lane_index);
    let size = finite_positive(hit_size).unwrap_or(1.0);
    let hit_rect = centered_rect(x, row.y + row.height * 0.5, size, size);
    let hit_rect = intersect_rect(hit_rect, bounds)?;

    Some(ResolvedTimelineKeyframe {
        descriptor: keyframe,
        source_index,
        item: keyframe.item,
        lane_index,
        time,
        x,
        hit_rect,
    })
}

fn apply_timeline_semantic_state(node: &mut SemanticNode, state: TimelineDescriptorState) {
    node.state.disabled = state.disabled;
    node.state.selected = state.selected;
    if state.read_only {
        node.description = Some("Read-only".to_owned());
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TimelineLaneWindow {
    content_extent: f32,
    max_scroll_offset: f32,
    clamped_scroll_offset: f32,
    visible_range: Range<usize>,
    materialized_range: Range<usize>,
}

impl TimelineLaneWindow {
    fn empty() -> Self {
        Self {
            content_extent: 0.0,
            max_scroll_offset: 0.0,
            clamped_scroll_offset: 0.0,
            visible_range: 0..0,
            materialized_range: 0..0,
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn timeline_lane_window(
    lane_count: usize,
    row_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
    overscan: usize,
) -> TimelineLaneWindow {
    let Some(row_height) = finite_positive(row_height) else {
        return TimelineLaneWindow::empty();
    };
    let Some(viewport_height) = finite_positive(viewport_height) else {
        return TimelineLaneWindow::empty();
    };
    if lane_count == 0 {
        return TimelineLaneWindow::empty();
    }

    let content_extent = finite_product_usize(lane_count, row_height);
    let max_scroll_offset = (content_extent - viewport_height).max(0.0);
    let clamped_scroll_offset = finite_f32_non_negative(scroll_offset).min(max_scroll_offset);
    let first = ((clamped_scroll_offset / row_height).floor() as usize).min(lane_count);
    let visible_end = ((clamped_scroll_offset + viewport_height) / row_height).ceil() as usize;
    let visible_end = visible_end.min(lane_count).max(first);
    let visible_range = first..visible_end;
    let materialized_visible = ((viewport_height / row_height).ceil() as usize)
        .saturating_add(1)
        .min(lane_count);
    let materialized_start = first.saturating_sub(overscan);
    let materialized_end = first
        .saturating_add(materialized_visible)
        .saturating_add(overscan)
        .min(lane_count);

    TimelineLaneWindow {
        content_extent,
        max_scroll_offset,
        clamped_scroll_offset,
        visible_range,
        materialized_range: materialized_start..materialized_end,
    }
}

#[allow(clippy::cast_precision_loss)]
fn finite_product_usize(count: usize, extent: f32) -> f32 {
    if extent.is_finite() {
        (count as f32 * extent).max(0.0)
    } else {
        0.0
    }
}

fn timeline_lane_rect(bounds: Rect, row_height: f32, scroll_offset: f32, row_index: usize) -> Rect {
    Rect::new(
        bounds.x,
        finite_sum(
            bounds.y,
            finite_sum(row_index_to_offset(row_index, row_height), -scroll_offset),
        ),
        bounds.width,
        row_height,
    )
}

#[allow(clippy::cast_precision_loss)]
fn row_index_to_offset(row_index: usize, row_height: f32) -> f32 {
    row_index as f32 * row_height
}

fn intersect_rect(rect: Rect, bounds: Rect) -> Option<Rect> {
    let rect = finite_rect(rect);
    let bounds = finite_rect(bounds);
    let left = rect.x.max(bounds.x);
    let top = rect.y.max(bounds.y);
    let right = rect_max_x(rect).min(rect_max_x(bounds));
    let bottom = rect_max_y(rect).min(rect_max_y(bounds));
    (right > left && bottom > top).then(|| Rect::new(left, top, right - left, bottom - top))
}

fn centered_rect(center_x: f32, center_y: f32, width: f32, height: f32) -> Rect {
    let width = finite_f32_non_negative(width);
    let height = finite_f32_non_negative(height);
    Rect::new(
        center_x - width * 0.5,
        center_y - height * 0.5,
        width,
        height,
    )
}

fn compare_resolved_timeline_items(
    left: &ResolvedTimelineItem<'_>,
    right: &ResolvedTimelineItem<'_>,
) -> std::cmp::Ordering {
    left.lane_index
        .cmp(&right.lane_index)
        .then_with(|| left.rect.x.total_cmp(&right.rect.x))
        .then_with(|| left.rect.width.total_cmp(&right.rect.width))
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

fn compare_resolved_timeline_markers(
    left: &ResolvedTimelineMarker<'_>,
    right: &ResolvedTimelineMarker<'_>,
) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

fn compare_resolved_timeline_keyframes(
    left: &ResolvedTimelineKeyframe<'_>,
    right: &ResolvedTimelineKeyframe<'_>,
) -> std::cmp::Ordering {
    left.lane_index
        .cmp(&right.lane_index)
        .then_with(|| left.x.total_cmp(&right.x))
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

fn finite_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_f32_or_zero(rect.x),
        finite_f32_or_zero(rect.y),
        finite_f32_non_negative(rect.width),
        finite_f32_non_negative(rect.height),
    )
}

fn finite_positive(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn finite_sum(a: f32, b: f32) -> f32 {
    let sum = f64::from(finite_f32_or_zero(a)) + f64::from(finite_f32_or_zero(b));
    finite_f64_to_f32(sum)
}

fn rect_max_x(rect: Rect) -> f32 {
    finite_sum(rect.x, rect.width)
}

fn rect_max_y(rect: Rect) -> f32 {
    finite_sum(rect.y, rect.height)
}

fn finite_f32_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_f32_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_f64_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[allow(clippy::cast_possible_truncation)]
fn finite_f64_to_f32(value: f64) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn f64_to_i64_saturating(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

fn round_frame(value: f64, rounding: TimelineFrameRounding) -> i64 {
    let rounded = match rounding {
        TimelineFrameRounding::Floor => value.floor(),
        TimelineFrameRounding::Ceil => value.ceil(),
        TimelineFrameRounding::Nearest => value.round(),
        TimelineFrameRounding::Truncate => value.trunc(),
    };
    f64_to_i64_saturating(rounded)
}

fn nice_frame_step(min_frames: i64) -> i64 {
    let min_frames = min_frames.max(1);
    let mut magnitude = 1_i64;
    while magnitude.saturating_mul(10) < min_frames {
        magnitude = magnitude.saturating_mul(10);
    }

    for multiplier in [1_i64, 2, 5, 10] {
        let step = magnitude.saturating_mul(multiplier);
        if step >= min_frames {
            return step.max(1);
        }
    }
    magnitude.saturating_mul(10).max(1)
}

fn floor_to_step(value: i64, step: i64) -> i64 {
    value.div_euclid(step.max(1)).saturating_mul(step.max(1))
}

fn ceil_to_step(value: i64, step: i64) -> i64 {
    let step = step.max(1);
    let floor = floor_to_step(value, step);
    if floor == value {
        floor
    } else {
        floor.saturating_add(step)
    }
}

fn tick_count(start_frame: i64, end_frame: i64, step: i64) -> usize {
    let step = step.max(1);
    let first = floor_to_step(start_frame, step);
    let last = ceil_to_step(end_frame, step);
    if last < first {
        0
    } else {
        let span = i128::from(last) - i128::from(first);
        let count = span / i128::from(step) + 1;
        usize::try_from(count).unwrap_or(usize::MAX)
    }
}
