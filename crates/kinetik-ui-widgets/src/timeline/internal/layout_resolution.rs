#[allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_timeline_layout(
    layout: TimelineLayout,
    bounds: Rect,
    scale: TimelineScale,
    descriptor: &TimelineDescriptor,
    scroll_offset: f32,
) -> Result<TimelineLayoutResult<'_>, TimelineDescriptorError> {
    descriptor.validate()?;

    let bounds = finite_rect(bounds);
    let window = timeline_lane_window(
        descriptor.lanes.len(),
        layout.row_height,
        bounds.height,
        scroll_offset,
        layout.overscan,
    );
    let row_height = finite_positive(layout.row_height).unwrap_or(0.0);
    let scale = scale.sanitized();
    let lane_indices = descriptor
        .lanes
        .iter()
        .enumerate()
        .map(|(index, lane)| (lane.id, index))
        .collect::<BTreeMap<_, _>>();
    let item_lane_indices = descriptor
        .items
        .iter()
        .filter_map(|item| {
            lane_indices
                .get(&item.lane)
                .map(|lane_index| (item.id, *lane_index))
        })
        .collect::<BTreeMap<_, _>>();
    let projection = TimelineLayoutProjection::new(
        bounds,
        scale,
        window.materialized_range.clone(),
        layout.marker_hit_width,
        layout.keyframe_hit_size,
    );

    let lanes = descriptor
        .lanes
        .iter()
        .enumerate()
        .skip(window.materialized_range.start)
        .take(
            window
                .materialized_range
                .end
                .saturating_sub(window.materialized_range.start),
        )
        .map(|(row_index, lane)| ResolvedTimelineLane {
            descriptor: lane,
            source_index: row_index,
            row_index,
            rect: timeline_lane_rect(bounds, row_height, window.clamped_scroll_offset, row_index),
        })
        .collect::<Vec<_>>();

    let mut items = project_timeline_items(descriptor, &lane_indices, &projection)
        .into_iter()
        .filter_map(|projected| {
            resolve_timeline_item(
                projected.source_index,
                projected.descriptor,
                projected.lane_index,
                bounds,
                row_height,
                window.clamped_scroll_offset,
                scale,
            )
        })
        .collect::<Vec<_>>();
    items.sort_by(compare_resolved_timeline_items);

    let mut markers = project_timeline_markers(descriptor, &projection)
        .into_iter()
        .filter_map(|projected| {
            resolve_timeline_marker(
                projected.source_index,
                projected.descriptor,
                bounds,
                scale,
                layout.marker_hit_width,
            )
        })
        .collect::<Vec<_>>();
    markers.sort_by(compare_resolved_timeline_markers);

    let mut keyframes = project_timeline_keyframes(descriptor, &item_lane_indices, &projection)
        .into_iter()
        .filter_map(|projected| {
            resolve_timeline_keyframe(
                projected.source_index,
                projected.descriptor,
                projected.lane_index,
                bounds,
                row_height,
                window.clamped_scroll_offset,
                scale,
                layout.keyframe_hit_size,
            )
        })
        .collect::<Vec<_>>();
    keyframes.sort_by(compare_resolved_timeline_keyframes);

    Ok(TimelineLayoutResult {
        bounds,
        content_height: window.content_extent,
        max_scroll_offset: window.max_scroll_offset,
        scroll_offset: window.clamped_scroll_offset,
        visible_lane_range: window.visible_range,
        materialized_lane_range: window.materialized_range,
        lanes,
        items,
        markers,
        keyframes,
    })
}

