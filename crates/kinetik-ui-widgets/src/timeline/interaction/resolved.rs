#[allow(clippy::wildcard_imports)]
use super::*;

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
