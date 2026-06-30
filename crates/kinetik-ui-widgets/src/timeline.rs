//! Data-only timeline ruler, frame-rate, lane, item, and coordinate contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, Point, Rect,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, WidgetId,
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

    /// Returns a new scale whose zoom changes while preserving the timeline time under `anchor_x`.
    #[must_use]
    pub fn zoom_around_anchor(self, anchor_x: f32, zoom: TimelineZoom) -> TimelineZoomAnchorResult {
        zoom_timeline_scale_around_anchor(self, anchor_x, zoom)
    }
}

/// Result of changing timeline zoom around a pointer or viewport anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineZoomAnchorResult {
    /// Scale after zoom and scroll clamping.
    pub scale: TimelineScale,
    /// Timeline time that was under the anchor before zooming.
    pub anchor_time: TimelineTime,
    /// Sanitized screen-space anchor.
    pub anchor_x: f32,
}

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

/// Generic transport intent metadata for action-backed playback controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransportControlIntent {
    /// Toggle between play and pause.
    PlayPause,
    /// Stop playback or preview.
    Stop,
    /// Step one unit backward.
    StepBackward,
    /// Step one unit forward.
    StepForward,
    /// Jump to the start of the available range.
    JumpToStart,
    /// Jump to the end of the available range.
    JumpToEnd,
    /// Jump to the previous marker.
    PreviousMarker,
    /// Jump to the next marker.
    NextMarker,
    /// Toggle loop playback.
    LoopToggle,
    /// Toggle playback constrained to the selected or marked range.
    RangePlaybackToggle,
}

impl TransportControlIntent {
    /// Returns a stable generic action ID for this transport intent.
    #[must_use]
    pub const fn default_action_id(self) -> &'static str {
        match self {
            Self::PlayPause => "transport.play-pause",
            Self::Stop => "transport.stop",
            Self::StepBackward => "transport.step-backward",
            Self::StepForward => "transport.step-forward",
            Self::JumpToStart => "transport.jump-to-start",
            Self::JumpToEnd => "transport.jump-to-end",
            Self::PreviousMarker => "transport.previous-marker",
            Self::NextMarker => "transport.next-marker",
            Self::LoopToggle => "transport.loop",
            Self::RangePlaybackToggle => "transport.range-playback",
        }
    }

    /// Returns a human-readable default label for this transport intent.
    #[must_use]
    pub const fn default_label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play/Pause",
            Self::Stop => "Stop",
            Self::StepBackward => "Step Backward",
            Self::StepForward => "Step Forward",
            Self::JumpToStart => "Jump to Start",
            Self::JumpToEnd => "Jump to End",
            Self::PreviousMarker => "Previous Marker",
            Self::NextMarker => "Next Marker",
            Self::LoopToggle => "Loop",
            Self::RangePlaybackToggle => "Range Playback",
        }
    }

    /// Returns the default control kind for this transport intent.
    #[must_use]
    pub const fn default_control_kind(self) -> TransportControlKind {
        match self {
            Self::LoopToggle | Self::RangePlaybackToggle => TransportControlKind::Toggle,
            Self::PlayPause
            | Self::Stop
            | Self::StepBackward
            | Self::StepForward
            | Self::JumpToStart
            | Self::JumpToEnd
            | Self::PreviousMarker
            | Self::NextMarker => TransportControlKind::Button,
        }
    }

    /// Creates a generic action descriptor for this transport intent.
    #[must_use]
    pub fn default_action_descriptor(self) -> ActionDescriptor {
        let mut action = ActionDescriptor::new(self.default_action_id(), self.default_label());
        if self.default_control_kind() == TransportControlKind::Toggle {
            action.state.checked = Some(false);
        }
        action
    }
}

/// Presentation kind for an action-backed transport control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransportControlKind {
    /// Momentary push-button style control.
    Button,
    /// Toggle/checkable style control.
    Toggle,
}

