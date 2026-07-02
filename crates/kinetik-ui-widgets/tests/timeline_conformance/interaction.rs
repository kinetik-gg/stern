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
fn playhead_seek_request_maps_pointer_x_to_time_and_frame() {
    let descriptor = TimelineDescriptor::new([], [], [], []);
    let timeline_scale = TimelineScale::new(
        0.0,
        240.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(24.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 240.0, 40.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");
    let config = TimelineHitTestConfig::new(
        TimelineId::from_raw(1),
        TimelineRulerId::from_raw(2),
        timeline_scale,
    );
    let requested = timeline_scale.screen_x_to_time(60.0);
    let snap = TimelineSnapMetadata::unsnapped(requested);

    let request = result.playhead_seek_request(60.0, TimelineFrameRate::integer(24), config, snap);

    assert_seconds_close(request.requested_time, 2.5);
    assert_seconds_close(request.snap.snapped_time, 2.5);
    assert_eq!(request.frame, TimelineFrame::from_raw(60));
}

#[test]
fn scrub_request_metadata_preserves_previous_current_source_and_capture() {
    let source = TimelineHitTarget::Playhead(TimelineId::from_raw(1));
    let previous_time = TimelineTime::from_seconds(1.0);
    let current_time = TimelineTime::from_seconds(2.0);
    let snap = TimelineSnapMetadata::snapped(
        current_time,
        TimelineTime::from_seconds(2.125),
        TimelineSnapSource::Frame,
        None,
    );

    let request = TimelineScrubUpdateRequest::new(source, previous_time, current_time, snap);

    assert_eq!(request.source, source);
    assert_seconds_close(request.previous_time, 1.0);
    assert_seconds_close(request.current_time, 2.0);
    assert_seconds_close(request.snap.snapped_time, 2.125);
    assert!(request.pointer_capture_requested);
}

#[test]
fn range_selection_clamps_and_normalizes_reversed_drag_ranges() {
    let request = TimelineRangeSelectionUpdateRequest::new(
        TimelineHitTarget::Background(TimelineId::from_raw(1)),
        TimelineTime::from_seconds(8.0),
        TimelineTime::from_seconds(-4.0),
        TimelineRange::seconds(0.0, 10.0),
        TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(-4.0)),
    );

    assert_seconds_close(request.anchor_time, 8.0);
    assert_seconds_close(request.current_time, -4.0);
    assert_seconds_close(request.range.start, 0.0);
    assert_seconds_close(request.range.end, 8.0);
    assert!(request.pointer_capture_requested);
}

#[test]
fn marker_hit_testing_has_priority_against_ruler_and_background() {
    let descriptor = TimelineDescriptor::new(
        [],
        [],
        [TimelineMarkerDescriptor::new(
            TimelineMarkerId::from_raw(30),
            TimelineTime::from_seconds(2.0),
            "Beat",
        )],
        [],
    );
    let timeline_scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(10.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .with_marker_hit_width(10.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 30.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");
    let config = TimelineHitTestConfig::new(
        TimelineId::from_raw(1),
        TimelineRulerId::from_raw(2),
        timeline_scale,
    )
    .with_ruler_rect(Rect::new(0.0, 0.0, 100.0, 30.0));

    let hit = result
        .hit_test(Point::new(20.0, 15.0), config)
        .expect("marker hit");

    assert_eq!(
        hit.target,
        TimelineHitTarget::Marker(TimelineMarkerId::from_raw(30))
    );
    assert_seconds_close(hit.time, 2.0);
}

#[test]
fn clip_body_and_trim_handle_hit_priority_is_deterministic() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(1.0, 4.0),
            "Clip",
        )],
        [],
        [],
    );
    let timeline_scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(10.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");
    let config = TimelineHitTestConfig::new(
        TimelineId::from_raw(1),
        TimelineRulerId::from_raw(2),
        timeline_scale,
    )
    .with_item_trim_handle_width(5.0);

    let start = result
        .hit_test(Point::new(11.0, 10.0), config)
        .expect("start trim hit");
    let body = result
        .hit_test(Point::new(25.0, 10.0), config)
        .expect("body hit");
    let end = result
        .hit_test(Point::new(39.0, 10.0), config)
        .expect("end trim hit");

    assert_eq!(
        start.target,
        TimelineHitTarget::ItemTrimStartHandle(TimelineItemId::from_raw(10))
    );
    assert_eq!(
        body.target,
        TimelineHitTarget::Item(TimelineItemId::from_raw(10))
    );
    assert_eq!(
        end.target,
        TimelineHitTarget::ItemTrimEndHandle(TimelineItemId::from_raw(10))
    );
}

