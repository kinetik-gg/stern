#[allow(clippy::wildcard_imports)]
use super::*;

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

    pub(crate) fn rounded_display_fps(self) -> i64 {
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