/// Optional timeline context captured with a transport action request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTransportContext {
    /// Timeline surface identity when the transport is associated with a timeline.
    pub timeline: TimelineId,
    /// Current playhead time at request construction, if known.
    pub playhead_time: Option<TimelineTime>,
    /// Current selected or marked playback range, if known.
    pub selection_range: Option<TimelineRange>,
}

impl TimelineTransportContext {
    /// Creates timeline transport context for a timeline surface.
    #[must_use]
    pub const fn new(timeline: TimelineId) -> Self {
        Self {
            timeline,
            playhead_time: None,
            selection_range: None,
        }
    }

    /// Captures current playhead time metadata.
    #[must_use]
    pub const fn with_playhead_time(mut self, playhead_time: TimelineTime) -> Self {
        self.playhead_time = Some(playhead_time);
        self
    }

    /// Captures current range metadata.
    #[must_use]
    pub const fn with_selection_range(mut self, selection_range: TimelineRange) -> Self {
        self.selection_range = Some(selection_range);
        self
    }

    fn sanitized(self) -> Self {
        Self {
            timeline: self.timeline,
            playhead_time: self.playhead_time.map(TimelineTime::sanitized),
            selection_range: self.selection_range.map(TimelineRange::sanitized),
        }
    }
}

/// Data-only request emitted by transport controls for application execution.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportActionRequest {
    /// Invoked action identity.
    pub action_id: ActionId,
    /// Generic transport intent used for presentation.
    pub intent: TransportControlIntent,
    /// Source surface that emitted the action request.
    pub source: ActionSource,
    /// Transport control presentation kind that emitted the request.
    pub control_kind: TransportControlKind,
    /// Optional timeline context captured with the request.
    pub timeline_context: Option<TimelineTransportContext>,
}

impl TransportActionRequest {
    /// Creates transport action request metadata.
    #[must_use]
    pub fn new(
        action_id: ActionId,
        intent: TransportControlIntent,
        source: ActionSource,
        control_kind: TransportControlKind,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Self {
        Self {
            action_id,
            intent,
            source,
            control_kind,
            timeline_context: timeline_context.map(TimelineTransportContext::sanitized),
        }
    }

    /// Converts this request to the shared action invocation boundary.
    #[must_use]
    pub fn action_invocation(&self, context: ActionContext) -> ActionInvocation {
        ActionInvocation::new(self.action_id.clone(), self.source, context)
    }
}

/// Data-only transport control descriptor backed by an app-owned action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportControlDescriptor {
    /// Stable transport control identity.
    pub id: TransportControlId,
    /// Generic transport intent used for presentation and semantics.
    pub intent: TransportControlIntent,
    /// Action metadata shared with menus, toolbars, shortcuts, and command palettes.
    pub action: ActionDescriptor,
    /// Preferred transport control presentation kind.
    pub control_kind: TransportControlKind,
}

impl TransportControlDescriptor {
    /// Creates a transport control from app-owned action metadata.
    #[must_use]
    pub fn new(
        id: TransportControlId,
        intent: TransportControlIntent,
        action: ActionDescriptor,
    ) -> Self {
        Self {
            id,
            intent,
            action,
            control_kind: intent.default_control_kind(),
        }
    }

    /// Creates a transport control with generic default action metadata.
    #[must_use]
    pub fn from_intent(id: TransportControlId, intent: TransportControlIntent) -> Self {
        Self::new(id, intent, intent.default_action_descriptor())
    }

    /// Sets the transport control presentation kind.
    #[must_use]
    pub const fn with_control_kind(mut self, control_kind: TransportControlKind) -> Self {
        self.control_kind = control_kind;
        self
    }

    /// Returns the backing action ID.
    #[must_use]
    pub const fn action_id(&self) -> &ActionId {
        &self.action.id
    }

