#[allow(clippy::wildcard_imports)]
use super::*;

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
