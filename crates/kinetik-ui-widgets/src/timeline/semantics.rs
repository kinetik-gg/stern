#[allow(clippy::wildcard_imports)]
use super::*;

/// Builds backend-neutral semantic nodes for a resolved timeline layout.
#[must_use]
pub fn timeline_semantics(
    id: WidgetId,
    bounds: Rect,
    result: &TimelineLayoutResult<'_>,
    label: impl Into<String>,
) -> Vec<SemanticNode> {
    let root = timeline_root_semantics(id, bounds, result, label);
    let mut semantics = Vec::with_capacity(
        1 + result.lanes.len() + result.items.len() + result.markers.len() + result.keyframes.len(),
    );
    semantics.push(root);
    semantics.extend(
        result
            .lanes
            .iter()
            .map(|lane| timeline_lane_semantics(id, lane, result)),
    );
    semantics.extend(
        result
            .items
            .iter()
            .map(|item| timeline_item_semantics(id, item, result)),
    );
    semantics.extend(
        result
            .markers
            .iter()
            .map(|marker| timeline_marker_semantics(id, marker)),
    );
    semantics.extend(
        result
            .keyframes
            .iter()
            .map(|keyframe| timeline_keyframe_semantics(id, keyframe)),
    );
    semantics
}

/// Builds the timeline root semantic node.
#[must_use]
pub fn timeline_root_semantics(
    id: WidgetId,
    bounds: Rect,
    result: &TimelineLayoutResult<'_>,
    label: impl Into<String>,
) -> SemanticNode {
    let lane_ids = result
        .lanes
        .iter()
        .map(|lane| timeline_lane_widget_id(id, lane.descriptor.id));
    let marker_ids = result
        .markers
        .iter()
        .map(|marker| timeline_marker_widget_id(id, marker.descriptor.id));
    let mut node = SemanticNode::new(
        id,
        SemanticRole::Custom("timeline".to_owned()),
        finite_rect(bounds),
    )
    .with_label(label)
    .with_children(lane_ids.chain(marker_ids));
    node.state.value = Some(SemanticValue::Text(format!(
        "{} lanes, {} items, {} markers, {} keyframes",
        result.lanes.len(),
        result.items.len(),
        result.markers.len(),
        result.keyframes.len()
    )));
    node
}

/// Builds a timeline lane semantic node.
#[must_use]
pub fn timeline_lane_semantics(
    root: WidgetId,
    lane: &ResolvedTimelineLane<'_>,
    result: &TimelineLayoutResult<'_>,
) -> SemanticNode {
    let children = result
        .items
        .iter()
        .filter(|item| item.descriptor.lane == lane.descriptor.id)
        .map(|item| timeline_item_widget_id(root, item.descriptor.id))
        .collect::<Vec<_>>();
    let mut node = SemanticNode::new(
        timeline_lane_widget_id(root, lane.descriptor.id),
        SemanticRole::Custom("timeline-lane".to_owned()),
        lane.rect,
    )
    .with_label(lane.descriptor.label.clone())
    .with_children(children)
    .focusable(!lane.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, lane.descriptor.state);
    node.state.value = Some(SemanticValue::Text(lane.descriptor.label.clone()));
    node
}

/// Builds a timeline clip/item semantic node.
#[must_use]
pub fn timeline_item_semantics(
    root: WidgetId,
    item: &ResolvedTimelineItem<'_>,
    result: &TimelineLayoutResult<'_>,
) -> SemanticNode {
    let children = result
        .keyframes
        .iter()
        .filter(|keyframe| keyframe.item == item.descriptor.id)
        .map(|keyframe| timeline_keyframe_widget_id(root, keyframe.descriptor.id))
        .collect::<Vec<_>>();
    let mut node = SemanticNode::new(
        timeline_item_widget_id(root, item.descriptor.id),
        SemanticRole::Custom("timeline-item".to_owned()),
        item.rect,
    )
    .with_label(item.descriptor.label.clone())
    .with_children(children)
    .focusable(!item.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, item.descriptor.state);
    node.state.value = Some(SemanticValue::Text(item.descriptor.label.clone()));
    if !item.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select item",
        ));
    }
    node
}

/// Builds a timeline marker semantic node.
#[must_use]
pub fn timeline_marker_semantics(
    root: WidgetId,
    marker: &ResolvedTimelineMarker<'_>,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        timeline_marker_widget_id(root, marker.descriptor.id),
        SemanticRole::Custom("timeline-marker".to_owned()),
        marker.hit_rect,
    )
    .with_label(marker.descriptor.label.clone())
    .focusable(!marker.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, marker.descriptor.state);
    node.state.value = Some(SemanticValue::Text(marker.descriptor.label.clone()));
    if !marker.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select marker",
        ));
    }
    node
}

/// Builds a timeline keyframe semantic node.
#[must_use]
pub fn timeline_keyframe_semantics(
    root: WidgetId,
    keyframe: &ResolvedTimelineKeyframe<'_>,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        timeline_keyframe_widget_id(root, keyframe.descriptor.id),
        SemanticRole::Custom("timeline-keyframe".to_owned()),
        keyframe.hit_rect,
    )
    .with_label(keyframe.descriptor.label.clone())
    .focusable(!keyframe.descriptor.state.disabled);
    apply_timeline_semantic_state(&mut node, keyframe.descriptor.state);
    node.state.value = Some(SemanticValue::Text(keyframe.descriptor.label.clone()));
    if !keyframe.descriptor.state.disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            "Select keyframe",
        ));
    }
    node
}

/// Derives a stable semantic widget ID for a timeline lane.
#[must_use]
pub fn timeline_lane_widget_id(root: WidgetId, lane: TimelineLaneId) -> WidgetId {
    root.child(("timeline-lane", lane.raw()))
}

/// Derives a stable semantic widget ID for a timeline clip/item.
#[must_use]
pub fn timeline_item_widget_id(root: WidgetId, item: TimelineItemId) -> WidgetId {
    root.child(("timeline-item", item.raw()))
}

/// Derives a stable semantic widget ID for a timeline clip.
#[must_use]
pub fn timeline_clip_widget_id(root: WidgetId, clip: TimelineClipId) -> WidgetId {
    timeline_item_widget_id(root, clip)
}

/// Derives a stable semantic widget ID for a timeline marker.
#[must_use]
pub fn timeline_marker_widget_id(root: WidgetId, marker: TimelineMarkerId) -> WidgetId {
    root.child(("timeline-marker", marker.raw()))
}

/// Derives a stable semantic widget ID for a timeline keyframe.
#[must_use]
pub fn timeline_keyframe_widget_id(root: WidgetId, keyframe: TimelineKeyframeId) -> WidgetId {
    root.child(("timeline-keyframe", keyframe.raw()))
}