    /// Returns true when the control should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the control can currently emit a request.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns checked/toggled action state when available.
    #[must_use]
    pub const fn checked(&self) -> Option<bool> {
        self.action.state.checked
    }

    /// Returns true when this control is visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates transport action request metadata when the backing action can invoke.
    #[must_use]
    pub fn request(
        &self,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.can_request().then(|| {
            TransportActionRequest::new(
                self.action.id.clone(),
                self.intent,
                source,
                self.control_kind,
                timeline_context,
            )
        })
    }

    /// Creates a shared action invocation when the backing action can invoke.
    #[must_use]
    pub fn action_invocation(&self, context: ActionContext) -> Option<ActionInvocation> {
        self.request(ActionSource::Button, None)
            .map(|request| request.action_invocation(context))
    }
}

/// Data-only transport control model preserving app-supplied presentation order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportControls {
    controls: Vec<TransportControlDescriptor>,
}

impl TransportControls {
    /// Creates an empty transport control model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates transport controls from ordered descriptors.
    #[must_use]
    pub fn from_controls(controls: impl IntoIterator<Item = TransportControlDescriptor>) -> Self {
        Self {
            controls: controls.into_iter().collect(),
        }
    }

    /// Creates generic transport controls from ordered intents.
    #[must_use]
    pub fn from_intents(intents: impl IntoIterator<Item = TransportControlIntent>) -> Self {
        Self::from_controls(intents.into_iter().enumerate().map(|(index, intent)| {
            TransportControlDescriptor::from_intent(
                TransportControlId::from_raw(usize_to_u64_saturating(index)),
                intent,
            )
        }))
    }

    /// Returns all transport controls in presentation order.
    #[must_use]
    pub fn controls(&self) -> &[TransportControlDescriptor] {
        &self.controls
    }

    /// Replaces transport controls.
    pub fn replace_controls(
        &mut self,
        controls: impl IntoIterator<Item = TransportControlDescriptor>,
    ) {
        self.controls = controls.into_iter().collect();
    }

    /// Returns a control by stable identity.
    #[must_use]
    pub fn control(&self, id: TransportControlId) -> Option<&TransportControlDescriptor> {
        self.controls.iter().find(|control| control.id == id)
    }

    /// Returns visible transport controls in presentation order.
    #[must_use]
    pub fn visible_controls(&self) -> Vec<&TransportControlDescriptor> {
        self.controls
            .iter()
            .filter(|control| control.visible())
            .collect()
    }

    /// Creates request metadata for a visible control index.
    #[must_use]
    pub fn request_for_visible(
        &self,
        visible_index: usize,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.visible_controls()
            .get(visible_index)
            .and_then(|control| control.request(source, timeline_context))
    }

    /// Creates request metadata for a stable transport control ID.
    #[must_use]
    pub fn request_for_control(
        &self,
        control_id: TransportControlId,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.control(control_id)
            .and_then(|control| control.request(source, timeline_context))
    }
}

/// Rect metadata used by transport semantic helper generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportControlSemanticRect {
    /// Stable transport control identity.
    pub id: TransportControlId,
    /// Control bounds.
    pub rect: Rect,
}

impl TransportControlSemanticRect {
    /// Creates semantic rect metadata for a transport control.
    #[must_use]
    pub const fn new(id: TransportControlId, rect: Rect) -> Self {
        Self { id, rect }
    }
}

/// Builds backend-neutral semantic nodes for transport controls.
#[must_use]
pub fn transport_controls_semantics(
    root: WidgetId,
    bounds: Rect,
    label: impl Into<String>,
    controls: &TransportControls,
    rects: impl IntoIterator<Item = TransportControlSemanticRect>,
) -> Vec<SemanticNode> {
    let rects = rects
        .into_iter()
        .map(|rect| (rect.id, rect.rect))
        .collect::<BTreeMap<_, _>>();
    let children = controls
        .visible_controls()
        .into_iter()
        .filter(|control| rects.contains_key(&control.id))
        .map(|control| transport_control_widget_id(root, control.id))
        .collect::<Vec<_>>();
    let mut nodes = Vec::with_capacity(children.len() + 1);
    nodes.push(
        SemanticNode::new(
            root,
            SemanticRole::Custom("transport-controls".to_owned()),
            finite_rect(bounds),
        )
        .with_label(label)
        .with_children(children),
    );
    nodes.extend(
        controls
            .visible_controls()
            .into_iter()
            .filter_map(|control| {
                let rect = *rects.get(&control.id)?;
                transport_control_semantics(root, rect, control)
            }),
    );
    nodes
}

