#[allow(unused_imports)]
use super::{
    DEFAULT_TIMELINE_RULER_MAX_TICKS, Point, Rect, SemanticActionKind, SemanticRole,
    TimelineDescriptor, TimelineDescriptorError, TimelineDescriptorState, TimelineFrame,
    TimelineFrameRate, TimelineFrameRounding, TimelineHitTarget, TimelineHitTestConfig, TimelineId,
    TimelineItemDescriptor, TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId,
    TimelineKeyframeSelectionRequest, TimelineLaneDescriptor, TimelineLaneId, TimelineLayout,
    TimelineMarkerDescriptor, TimelineMarkerId, TimelineRange, TimelineRangeSelectionUpdateRequest,
    TimelineRulerId, TimelineRulerTickKind, TimelineRulerTickRequest, TimelineScale,
    TimelineScrubUpdateRequest, TimelineSelection, TimelineSelectionOperation,
    TimelineSelectionTarget, TimelineSnapCandidate, TimelineSnapCandidateRequest,
    TimelineSnapMetadata, TimelineSnapSource, TimelineTime, TimelineTrimEdge,
    TimelineViewportState, TimelineZoom, TransportControlId, WidgetId, assert_close,
    assert_seconds_close, clamp_timeline_scroll_offset, descriptor, max_timeline_scroll_offset,
    sanitize_timeline_zoom, scale, timeline_item_widget_id, timeline_keyframe_widget_id,
    timeline_lane_widget_id, timeline_layout_indexing_descriptor, timeline_marker_widget_id,
    timeline_semantics, timeline_snap_candidates, timeline_snap_time, timeline_timecode_label,
};

#[test]
fn snap_metadata_reports_snapped_and_unsnapped_time_with_source_identity() {
    let requested = TimelineTime::from_seconds(2.04);
    let candidates = [
        TimelineSnapCandidate::new(
            TimelineTime::from_seconds(2.5),
            TimelineSnapSource::Marker,
            Some(TimelineHitTarget::Marker(TimelineMarkerId::from_raw(30))),
        ),
        TimelineSnapCandidate::new(
            TimelineTime::from_seconds(2.0),
            TimelineSnapSource::Keyframe,
            Some(TimelineHitTarget::Keyframe(TimelineKeyframeId::from_raw(
                40,
            ))),
        ),
    ];

    let snapped = timeline_snap_time(requested, &candidates, 0.1);
    let unsnapped = timeline_snap_time(requested, &candidates, 0.01);

    assert_seconds_close(snapped.requested_time, 2.04);
    assert_seconds_close(snapped.snapped_time, 2.0);
    assert_eq!(snapped.source, TimelineSnapSource::Keyframe);
    assert_eq!(
        snapped.target,
        Some(TimelineHitTarget::Keyframe(TimelineKeyframeId::from_raw(
            40
        )))
    );
    assert_seconds_close(unsnapped.requested_time, 2.04);
    assert_seconds_close(unsnapped.snapped_time, 2.04);
    assert_eq!(unsnapped.source, TimelineSnapSource::None);
    assert_eq!(unsnapped.target, None);
}

#[test]
fn snap_candidates_include_grid_markers_keyframes_clip_edges_and_range_boundaries() {
    let descriptor = descriptor();

    let candidates = timeline_snap_candidates(
        TimelineSnapCandidateRequest::new(
            TimelineId::from_raw(1),
            TimelineRange::seconds(1.9, 3.1),
            TimelineFrameRate::integer(2),
            &descriptor,
        )
        .with_selection_range(TimelineRange::seconds(1.5, 3.5)),
    );

    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Frame)
    );
    assert!(candidates.iter().any(|candidate| {
        candidate.source == TimelineSnapSource::RangeBoundary
            && candidate.target
                == Some(TimelineHitTarget::RangeStartHandle(TimelineId::from_raw(1)))
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.source == TimelineSnapSource::ItemBoundary
            && candidate.target
                == Some(TimelineHitTarget::ItemTrimStartHandle(
                    TimelineItemId::from_raw(11),
                ))
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.source == TimelineSnapSource::Marker
            && candidate.target == Some(TimelineHitTarget::Marker(TimelineMarkerId::from_raw(30)))
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.source == TimelineSnapSource::Keyframe
            && candidate.target
                == Some(TimelineHitTarget::Keyframe(TimelineKeyframeId::from_raw(
                    40,
                )))
    }));
}

#[test]
fn snap_resolution_uses_deterministic_priority_and_tie_breaking() {
    let requested = TimelineTime::from_seconds(2.0);
    let candidates = [
        TimelineSnapCandidate::new(
            TimelineTime::from_seconds(2.0),
            TimelineSnapSource::Marker,
            Some(TimelineHitTarget::Marker(TimelineMarkerId::from_raw(2))),
        ),
        TimelineSnapCandidate::new(
            TimelineTime::from_seconds(2.0),
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeStartHandle(TimelineId::from_raw(1))),
        ),
        TimelineSnapCandidate::new(
            TimelineTime::from_seconds(2.0),
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeEndHandle(TimelineId::from_raw(1))),
        ),
    ];

    let snapped = timeline_snap_time(requested, &candidates, 0.01);

    assert_eq!(snapped.source, TimelineSnapSource::RangeBoundary);
    assert_eq!(
        snapped.target,
        Some(TimelineHitTarget::RangeStartHandle(TimelineId::from_raw(1)))
    );
}