pub(crate) fn resolve_timeline_item(
    source_index: usize,
    item: &TimelineItemDescriptor,
    lane_index: usize,
    bounds: Rect,
    row_height: f32,
    scroll_offset: f32,
    scale: TimelineScale,
) -> Option<ResolvedTimelineItem<'_>> {
    let time_range = item.time_range.sanitized();
    let row = timeline_lane_rect(bounds, row_height, scroll_offset, lane_index);
    let start_x = scale.time_to_screen_x(time_range.start);
    let end_x = scale.time_to_screen_x(time_range.end);
    let left = start_x.min(end_x);
    let right = start_x.max(end_x);
    let unclipped_rect = finite_rect(Rect::new(left, row.y, right - left, row.height));
    let rect = intersect_rect(unclipped_rect, bounds)?;
    let visible_time_range = TimelineRange::new(
        scale.screen_x_to_time(rect.x),
        scale.screen_x_to_time(rect_max_x(rect)),
    )
    .sanitized();

    Some(ResolvedTimelineItem {
        descriptor: item,
        source_index,
        lane_index,
        time_range,
        visible_time_range,
        rect,
        unclipped_rect,
    })
}

pub(crate) fn resolve_timeline_marker(
    source_index: usize,
    marker: &TimelineMarkerDescriptor,
    bounds: Rect,
    scale: TimelineScale,
    hit_width: f32,
) -> Option<ResolvedTimelineMarker<'_>> {
    let time = marker.time.sanitized();
    let x = scale.time_to_screen_x(time);
    let width = finite_positive(hit_width).unwrap_or(1.0);
    let hit_rect = centered_rect(x, bounds.y + bounds.height * 0.5, width, bounds.height);
    let hit_rect = intersect_rect(hit_rect, bounds)?;

    Some(ResolvedTimelineMarker {
        descriptor: marker,
        source_index,
        time,
        x,
        hit_rect,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_timeline_keyframe(
    source_index: usize,
    keyframe: &TimelineKeyframeDescriptor,
    lane_index: usize,
    bounds: Rect,
    row_height: f32,
    scroll_offset: f32,
    scale: TimelineScale,
    hit_size: f32,
) -> Option<ResolvedTimelineKeyframe<'_>> {
    let time = keyframe.time.sanitized();
    let x = scale.time_to_screen_x(time);
    let row = timeline_lane_rect(bounds, row_height, scroll_offset, lane_index);
    let size = finite_positive(hit_size).unwrap_or(1.0);
    let hit_rect = centered_rect(x, row.y + row.height * 0.5, size, size);
    let hit_rect = intersect_rect(hit_rect, bounds)?;

    Some(ResolvedTimelineKeyframe {
        descriptor: keyframe,
        source_index,
        item: keyframe.item,
        lane_index,
        time,
        x,
        hit_rect,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimelineLayoutProjection {
    materialized_lane_range: Range<usize>,
    item_time_window: TimelineRange,
    marker_time_window: TimelineRange,
    keyframe_time_window: TimelineRange,
}

impl TimelineLayoutProjection {
    fn new(
        bounds: Rect,
        scale: TimelineScale,
        materialized_lane_range: Range<usize>,
        marker_hit_width: f32,
        keyframe_hit_size: f32,
    ) -> Self {
        Self {
            materialized_lane_range,
            item_time_window: timeline_screen_time_window(bounds, scale, 0.0),
            marker_time_window: timeline_screen_time_window(
                bounds,
                scale,
                finite_positive(marker_hit_width).unwrap_or(1.0) * 0.5,
            ),
            keyframe_time_window: timeline_screen_time_window(
                bounds,
                scale,
                finite_positive(keyframe_hit_size).unwrap_or(1.0) * 0.5,
            ),
        }
    }

    fn contains_lane(&self, lane_index: usize) -> bool {
        self.materialized_lane_range.contains(&lane_index)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedTimelineItem<'a> {
    source_index: usize,
    descriptor: &'a TimelineItemDescriptor,
    lane_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedTimelineMarker<'a> {
    source_index: usize,
    descriptor: &'a TimelineMarkerDescriptor,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedTimelineKeyframe<'a> {
    source_index: usize,
    descriptor: &'a TimelineKeyframeDescriptor,
    lane_index: usize,
}

pub(crate) fn project_timeline_items<'a>(
    descriptor: &'a TimelineDescriptor,
    lane_indices: &BTreeMap<TimelineLaneId, usize>,
    projection: &TimelineLayoutProjection,
) -> Vec<ProjectedTimelineItem<'a>> {
    descriptor
        .items
        .iter()
        .enumerate()
        .filter_map(|(source_index, item)| {
            let lane_index = *lane_indices.get(&item.lane)?;
            (projection.contains_lane(lane_index)
                && timeline_ranges_overlap(item.time_range, projection.item_time_window))
            .then_some(ProjectedTimelineItem {
                source_index,
                descriptor: item,
                lane_index,
            })
        })
        .collect()
}

pub(crate) fn project_timeline_markers<'a>(
    descriptor: &'a TimelineDescriptor,
    projection: &TimelineLayoutProjection,
) -> Vec<ProjectedTimelineMarker<'a>> {
    descriptor
        .markers
        .iter()
        .enumerate()
        .filter_map(|(source_index, marker)| {
            timeline_time_overlaps(marker.time, projection.marker_time_window).then_some(
                ProjectedTimelineMarker {
                    source_index,
                    descriptor: marker,
                },
            )
        })
        .collect()
}

pub(crate) fn project_timeline_keyframes<'a>(
    descriptor: &'a TimelineDescriptor,
    item_lane_indices: &BTreeMap<TimelineItemId, usize>,
    projection: &TimelineLayoutProjection,
) -> Vec<ProjectedTimelineKeyframe<'a>> {
    descriptor
        .keyframes
        .iter()
        .enumerate()
        .filter_map(|(source_index, keyframe)| {
            let lane_index = *item_lane_indices.get(&keyframe.item)?;
            (projection.contains_lane(lane_index)
                && timeline_time_overlaps(keyframe.time, projection.keyframe_time_window))
            .then_some(ProjectedTimelineKeyframe {
                source_index,
                descriptor: keyframe,
                lane_index,
            })
        })
        .collect()
}

