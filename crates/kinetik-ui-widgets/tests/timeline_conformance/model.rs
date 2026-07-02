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
fn timeline_ids_round_trip_raw_bits() {
    assert_eq!(TimelineId::from_raw(1).raw(), 1);
    assert_eq!(TimelineRulerId::from_raw(2).raw(), 2);
    assert_eq!(TransportControlId::from_raw(3).raw(), 3);
    assert_eq!(TimelineLaneId::from_raw(4).raw(), 4);
    assert_eq!(TimelineItemId::from_raw(5).raw(), 5);
    assert_eq!(TimelineMarkerId::from_raw(6).raw(), 6);
    assert_eq!(TimelineKeyframeId::from_raw(7).raw(), 7);
}

#[test]
fn integer_frame_rate_converts_time_and_frames() {
    let rate = TimelineFrameRate::integer(24);

    assert_seconds_close(rate.frame_to_time(TimelineFrame::from_raw(48)), 2.0);
    assert_eq!(
        rate.time_to_frame(
            TimelineTime::from_seconds(2.5),
            TimelineFrameRounding::Nearest,
        ),
        TimelineFrame::from_raw(60)
    );
    assert_eq!(
        rate.time_to_frame(
            TimelineTime::from_seconds(1.49),
            TimelineFrameRounding::Floor,
        ),
        TimelineFrame::from_raw(35)
    );
    assert_eq!(
        rate.time_to_frame(
            TimelineTime::from_seconds(1.49),
            TimelineFrameRounding::Ceil,
        ),
        TimelineFrame::from_raw(36)
    );
    assert_eq!(
        rate.time_to_frame(
            TimelineTime::from_seconds(-1.49),
            TimelineFrameRounding::Truncate,
        ),
        TimelineFrame::from_raw(-35)
    );
}

#[test]
fn fractional_frame_rate_preserves_rational_metadata() {
    let rate = TimelineFrameRate::new(24_000, 1001);

    assert_seconds_close(rate.frame_to_time(TimelineFrame::from_raw(24_000)), 1001.0);
    assert_eq!(
        rate.time_to_frame(
            TimelineTime::from_seconds(1001.0),
            TimelineFrameRounding::Nearest,
        ),
        TimelineFrame::from_raw(24_000)
    );
}

#[test]
fn time_and_frame_screen_conversions_round_trip() {
    let x = scale().time_to_screen_x(TimelineTime::from_seconds(4.0));
    let time = scale().screen_x_to_time(x);
    let frame_x =
        scale().frame_to_screen_x(TimelineFrameRate::integer(24), TimelineFrame::from_raw(96));
    let frame = scale().screen_x_to_frame(
        TimelineFrameRate::integer(24),
        frame_x,
        TimelineFrameRounding::Nearest,
    );

    assert_close(x, 160.0);
    assert_seconds_close(time, 4.0);
    assert_close(frame_x, 160.0);
    assert_eq!(frame, TimelineFrame::from_raw(96));
}

#[test]
fn visible_range_content_width_and_scroll_clamp_are_deterministic() {
    let range = TimelineRange::seconds(20.0, 0.0);
    let zoom = TimelineZoom::new(50.0);

    assert_close(range.content_width(zoom), 1000.0);
    assert_close(max_timeline_scroll_offset(range, zoom, 100.0), 900.0);
    assert_close(clamp_timeline_scroll_offset(-10.0, 900.0), 0.0);
    assert_close(clamp_timeline_scroll_offset(f32::INFINITY, 900.0), 0.0);
    assert_close(clamp_timeline_scroll_offset(1200.0, 900.0), 900.0);

    let visible = scale().visible_range();
    assert_seconds_close(visible.start, 2.5);
    assert_seconds_close(visible.end, 6.5);
}

#[test]
fn zoom_scroll_and_non_finite_inputs_sanitize() {
    assert_close(sanitize_timeline_zoom(f32::NAN), 100.0);
    assert_close(sanitize_timeline_zoom(-1.0), 100.0);
    assert_close(sanitize_timeline_zoom(0.000_000_1), 0.001);
    assert_close(sanitize_timeline_zoom(f32::MAX), 1_000_000.0);

    let sanitized = TimelineScale::new(
        f32::NAN,
        f32::INFINITY,
        TimelineRange::seconds(f64::NAN, 2.0),
        TimelineZoom::new(f32::NAN),
        f32::NEG_INFINITY,
    )
    .sanitized();

    assert_close(sanitized.origin_x, 0.0);
    assert_close(sanitized.viewport_width, 0.0);
    assert_seconds_close(sanitized.content_range.start, 0.0);
    assert_seconds_close(sanitized.content_range.end, 2.0);
    assert_close(sanitized.zoom.pixels_per_second, 100.0);
    assert_close(sanitized.scroll_offset, 0.0);
}