/// Builds a backend-neutral semantic node for one visible transport control.
#[must_use]
pub fn transport_control_semantics(
    root: WidgetId,
    rect: Rect,
    control: &TransportControlDescriptor,
) -> Option<SemanticNode> {
    if !control.visible() {
        return None;
    }

    let enabled = control.enabled();
    let role = match control.control_kind {
        TransportControlKind::Button => SemanticRole::Button,
        TransportControlKind::Toggle => SemanticRole::Toggle,
    };
    let mut node = SemanticNode::new(
        transport_control_widget_id(root, control.id),
        role,
        finite_rect(rect),
    )
    .with_label(control.action.label.clone())
    .focusable(enabled);
    node.description.clone_from(&control.action.tooltip);
    node.state.disabled = !enabled;
    node.state.checked = control.checked();
    node.state.selected = control.action.state.is_checked();
    node.state.value = Some(SemanticValue::Text(
        control.intent.default_label().to_owned(),
    ));
    if enabled {
        node.actions
            .push(SemanticAction::from_action_descriptor(&control.action));
    }
    Some(node)
}

/// Derives a stable semantic widget ID for a transport control.
#[must_use]
pub fn transport_control_widget_id(root: WidgetId, control: TransportControlId) -> WidgetId {
    root.child(("transport-control", control.raw()))
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

    fn sanitized(self) -> Self {
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

/// Resolves one timeline time against snap candidates without mutating descriptors.
#[must_use]
pub fn timeline_snap_time(
    requested_time: TimelineTime,
    candidates: &[TimelineSnapCandidate],
    tolerance_seconds: f64,
) -> TimelineSnapMetadata {
    let requested_time = requested_time.sanitized();
    let tolerance_seconds = finite_f64_or_zero(tolerance_seconds).max(0.0);
    let mut best: Option<(f64, TimelineSnapCandidate)> = None;

    for candidate in candidates {
        let candidate = TimelineSnapCandidate {
            time: candidate.time.sanitized(),
            source: candidate.source,
            target: candidate.target,
        };
        let distance = (candidate.time.seconds() - requested_time.seconds()).abs();
        if distance > tolerance_seconds {
            continue;
        }

        let replace = best.is_none_or(|(best_distance, best_candidate)| {
            let distance_order = distance.total_cmp(&best_distance);
            distance_order.is_lt()
                || (distance_order.is_eq()
                    && compare_snap_candidates(candidate, best_candidate).is_lt())
        });
        if replace {
            best = Some((distance, candidate));
        }
    }

    best.map_or_else(
        || TimelineSnapMetadata::unsnapped(requested_time),
        |(_, candidate)| {
            TimelineSnapMetadata::snapped(
                requested_time,
                candidate.time,
                candidate.source,
                candidate.target,
            )
        },
    )
}

/// Collects deterministic snap candidates without mutating descriptors or app state.
#[must_use]
pub fn timeline_snap_candidates(
    request: TimelineSnapCandidateRequest<'_>,
) -> Vec<TimelineSnapCandidate> {
    let mut candidates = Vec::new();
    append_frame_snap_candidates(&mut candidates, request);

    if let Some(playhead_time) = request.playhead_time {
        candidates.push(TimelineSnapCandidate::new(
            playhead_time.sanitized(),
            TimelineSnapSource::Playhead,
            Some(TimelineHitTarget::Playhead(request.timeline)),
        ));
    }

    if let Some(range) = request.selection_range {
        let range = range.sanitized();
        candidates.push(TimelineSnapCandidate::new(
            range.start,
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeStartHandle(request.timeline)),
        ));
        candidates.push(TimelineSnapCandidate::new(
            range.end,
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeEndHandle(request.timeline)),
        ));
    }

    for item in &request.descriptor.items {
        let range = item.time_range.sanitized();
        candidates.push(TimelineSnapCandidate::new(
            range.start,
            TimelineSnapSource::ItemBoundary,
            Some(TimelineHitTarget::ItemTrimStartHandle(item.id)),
        ));
        candidates.push(TimelineSnapCandidate::new(
            range.end,
            TimelineSnapSource::ItemBoundary,
            Some(TimelineHitTarget::ItemTrimEndHandle(item.id)),
        ));
    }

    for marker in &request.descriptor.markers {
        candidates.push(TimelineSnapCandidate::new(
            marker.time.sanitized(),
            TimelineSnapSource::Marker,
            Some(TimelineHitTarget::Marker(marker.id)),
        ));
    }

    for keyframe in &request.descriptor.keyframes {
        candidates.push(TimelineSnapCandidate::new(
            keyframe.time.sanitized(),
            TimelineSnapSource::Keyframe,
            Some(TimelineHitTarget::Keyframe(keyframe.id)),
        ));
    }

    candidates.sort_by(|left, right| {
        left.time
            .seconds()
            .total_cmp(&right.time.seconds())
            .then_with(|| compare_snap_candidates(*left, *right))
    });
    candidates
}

