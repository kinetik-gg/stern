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
fn timeline_descriptor_duplicate_ids_are_diagnosed_deterministically() {
    let duplicate_lane = TimelineDescriptor::new(
        [
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "A"),
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "B"),
        ],
        [],
        [],
        [],
    );

    assert_eq!(
        duplicate_lane.validate(),
        Err(TimelineDescriptorError::DuplicateLaneId {
            id: TimelineLaneId::from_raw(1)
        })
    );

    let duplicate_item = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "A",
        )],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(2),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(0.0, 1.0),
                "First",
            ),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(2),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 2.0),
                "Second",
            ),
        ],
        [],
        [],
    );

    assert_eq!(
        duplicate_item.validate(),
        Err(TimelineDescriptorError::DuplicateItemId {
            id: TimelineItemId::from_raw(2)
        })
    );
}

#[test]
fn timeline_lane_visible_and_materialized_ranges_are_deterministic() {
    let lanes = (0..6)
        .map(|raw| TimelineLaneDescriptor::new(TimelineLaneId::from_raw(raw), "Lane"))
        .collect::<Vec<_>>();
    let descriptor = TimelineDescriptor::new(lanes, [], [], []);

    let result = TimelineLayout::new(10.0)
        .with_overscan(1)
        .resolve(Rect::new(0.0, 0.0, 100.0, 24.0), scale(), &descriptor, 15.0)
        .expect("timeline layout resolves");

    assert_eq!(result.visible_lane_range, 1..4);
    assert_eq!(result.materialized_lane_range, 0..6);
    assert_close(result.content_height, 60.0);
    assert_close(result.max_scroll_offset, 36.0);
    assert_close(result.scroll_offset, 15.0);
    assert_eq!(
        result.materialized_lane_ids(),
        vec![
            TimelineLaneId::from_raw(0),
            TimelineLaneId::from_raw(1),
            TimelineLaneId::from_raw(2),
            TimelineLaneId::from_raw(3),
            TimelineLaneId::from_raw(4),
            TimelineLaneId::from_raw(5),
        ]
    );
}

#[test]
fn timeline_item_rectangles_clamp_to_visible_bounds_and_preserve_source_time() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(-5.0, 15.0),
            "Long clip",
        )],
        [],
        [],
    );
    let scale = TimelineScale::new(
        0.0,
        100.0,
        TimelineRange::seconds(0.0, 20.0),
        TimelineZoom::new(10.0),
        0.0,
    );

    let result = TimelineLayout::new(20.0)
        .resolve(Rect::new(0.0, 0.0, 100.0, 20.0), scale, &descriptor, 0.0)
        .expect("timeline layout resolves");
    let item = &result.items[0];

    assert_eq!(item.descriptor.id, TimelineItemId::from_raw(10));
    assert_seconds_close(item.time_range.start, -5.0);
    assert_seconds_close(item.time_range.end, 15.0);
    assert_eq!(item.rect, Rect::new(0.0, 0.0, 100.0, 20.0));
    assert_eq!(item.unclipped_rect, Rect::new(-50.0, 0.0, 200.0, 20.0));
    assert_seconds_close(item.visible_time_range.start, 0.0);
    assert_seconds_close(item.visible_time_range.end, 10.0);
}

#[test]
fn timeline_marker_and_keyframe_hit_rectangles_are_finite_and_stable() {
    let timeline = descriptor();
    let repeated_descriptor = descriptor();
    let result = TimelineLayout::new(20.0)
        .with_marker_hit_width(10.0)
        .with_keyframe_hit_size(8.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 40.0),
            TimelineScale::new(
                0.0,
                100.0,
                TimelineRange::seconds(0.0, 10.0),
                TimelineZoom::new(10.0),
                0.0,
            ),
            &timeline,
            0.0,
        )
        .expect("timeline layout resolves");
    let repeated = TimelineLayout::new(20.0)
        .with_marker_hit_width(10.0)
        .with_keyframe_hit_size(8.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 40.0),
            TimelineScale::new(
                0.0,
                100.0,
                TimelineRange::seconds(0.0, 10.0),
                TimelineZoom::new(10.0),
                0.0,
            ),
            &repeated_descriptor,
            0.0,
        )
        .expect("timeline layout resolves");

    assert_eq!(result.markers, repeated.markers);
    assert_eq!(result.keyframes, repeated.keyframes);
    assert_eq!(result.markers[0].hit_rect, Rect::new(15.0, 0.0, 10.0, 40.0));
    assert_eq!(
        result.keyframes[0].hit_rect,
        Rect::new(26.0, 26.0, 8.0, 8.0)
    );
    assert!(result.markers[0].x.is_finite());
    assert!(result.keyframes[0].x.is_finite());
}

#[test]
fn timeline_overlapping_items_use_stable_id_tie_breaking() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(9),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 3.0),
                "Later id",
            ),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(4),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 3.0),
                "Earlier id",
            ),
        ],
        [
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(12),
                TimelineTime::from_seconds(2.0),
                "B",
            ),
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(3),
                TimelineTime::from_seconds(2.0),
                "A",
            ),
        ],
        [],
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
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

    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.descriptor.id)
            .collect::<Vec<_>>(),
        vec![TimelineItemId::from_raw(4), TimelineItemId::from_raw(9)]
    );
    assert_eq!(
        result
            .markers
            .iter()
            .map(|marker| marker.descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            TimelineMarkerId::from_raw(3),
            TimelineMarkerId::from_raw(12)
        ]
    );
}

