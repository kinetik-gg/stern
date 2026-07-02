//! Timeline ruler and coordinate contract conformance tests.

mod timeline_conformance {
    use kinetik_ui_core::{Point, Rect, SemanticActionKind, SemanticRole, WidgetId};
    use kinetik_ui_widgets::{
        DEFAULT_TIMELINE_RULER_MAX_TICKS, TimelineDescriptor, TimelineDescriptorError,
        TimelineDescriptorState, TimelineFrame, TimelineFrameRate, TimelineFrameRounding,
        TimelineHitTarget, TimelineHitTestConfig, TimelineId, TimelineItemDescriptor,
        TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId,
        TimelineKeyframeSelectionRequest, TimelineLaneDescriptor, TimelineLaneId, TimelineLayout,
        TimelineMarkerDescriptor, TimelineMarkerId, TimelineRange,
        TimelineRangeSelectionUpdateRequest, TimelineRulerId, TimelineRulerTickKind,
        TimelineRulerTickRequest, TimelineScale, TimelineScrubUpdateRequest, TimelineSelection,
        TimelineSelectionOperation, TimelineSelectionTarget, TimelineSnapCandidate,
        TimelineSnapCandidateRequest, TimelineSnapMetadata, TimelineSnapSource, TimelineTime,
        TimelineTrimEdge, TimelineViewportState, TimelineZoom, TransportControlId,
        clamp_timeline_scroll_offset, max_timeline_scroll_offset, sanitize_timeline_zoom,
        timeline_item_widget_id, timeline_keyframe_widget_id, timeline_lane_widget_id,
        timeline_marker_widget_id, timeline_semantics, timeline_snap_candidates,
        timeline_snap_time, timeline_timecode_label,
    };

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_seconds_close(actual: TimelineTime, expected: f64) {
        assert!(
            (actual.seconds() - expected).abs() <= 0.000_001,
            "expected {} to equal {expected}",
            actual.seconds()
        );
    }

    fn scale() -> TimelineScale {
        TimelineScale::new(
            10.0,
            400.0,
            TimelineRange::seconds(0.0, 10.0),
            TimelineZoom::new(100.0),
            250.0,
        )
    }

    fn descriptor() -> TimelineDescriptor {
        TimelineDescriptor::new(
            [
                TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Video"),
                TimelineLaneDescriptor::new(TimelineLaneId::from_raw(2), "Audio")
                    .with_state(TimelineDescriptorState::default().selected(true)),
            ],
            [
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(10),
                    TimelineLaneId::from_raw(1),
                    TimelineRange::seconds(-1.0, 5.0),
                    "Opening",
                ),
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(11),
                    TimelineLaneId::from_raw(2),
                    TimelineRange::seconds(2.0, 4.0),
                    "Voice",
                )
                .with_state(
                    TimelineDescriptorState::default()
                        .disabled(true)
                        .read_only(true),
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
            )
            .with_label("Opacity")],
        )
    }

    fn timeline_layout_indexing_descriptor() -> TimelineDescriptor {
        let lanes = (0..6)
            .map(|raw| TimelineLaneDescriptor::new(TimelineLaneId::from_raw(raw), "Lane"))
            .collect::<Vec<_>>();
        TimelineDescriptor::new(
            lanes,
            [
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(10),
                    TimelineLaneId::from_raw(1),
                    TimelineRange::seconds(1.0, 2.0),
                    "Off lane",
                ),
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(11),
                    TimelineLaneId::from_raw(2),
                    TimelineRange::seconds(1.0, 2.0),
                    "Visible item",
                ),
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(12),
                    TimelineLaneId::from_raw(4),
                    TimelineRange::seconds(12.0, 13.0),
                    "Off time",
                ),
                TimelineItemDescriptor::new(
                    TimelineItemId::from_raw(13),
                    TimelineLaneId::from_raw(3),
                    TimelineRange::seconds(4.0, 5.0),
                    "Visible parent",
                ),
            ],
            [
                TimelineMarkerDescriptor::new(
                    TimelineMarkerId::from_raw(20),
                    TimelineTime::from_seconds(-2.0),
                    "Off time",
                ),
                TimelineMarkerDescriptor::new(
                    TimelineMarkerId::from_raw(21),
                    TimelineTime::from_seconds(6.0),
                    "Visible marker",
                ),
            ],
            [
                TimelineKeyframeDescriptor::new(
                    TimelineKeyframeId::from_raw(30),
                    TimelineItemId::from_raw(10),
                    TimelineTime::from_seconds(1.5),
                ),
                TimelineKeyframeDescriptor::new(
                    TimelineKeyframeId::from_raw(31),
                    TimelineItemId::from_raw(11),
                    TimelineTime::from_seconds(12.0),
                ),
                TimelineKeyframeDescriptor::new(
                    TimelineKeyframeId::from_raw(32),
                    TimelineItemId::from_raw(13),
                    TimelineTime::from_seconds(4.5),
                ),
            ],
        )
    }

    mod interaction;
    mod layout;
    mod model;
    mod ruler;
    mod semantics;
    mod snap;
    mod viewport;
}