fn zoom_timeline_scale_around_anchor(
    scale: TimelineScale,
    anchor_x: f32,
    zoom: TimelineZoom,
) -> TimelineZoomAnchorResult {
    let scale = scale.sanitized();
    let anchor_x = finite_f32_or_zero(anchor_x);
    let anchor_time = scale.screen_x_to_time(anchor_x);
    let zoom = zoom.sanitized();
    let content_seconds = anchor_time.seconds() - scale.content_range.start.seconds();
    let anchor_local_x = f64::from(anchor_x - scale.origin_x);
    let requested_scroll_offset =
        finite_f64_to_f32(content_seconds * f64::from(zoom.pixels_per_second) - anchor_local_x);
    let max_scroll_offset =
        max_timeline_scroll_offset(scale.content_range, zoom, scale.viewport_width);
    let scale = TimelineScale {
        zoom,
        scroll_offset: clamp_timeline_scroll_offset(requested_scroll_offset, max_scroll_offset),
        ..scale
    }
    .sanitized();

    TimelineZoomAnchorResult {
        scale,
        anchor_time,
        anchor_x,
    }
}

fn sanitize_timeline_snap_metadata(snap: TimelineSnapMetadata) -> TimelineSnapMetadata {
    if snap.source == TimelineSnapSource::None {
        TimelineSnapMetadata::unsnapped(snap.requested_time.sanitized())
    } else {
        TimelineSnapMetadata::snapped(
            snap.requested_time.sanitized(),
            snap.snapped_time.sanitized(),
            snap.source,
            snap.target,
        )
    }
}

fn append_frame_snap_candidates(
    candidates: &mut Vec<TimelineSnapCandidate>,
    request: TimelineSnapCandidateRequest<'_>,
) {
    let range = request.range.sanitized();
    if range.is_empty() || request.max_frame_candidates == 0 {
        return;
    }

    let frame_rate = request.frame_rate.sanitized();
    let start = frame_rate
        .time_to_frame(range.start, TimelineFrameRounding::Ceil)
        .raw();
    let end = frame_rate
        .time_to_frame(range.end, TimelineFrameRounding::Floor)
        .raw();
    if end < start {
        return;
    }

    let mut frame = start;
    let mut emitted = 0_usize;
    while frame <= end && emitted < request.max_frame_candidates {
        candidates.push(TimelineSnapCandidate::new(
            frame_rate.frame_to_time(TimelineFrame::from_raw(frame)),
            TimelineSnapSource::Frame,
            None,
        ));
        emitted = emitted.saturating_add(1);
        let Some(next) = frame.checked_add(1) else {
            break;
        };
        frame = next;
    }
}

