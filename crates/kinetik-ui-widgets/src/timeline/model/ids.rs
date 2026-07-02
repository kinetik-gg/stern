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