#[test]
fn clip_move_request_reports_delta_without_mutating_descriptor() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(1.0, 3.0),
            "Clip",
        )],
        [],
        [],
    );
    let timeline_scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(10.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");
    let item = &result.items[0];
    let snap = TimelineSnapMetadata::snapped(
        TimelineTime::from_seconds(2.25),
        TimelineTime::from_seconds(2.5),
        TimelineSnapSource::Marker,
        Some(TimelineHitTarget::Marker(TimelineMarkerId::from_raw(30))),
    );

    let request = item
        .move_request(TimelineTime::from_seconds(1.25), snap)
        .expect("move request");

    assert_seconds_close(request.original_range.start, 1.0);
    assert_seconds_close(request.original_range.end, 3.0);
    assert_seconds_close(request.requested_delta, 1.25);
    assert_seconds_close(request.snapped_delta, 1.5);
    assert_seconds_close(request.requested_range.start, 2.25);
    assert_seconds_close(request.snapped_range.start, 2.5);
    assert_seconds_close(descriptor.items[0].time_range.start, 1.0);
    assert!(request.pointer_capture_requested);
}

#[test]
fn clip_trim_request_clamps_to_valid_start_end_and_reports_noop_state() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(1.0, 3.0),
            "Clip",
        )],
        [],
        [],
    );
    let timeline_scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(10.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");
    let item = &result.items[0];

    let clamped = item
        .trim_request(
            TimelineTrimEdge::Start,
            TimelineTime::from_seconds(5.0),
            TimelineRange::seconds(0.0, 10.0),
            TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(5.0)),
        )
        .expect("trim request");
    let noop = item
        .trim_request(
            TimelineTrimEdge::End,
            TimelineTime::from_seconds(3.0),
            TimelineRange::seconds(0.0, 10.0),
            TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(3.0)),
        )
        .expect("noop trim request");

    assert_eq!(clamped.edge, TimelineTrimEdge::Start);
    assert_seconds_close(clamped.clamped_time, 3.0);
    assert_seconds_close(clamped.clamped_range.start, 3.0);
    assert_seconds_close(clamped.clamped_range.end, 3.0);
    assert!(!clamped.is_noop());
    assert!(noop.is_noop());
    assert_seconds_close(descriptor.items[0].time_range.end, 3.0);
}

#[test]
fn keyframe_selection_request_preserves_stable_keyframe_id() {
    let descriptor = descriptor();
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 40.0),
            TimelineScale::new(
                0.0,
                100.0,
                TimelineRange::seconds(0.0, 10.0),
                TimelineZoom::new(10.0),
                0.0,
            ),
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");

    let request = result.keyframes[0]
        .selection_request(TimelineSelectionOperation::Toggle)
        .expect("keyframe selection");

    assert_eq!(
        request,
        TimelineKeyframeSelectionRequest {
            target: TimelineKeyframeId::from_raw(40),
            item: TimelineItemId::from_raw(11),
            operation: TimelineSelectionOperation::Toggle,
        }
    );
}

#[test]
fn disabled_and_read_only_descriptors_suppress_unavailable_requests() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(10),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 3.0),
                "Disabled",
            )
            .with_state(TimelineDescriptorState::default().disabled(true)),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(11),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(4.0, 6.0),
                "Read only",
            )
            .with_state(TimelineDescriptorState::default().read_only(true)),
        ],
        [TimelineMarkerDescriptor::new(
            TimelineMarkerId::from_raw(30),
            TimelineTime::from_seconds(2.0),
            "Disabled marker",
        )
        .with_state(TimelineDescriptorState::default().disabled(true))],
        [],
    );
    let timeline_scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(10.0),
        0.0,
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            timeline_scale,
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");

    let disabled = &result.items[0];
    let read_only = &result.items[1];
    let marker = &result.markers[0];

    assert!(
        disabled
            .selection_request(TimelineSelectionOperation::Replace)
            .is_none()
    );
    assert!(
        disabled
            .move_request(
                TimelineTime::from_seconds(1.0),
                TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(2.0)),
            )
            .is_none()
    );
    assert!(
        read_only
            .selection_request(TimelineSelectionOperation::Replace)
            .is_some()
    );
    assert!(
        read_only
            .move_request(
                TimelineTime::from_seconds(1.0),
                TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(5.0)),
            )
            .is_none()
    );
    assert!(
        read_only
            .trim_request(
                TimelineTrimEdge::End,
                TimelineTime::from_seconds(7.0),
                TimelineRange::seconds(0.0, 10.0),
                TimelineSnapMetadata::unsnapped(TimelineTime::from_seconds(7.0)),
            )
            .is_none()
    );
    assert!(
        marker
            .selection_request(TimelineSelectionOperation::Replace)
            .is_none()
    );
    assert!(marker.context_request().is_none());
}