fn hit_test_timeline(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let bounds = finite_rect(result.bounds);
    let point = sanitize_point(point);
    if !bounds.contains_point(point) {
        return None;
    }

    let config = config.sanitized();
    let time = config.scale.screen_x_to_time(point.x);

    if let Some(hit) = hit_test_timeline_keyframes(result, point, time) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_items(result, point, time, config.item_trim_handle_width) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_markers(result, point, time) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_playhead(bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_range_handles(bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_lane_headers(result, bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_ruler(point, time, config) {
        return Some(hit);
    }

    Some(TimelineHitMetadata {
        target: TimelineHitTarget::Background(config.timeline),
        rect: bounds,
        time,
        state: TimelineDescriptorState::default(),
    })
}

fn hit_test_timeline_playhead(
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let playhead_time = config.playhead_time?;
    let x = config.scale.time_to_screen_x(playhead_time);
    let rect = centered_rect(
        x,
        bounds.y + bounds.height * 0.5,
        config.playhead_hit_width,
        bounds.height,
    );
    rect.contains_point(point).then_some(TimelineHitMetadata {
        target: TimelineHitTarget::Playhead(config.timeline),
        rect,
        time,
        state: TimelineDescriptorState::default(),
    })
}

fn hit_test_timeline_range_handles(
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let range = config.selection_range?;
    let start_x = config.scale.time_to_screen_x(range.start);
    let end_x = config.scale.time_to_screen_x(range.end);
    [
        (
            TimelineHitTarget::RangeStartHandle(config.timeline),
            start_x,
        ),
        (TimelineHitTarget::RangeEndHandle(config.timeline), end_x),
    ]
    .into_iter()
    .find_map(|(target, x)| {
        let rect = centered_rect(
            x,
            bounds.y + bounds.height * 0.5,
            config.range_handle_hit_width,
            bounds.height,
        );
        rect.contains_point(point).then_some(TimelineHitMetadata {
            target,
            rect,
            time,
            state: TimelineDescriptorState::default(),
        })
    })
}

fn hit_test_timeline_lane_headers(
    result: &TimelineLayoutResult<'_>,
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    if config.lane_header_width <= 0.0 {
        return None;
    }

    let header_rect = Rect::new(
        bounds.x,
        bounds.y,
        config.lane_header_width.min(bounds.width),
        bounds.height,
    );
    if !header_rect.contains_point(point) {
        return None;
    }

    result
        .lanes
        .iter()
        .find(|lane| lane.rect.contains_point(point))
        .map(|lane| TimelineHitMetadata {
            target: TimelineHitTarget::LaneHeader(lane.descriptor.id),
            rect: lane.rect,
            time,
            state: lane.descriptor.state,
        })
}

fn hit_test_timeline_ruler(
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    config
        .ruler_rect
        .filter(|ruler_rect| ruler_rect.contains_point(point))
        .map(|ruler_rect| TimelineHitMetadata {
            target: TimelineHitTarget::Ruler(config.ruler),
            rect: ruler_rect,
            time,
            state: TimelineDescriptorState::default(),
        })
}

fn hit_test_timeline_keyframes(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
) -> Option<TimelineHitMetadata> {
    result
        .keyframes
        .iter()
        .rev()
        .find(|keyframe| keyframe.hit_rect.contains_point(point))
        .map(|keyframe| TimelineHitMetadata {
            target: TimelineHitTarget::Keyframe(keyframe.descriptor.id),
            rect: keyframe.hit_rect,
            time,
            state: keyframe.descriptor.state,
        })
}

fn hit_test_timeline_items(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
    trim_handle_width: f32,
) -> Option<TimelineHitMetadata> {
    let trim_handle_width = finite_f32_non_negative(trim_handle_width);
    result.items.iter().rev().find_map(|item| {
        if !item.rect.contains_point(point) {
            return None;
        }

        let start_rect = item_start_trim_rect(item.rect, trim_handle_width);
        if start_rect.contains_point(point) {
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::ItemTrimStartHandle(item.descriptor.id),
                rect: start_rect,
                time,
                state: item.descriptor.state,
            });
        }

        let end_rect = item_end_trim_rect(item.rect, trim_handle_width);
        if end_rect.contains_point(point) {
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::ItemTrimEndHandle(item.descriptor.id),
                rect: end_rect,
                time,
                state: item.descriptor.state,
            });
        }

        Some(TimelineHitMetadata {
            target: TimelineHitTarget::Item(item.descriptor.id),
            rect: item.rect,
            time,
            state: item.descriptor.state,
        })
    })
}

