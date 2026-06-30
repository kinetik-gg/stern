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
        TimelineRulerTickRequest, TimelineScale, TimelineScrubUpdateRequest,
        TimelineSelectionOperation, TimelineSnapCandidate, TimelineSnapMetadata,
        TimelineSnapSource, TimelineTime, TimelineTrimEdge, TimelineZoom, TransportControlId,
        clamp_timeline_scroll_offset, max_timeline_scroll_offset, sanitize_timeline_zoom,
        timeline_item_widget_id, timeline_keyframe_widget_id, timeline_lane_widget_id,
        timeline_marker_widget_id, timeline_semantics, timeline_snap_time, timeline_timecode_label,
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
        assert!(first.iter().any(|tick| {
            tick.kind == TimelineRulerTickKind::Major && tick.label == "00:00:00:00"
        }));
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
    fn timeline_descriptor_state_and_semantics_are_exposed_without_renderer_dependencies() {
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

        assert!(result.lanes[1].descriptor.state.selected);
        assert!(result.items[1].descriptor.state.disabled);
        assert!(result.items[1].descriptor.state.read_only);

        let root = WidgetId::from_key("timeline");
        let semantics = timeline_semantics(root, result.bounds, &result, "Timeline");

        assert_eq!(semantics[0].id, root);
        assert_eq!(
            semantics[0].role,
            SemanticRole::Custom("timeline".to_owned())
        );
        assert!(
            semantics[0]
                .children
                .contains(&timeline_lane_widget_id(root, TimelineLaneId::from_raw(1)))
        );
        assert!(semantics[0].children.contains(&timeline_marker_widget_id(
            root,
            TimelineMarkerId::from_raw(30)
        )));

        let selected_lane = semantics
            .iter()
            .find(|node| node.id == timeline_lane_widget_id(root, TimelineLaneId::from_raw(2)))
            .expect("selected lane semantics");
        assert_eq!(
            selected_lane.role,
            SemanticRole::Custom("timeline-lane".to_owned())
        );
        assert_eq!(selected_lane.label.as_deref(), Some("Audio"));
        assert!(selected_lane.state.selected);

        let disabled_item = semantics
            .iter()
            .find(|node| node.id == timeline_item_widget_id(root, TimelineItemId::from_raw(11)))
            .expect("disabled item semantics");
        assert_eq!(
            disabled_item.role,
            SemanticRole::Custom("timeline-item".to_owned())
        );
        assert!(disabled_item.state.disabled);
        assert_eq!(disabled_item.description.as_deref(), Some("Read-only"));
        assert!(!disabled_item.focusable);

        let keyframe = semantics
            .iter()
            .find(|node| {
                node.id == timeline_keyframe_widget_id(root, TimelineKeyframeId::from_raw(40))
            })
            .expect("keyframe semantics");
        assert_eq!(
            keyframe.role,
            SemanticRole::Custom("timeline-keyframe".to_owned())
        );
        assert!(keyframe.actions.iter().any(|action| {
            action.kind == SemanticActionKind::Invoke && action.label == "Select keyframe"
        }));
    }

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

        let request =
            result.playhead_seek_request(60.0, TimelineFrameRate::integer(24), config, snap);

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
}
