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
fn ruler_ticks_are_deterministic_finite_and_ordered() {
    let request = TimelineRulerTickRequest::new(
        TimelineRange::seconds(0.0, 5.0),
        TimelineFrameRate::integer(24),
        TimelineZoom::new(120.0),
    );

    let first = request.ticks();
    let second = request.ticks();

    assert_eq!(first, second);
    assert!(!first.is_empty());
    assert!(
        first
            .iter()
            .all(|tick| tick.time(request.frame_rate).seconds().is_finite())
    );
    assert!(first.windows(2).all(|pair| pair[0].frame < pair[1].frame));
    assert!(
        first
            .iter()
            .any(|tick| tick.kind == TimelineRulerTickKind::Major)
    );
    assert!(
        first
            .iter()
            .any(|tick| tick.kind == TimelineRulerTickKind::Minor)
    );
    assert!(
        first.iter().any(|tick| {
            tick.kind == TimelineRulerTickKind::Major && tick.label == "00:00:00:00"
        })
    );
    assert!(
        first
            .iter()
            .filter(|tick| tick.kind == TimelineRulerTickKind::Minor)
            .all(|tick| tick.label.is_empty())
    );
}

#[test]
fn ruler_ticks_respect_max_tick_bound_for_large_ranges() {
    let ticks = TimelineRulerTickRequest::new(
        TimelineRange::seconds(0.0, 1_000_000.0),
        TimelineFrameRate::integer(24),
        TimelineZoom::new(1_000_000.0),
    )
    .with_max_ticks(128)
    .ticks();

    assert!(ticks.len() <= 128);
    assert!(ticks.windows(2).all(|pair| pair[0].frame < pair[1].frame));
}

#[test]
fn ruler_ticks_bound_saturated_finite_ranges_with_small_max_ticks() {
    let request = TimelineRulerTickRequest::new(
        TimelineRange::seconds(-1.0e20, 1.0e20),
        TimelineFrameRate::integer(24),
        TimelineZoom::default(),
    )
    .with_max_ticks(2);

    let ticks = request.ticks();
    let repeated = request.ticks();

    assert_eq!(ticks, repeated);
    assert!(ticks.len() <= 2);
    assert!(ticks.windows(2).all(|pair| pair[0].frame < pair[1].frame));
    assert!(
        ticks
            .iter()
            .all(|tick| tick.time(request.frame_rate).seconds().is_finite())
    );
}

#[test]
fn ruler_ticks_bound_saturated_finite_ranges_with_default_max_ticks() {
    let request = TimelineRulerTickRequest::new(
        TimelineRange::seconds(-1.0e20, 1.0e20),
        TimelineFrameRate::integer(24),
        TimelineZoom::default(),
    );

    let ticks = request.ticks();
    let repeated = request.ticks();

    assert_eq!(ticks, repeated);
    assert!(ticks.len() <= DEFAULT_TIMELINE_RULER_MAX_TICKS);
    assert!(ticks.windows(2).all(|pair| pair[0].frame < pair[1].frame));
    assert!(
        ticks
            .iter()
            .all(|tick| tick.time(request.frame_rate).seconds().is_finite())
    );
}

#[test]
fn timecode_labels_are_stable_for_positive_negative_and_fractional_rates() {
    assert_eq!(
        timeline_timecode_label(TimelineFrame::from_raw(49), TimelineFrameRate::integer(24)),
        "00:00:02:01"
    );
    assert_eq!(
        timeline_timecode_label(TimelineFrame::from_raw(-25), TimelineFrameRate::integer(24)),
        "-00:00:01:01"
    );
    assert_eq!(
        timeline_timecode_label(
            TimelineFrame::from_raw(30),
            TimelineFrameRate::new(30_000, 1001),
        ),
        "00:00:01:00"
    );
}