fn hit_test_timeline_markers(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
) -> Option<TimelineHitMetadata> {
    result
        .markers
        .iter()
        .rev()
        .find(|marker| marker.hit_rect.contains_point(point))
        .map(|marker| TimelineHitMetadata {
            target: TimelineHitTarget::Marker(marker.descriptor.id),
            rect: marker.hit_rect,
            time,
            state: marker.descriptor.state,
        })
}

fn compare_snap_candidates(
    left: TimelineSnapCandidate,
    right: TimelineSnapCandidate,
) -> std::cmp::Ordering {
    snap_source_rank(left.source)
        .cmp(&snap_source_rank(right.source))
        .then_with(|| left.time.seconds().total_cmp(&right.time.seconds()))
        .then_with(|| left.target.cmp(&right.target))
}

fn snap_source_rank(source: TimelineSnapSource) -> u8 {
    match source {
        TimelineSnapSource::Frame => 0,
        TimelineSnapSource::Playhead => 1,
        TimelineSnapSource::RangeBoundary => 2,
        TimelineSnapSource::ItemBoundary => 3,
        TimelineSnapSource::Marker => 4,
        TimelineSnapSource::Keyframe => 5,
        TimelineSnapSource::None => 6,
    }
}

fn clamped_timeline_drag_range(
    anchor_time: TimelineTime,
    current_time: TimelineTime,
    bounds: TimelineRange,
) -> TimelineRange {
    TimelineRange::new(
        clamp_timeline_time(anchor_time, bounds),
        clamp_timeline_time(current_time, bounds),
    )
    .sanitized()
}

fn clamp_timeline_time(time: TimelineTime, bounds: TimelineRange) -> TimelineTime {
    let bounds = bounds.sanitized();
    TimelineTime::from_seconds(
        time.sanitized()
            .seconds()
            .clamp(bounds.start.seconds(), bounds.end.seconds()),
    )
}

fn offset_timeline_range(range: TimelineRange, delta: TimelineTime) -> TimelineRange {
    let range = range.sanitized();
    let delta = delta.sanitized().seconds();
    TimelineRange::seconds(range.start.seconds() + delta, range.end.seconds() + delta)
}

fn item_start_trim_rect(rect: Rect, trim_handle_width: f32) -> Rect {
    let width = trim_handle_width.min(rect.width).max(0.0);
    Rect::new(rect.x, rect.y, width, rect.height)
}

fn item_end_trim_rect(rect: Rect, trim_handle_width: f32) -> Rect {
    let width = trim_handle_width.min(rect.width).max(0.0);
    Rect::new(rect_max_x(rect) - width, rect.y, width, rect.height)
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

fn sanitize_point(point: Point) -> Point {
    Point::new(finite_f32_or_zero(point.x), finite_f32_or_zero(point.y))
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

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
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