pub(crate) fn timeline_screen_time_window(
    bounds: Rect,
    scale: TimelineScale,
    horizontal_padding: f32,
) -> TimelineRange {
    let bounds = finite_rect(bounds);
    if bounds.width <= 0.0 {
        return TimelineRange::seconds(0.0, 0.0);
    }

    let padding = finite_f32_non_negative(horizontal_padding);
    TimelineRange::new(
        scale.screen_x_to_time(bounds.x - padding),
        scale.screen_x_to_time(rect_max_x(bounds) + padding),
    )
    .sanitized()
}

pub(crate) fn timeline_ranges_overlap(range: TimelineRange, window: TimelineRange) -> bool {
    let range = range.sanitized();
    let window = window.sanitized();
    !range.is_empty()
        && !window.is_empty()
        && range.start.seconds() < window.end.seconds()
        && range.end.seconds() > window.start.seconds()
}

pub(crate) fn timeline_time_overlaps(time: TimelineTime, window: TimelineRange) -> bool {
    let time = time.sanitized().seconds();
    let window = window.sanitized();
    !window.is_empty() && time > window.start.seconds() && time < window.end.seconds()
}

pub(crate) fn compare_resolved_timeline_items(
    left: &ResolvedTimelineItem<'_>,
    right: &ResolvedTimelineItem<'_>,
) -> std::cmp::Ordering {
    left.lane_index
        .cmp(&right.lane_index)
        .then_with(|| left.rect.x.total_cmp(&right.rect.x))
        .then_with(|| left.rect.width.total_cmp(&right.rect.width))
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

pub(crate) fn compare_resolved_timeline_markers(
    left: &ResolvedTimelineMarker<'_>,
    right: &ResolvedTimelineMarker<'_>,
) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}

pub(crate) fn compare_resolved_timeline_keyframes(
    left: &ResolvedTimelineKeyframe<'_>,
    right: &ResolvedTimelineKeyframe<'_>,
) -> std::cmp::Ordering {
    left.lane_index
        .cmp(&right.lane_index)
        .then_with(|| left.x.total_cmp(&right.x))
        .then_with(|| left.descriptor.id.cmp(&right.descriptor.id))
        .then_with(|| left.source_index.cmp(&right.source_index))
}