#[test]
fn timeline_layout_indexing_culls_off_lane_and_off_time_entities() {
    let descriptor = timeline_layout_indexing_descriptor();
    let result = TimelineLayout::new(10.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            TimelineScale::new(
                0.0,
                100.0,
                TimelineRange::seconds(0.0, 20.0),
                TimelineZoom::new(10.0),
                0.0,
            ),
            &descriptor,
            20.0,
        )
        .expect("timeline layout resolves");

    assert_eq!(result.visible_lane_range, 2..4);
    assert_eq!(result.materialized_lane_range, 2..5);
    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.descriptor.id)
            .collect::<Vec<_>>(),
        vec![TimelineItemId::from_raw(11), TimelineItemId::from_raw(13)]
    );
    assert_eq!(
        result
            .markers
            .iter()
            .map(|marker| marker.descriptor.id)
            .collect::<Vec<_>>(),
        vec![TimelineMarkerId::from_raw(21)]
    );
    assert_eq!(
        result
            .keyframes
            .iter()
            .map(|keyframe| keyframe.descriptor.id)
            .collect::<Vec<_>>(),
        vec![TimelineKeyframeId::from_raw(32)]
    );
}

#[test]
fn timeline_layout_indexing_keeps_boundary_visible_hit_targets() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [TimelineItemDescriptor::new(
            TimelineItemId::from_raw(10),
            TimelineLaneId::from_raw(1),
            TimelineRange::seconds(0.0, 10.0),
            "Item",
        )],
        [
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(20),
                TimelineTime::from_seconds(-0.51),
                "Outside left",
            ),
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(21),
                TimelineTime::from_seconds(-0.49),
                "Touches left",
            ),
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(22),
                TimelineTime::from_seconds(10.49),
                "Touches right",
            ),
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(23),
                TimelineTime::from_seconds(10.51),
                "Outside right",
            ),
        ],
        [
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(30),
                TimelineItemId::from_raw(10),
                TimelineTime::from_seconds(-0.51),
            ),
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(31),
                TimelineItemId::from_raw(10),
                TimelineTime::from_seconds(-0.49),
            ),
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(32),
                TimelineItemId::from_raw(10),
                TimelineTime::from_seconds(10.49),
            ),
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(33),
                TimelineItemId::from_raw(10),
                TimelineTime::from_seconds(10.51),
            ),
        ],
    );
    let result = TimelineLayout::new(20.0)
        .with_marker_hit_width(10.0)
        .with_keyframe_hit_size(10.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            TimelineScale::new(
                0.0,
                100.0,
                TimelineRange::seconds(-2.0, 12.0),
                TimelineZoom::new(10.0),
                20.0,
            ),
            &descriptor,
            0.0,
        )
        .expect("timeline layout resolves");

    assert_eq!(
        result
            .markers
            .iter()
            .map(|marker| marker.descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            TimelineMarkerId::from_raw(21),
            TimelineMarkerId::from_raw(22)
        ]
    );
    assert_eq!(
        result
            .keyframes
            .iter()
            .map(|keyframe| keyframe.descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            TimelineKeyframeId::from_raw(31),
            TimelineKeyframeId::from_raw(32)
        ]
    );
}

#[test]
fn timeline_layout_indexing_preserves_visible_tie_breaking() {
    let descriptor = TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(
            TimelineLaneId::from_raw(1),
            "Video",
        )],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(12),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 3.0),
                "Later item",
            ),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(4),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(1.0, 3.0),
                "Earlier item",
            ),
        ],
        [
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(22),
                TimelineTime::from_seconds(2.0),
                "Later marker",
            ),
            TimelineMarkerDescriptor::new(
                TimelineMarkerId::from_raw(3),
                TimelineTime::from_seconds(2.0),
                "Earlier marker",
            ),
        ],
        [
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(42),
                TimelineItemId::from_raw(12),
                TimelineTime::from_seconds(2.0),
            ),
            TimelineKeyframeDescriptor::new(
                TimelineKeyframeId::from_raw(7),
                TimelineItemId::from_raw(12),
                TimelineTime::from_seconds(2.0),
            ),
        ],
    );
    let result = TimelineLayout::new(20.0)
        .resolve(
            Rect::new(0.0, 0.0, 100.0, 20.0),
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

    assert_eq!(
        result
            .items
            .iter()
            .map(|item| item.descriptor.id)
            .collect::<Vec<_>>(),
        vec![TimelineItemId::from_raw(4), TimelineItemId::from_raw(12)]
    );
    assert_eq!(
        result
            .markers
            .iter()
            .map(|marker| marker.descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            TimelineMarkerId::from_raw(3),
            TimelineMarkerId::from_raw(22)
        ]
    );
    assert_eq!(
        result
            .keyframes
            .iter()
            .map(|keyframe| keyframe.descriptor.id)
            .collect::<Vec<_>>(),
        vec![
            TimelineKeyframeId::from_raw(7),
            TimelineKeyframeId::from_raw(42)
        ]
    );
}
