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
fn selection_targets_survive_lane_reorder_and_scroll_changes_by_stable_id() {
    let selection = TimelineSelection::from_targets([
        TimelineSelectionTarget::Item(TimelineItemId::from_raw(11)),
        TimelineSelectionTarget::Marker(TimelineMarkerId::from_raw(30)),
        TimelineSelectionTarget::Keyframe(TimelineKeyframeId::from_raw(40)),
    ]);
    let reordered = TimelineDescriptor::new(
        [
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(2), "Audio"),
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Video"),
        ],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(11),
                TimelineLaneId::from_raw(2),
                TimelineRange::seconds(2.0, 4.0),
                "Voice",
            ),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(10),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(0.0, 1.0),
                "Opening",
            ),
        ],
        [TimelineMarkerDescriptor::new(
            TimelineMarkerId::from_raw(30),
            TimelineTime::from_seconds(2.0),
            "Beat",
        )],
        [TimelineKeyframeDescriptor::new(
            TimelineKeyframeId::from_raw(40),
            TimelineItemId::from_raw(11),
            TimelineTime::from_seconds(3.0),
        )],
    );
    let state = TimelineViewportState::new(scale())
        .with_selection(selection.clone())
        .with_horizontal_scroll_offset(10_000.0)
        .with_lane_scroll_offset(100.0, 24.0);

    assert_eq!(state.selection, selection);
    assert_close(state.scale.scroll_offset, state.scale.max_scroll_offset());
    assert_close(state.lane_scroll_offset, 24.0);
    assert_eq!(
        state.selection.targets_in_descriptor_order(&reordered),
        vec![
            TimelineSelectionTarget::Item(TimelineItemId::from_raw(11)),
            TimelineSelectionTarget::Marker(TimelineMarkerId::from_raw(30)),
            TimelineSelectionTarget::Keyframe(TimelineKeyframeId::from_raw(40)),
        ]
    );
}

#[test]
fn range_selection_metadata_survives_zoom_and_normalizes_deterministically() {
    let state =
        TimelineViewportState::new(scale()).with_selection_range(TimelineRange::seconds(8.0, 3.0));

    let zoomed = state.with_zoom_around_anchor(210.0, TimelineZoom::new(220.0));

    let range = zoomed
        .state
        .selection_range
        .expect("range metadata survives zoom");
    assert_seconds_close(range.start, 3.0);
    assert_seconds_close(range.end, 8.0);
    assert_seconds_close(
        zoomed.anchor_time,
        scale().screen_x_to_time(210.0).seconds(),
    );
}

#[test]
fn zoom_around_anchor_preserves_anchor_time_under_pointer_when_possible() {
    let before = scale();
    let anchor_x = 180.0;
    let anchor_time = before.screen_x_to_time(anchor_x);

    let zoomed = before.zoom_around_anchor(anchor_x, TimelineZoom::new(250.0));

    assert_seconds_close(zoomed.anchor_time, anchor_time.seconds());
    assert_seconds_close(
        zoomed.scale.screen_x_to_time(anchor_x),
        anchor_time.seconds(),
    );
    assert_close(zoomed.scale.zoom.pixels_per_second, 250.0);
}

#[test]
fn scroll_clamping_preserves_playhead_range_and_snap_metadata() {
    let snap = TimelineSnapMetadata::snapped(
        TimelineTime::from_seconds(2.49),
        TimelineTime::from_seconds(2.5),
        TimelineSnapSource::Frame,
        None,
    );
    let state = TimelineViewportState::new(scale())
        .with_playhead_time(TimelineTime::from_seconds(2.0))
        .with_selection_range(TimelineRange::seconds(4.0, 1.0))
        .with_snap(snap)
        .with_horizontal_scroll_offset(f32::INFINITY)
        .with_lane_scroll_offset(f32::INFINITY, 48.0);

    assert_close(state.scale.scroll_offset, 0.0);
    assert_close(state.lane_scroll_offset, 0.0);
    assert_seconds_close(state.playhead_time.expect("playhead"), 2.0);
    assert_seconds_close(state.selection_range.expect("range").start, 1.0);
    assert_eq!(state.snap, Some(snap));
}
