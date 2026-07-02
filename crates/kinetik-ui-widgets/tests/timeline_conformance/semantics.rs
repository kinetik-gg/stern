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
        .find(|node| node.id == timeline_keyframe_widget_id(root, TimelineKeyframeId::from_raw(40)))
        .expect("keyframe semantics");
    assert_eq!(
        keyframe.role,
        SemanticRole::Custom("timeline-keyframe".to_owned())
    );
    assert!(keyframe.actions.iter().any(|action| {
        action.kind == SemanticActionKind::Invoke && action.label == "Select keyframe"
    }));
}
