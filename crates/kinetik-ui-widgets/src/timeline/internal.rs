#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn zoom_timeline_scale_around_anchor(
    scale: TimelineScale,
    anchor_x: f32,
    zoom: TimelineZoom,
) -> TimelineZoomAnchorResult {
    let scale = scale.sanitized();
    let anchor_x = finite_f32_or_zero(anchor_x);
    let anchor_time = scale.screen_x_to_time(anchor_x);
    let zoom = zoom.sanitized();
    let content_seconds = anchor_time.seconds() - scale.content_range.start.seconds();
    let anchor_local_x = f64::from(anchor_x - scale.origin_x);
    let requested_scroll_offset =
        finite_f64_to_f32(content_seconds * f64::from(zoom.pixels_per_second) - anchor_local_x);
    let max_scroll_offset =
        max_timeline_scroll_offset(scale.content_range, zoom, scale.viewport_width);
    let scale = TimelineScale {
        zoom,
        scroll_offset: clamp_timeline_scroll_offset(requested_scroll_offset, max_scroll_offset),
        ..scale
    }
    .sanitized();

    TimelineZoomAnchorResult {
        scale,
        anchor_time,
        anchor_x,
    }
}

pub(crate) fn sanitize_timeline_snap_metadata(snap: TimelineSnapMetadata) -> TimelineSnapMetadata {
    if snap.source == TimelineSnapSource::None {
        TimelineSnapMetadata::unsnapped(snap.requested_time.sanitized())
    } else {
        TimelineSnapMetadata::snapped(
            snap.requested_time.sanitized(),
            snap.snapped_time.sanitized(),
            snap.source,
            snap.target,
        )
    }
}

pub(crate) fn append_frame_snap_candidates(
    candidates: &mut Vec<TimelineSnapCandidate>,
    request: TimelineSnapCandidateRequest<'_>,
) {
    let range = request.range.sanitized();
    if range.is_empty() || request.max_frame_candidates == 0 {
        return;
    }

    let frame_rate = request.frame_rate.sanitized();
    let start = frame_rate
        .time_to_frame(range.start, TimelineFrameRounding::Ceil)
        .raw();
    let end = frame_rate
        .time_to_frame(range.end, TimelineFrameRounding::Floor)
        .raw();
    if end < start {
        return;
    }

    let mut frame = start;
    let mut emitted = 0_usize;
    while frame <= end && emitted < request.max_frame_candidates {
        candidates.push(TimelineSnapCandidate::new(
            frame_rate.frame_to_time(TimelineFrame::from_raw(frame)),
            TimelineSnapSource::Frame,
            None,
        ));
        emitted = emitted.saturating_add(1);
        let Some(next) = frame.checked_add(1) else {
            break;
        };
        frame = next;
    }
}

pub(crate) fn hit_test_timeline(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let bounds = finite_rect(result.bounds);
    let point = sanitize_point(point);
    if !bounds.contains_point(point) {
        return None;
    }

    let config = config.sanitized();
    let time = config.scale.screen_x_to_time(point.x);

    if let Some(hit) = hit_test_timeline_keyframes(result, point, time) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_items(result, point, time, config.item_trim_handle_width) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_markers(result, point, time) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_playhead(bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_range_handles(bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_lane_headers(result, bounds, point, time, config) {
        return Some(hit);
    }

    if let Some(hit) = hit_test_timeline_ruler(point, time, config) {
        return Some(hit);
    }

    Some(TimelineHitMetadata {
        target: TimelineHitTarget::Background(config.timeline),
        rect: bounds,
        time,
        state: TimelineDescriptorState::default(),
    })
}

pub(crate) fn hit_test_timeline_playhead(
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let playhead_time = config.playhead_time?;
    let x = config.scale.time_to_screen_x(playhead_time);
    let rect = centered_rect(
        x,
        bounds.y + bounds.height * 0.5,
        config.playhead_hit_width,
        bounds.height,
    );
    rect.contains_point(point).then_some(TimelineHitMetadata {
        target: TimelineHitTarget::Playhead(config.timeline),
        rect,
        time,
        state: TimelineDescriptorState::default(),
    })
}

pub(crate) fn hit_test_timeline_range_handles(
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    let range = config.selection_range?;
    let start_x = config.scale.time_to_screen_x(range.start);
    let end_x = config.scale.time_to_screen_x(range.end);
    [
        (
            TimelineHitTarget::RangeStartHandle(config.timeline),
            start_x,
        ),
        (TimelineHitTarget::RangeEndHandle(config.timeline), end_x),
    ]
    .into_iter()
    .find_map(|(target, x)| {
        let rect = centered_rect(
            x,
            bounds.y + bounds.height * 0.5,
            config.range_handle_hit_width,
            bounds.height,
        );
        rect.contains_point(point).then_some(TimelineHitMetadata {
            target,
            rect,
            time,
            state: TimelineDescriptorState::default(),
        })
    })
}

pub(crate) fn hit_test_timeline_lane_headers(
    result: &TimelineLayoutResult<'_>,
    bounds: Rect,
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    if config.lane_header_width <= 0.0 {
        return None;
    }

    let header_rect = Rect::new(
        bounds.x,
        bounds.y,
        config.lane_header_width.min(bounds.width),
        bounds.height,
    );
    if !header_rect.contains_point(point) {
        return None;
    }

    result
        .lanes
        .iter()
        .find(|lane| lane.rect.contains_point(point))
        .map(|lane| TimelineHitMetadata {
            target: TimelineHitTarget::LaneHeader(lane.descriptor.id),
            rect: lane.rect,
            time,
            state: lane.descriptor.state,
        })
}

pub(crate) fn hit_test_timeline_ruler(
    point: Point,
    time: TimelineTime,
    config: TimelineHitTestConfig,
) -> Option<TimelineHitMetadata> {
    config
        .ruler_rect
        .filter(|ruler_rect| ruler_rect.contains_point(point))
        .map(|ruler_rect| TimelineHitMetadata {
            target: TimelineHitTarget::Ruler(config.ruler),
            rect: ruler_rect,
            time,
            state: TimelineDescriptorState::default(),
        })
}

pub(crate) fn hit_test_timeline_keyframes(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
) -> Option<TimelineHitMetadata> {
    result
        .keyframes
        .iter()
        .rev()
        .find(|keyframe| keyframe.hit_rect.contains_point(point))
        .map(|keyframe| TimelineHitMetadata {
            target: TimelineHitTarget::Keyframe(keyframe.descriptor.id),
            rect: keyframe.hit_rect,
            time,
            state: keyframe.descriptor.state,
        })
}

pub(crate) fn hit_test_timeline_items(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
    trim_handle_width: f32,
) -> Option<TimelineHitMetadata> {
    let trim_handle_width = finite_f32_non_negative(trim_handle_width);
    result.items.iter().rev().find_map(|item| {
        if !item.rect.contains_point(point) {
            return None;
        }

        let start_rect = item_start_trim_rect(item.rect, trim_handle_width);
        if start_rect.contains_point(point) {
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::ItemTrimStartHandle(item.descriptor.id),
                rect: start_rect,
                time,
                state: item.descriptor.state,
            });
        }

        let end_rect = item_end_trim_rect(item.rect, trim_handle_width);
        if end_rect.contains_point(point) {
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::ItemTrimEndHandle(item.descriptor.id),
                rect: end_rect,
                time,
                state: item.descriptor.state,
            });
        }

        Some(TimelineHitMetadata {
            target: TimelineHitTarget::Item(item.descriptor.id),
            rect: item.rect,
            time,
            state: item.descriptor.state,
        })
    })
}

pub(crate) fn hit_test_timeline_markers(
    result: &TimelineLayoutResult<'_>,
    point: Point,
    time: TimelineTime,
) -> Option<TimelineHitMetadata> {
    result
        .markers
        .iter()
        .rev()
        .find(|marker| marker.hit_rect.contains_point(point))
        .map(|marker| TimelineHitMetadata {
            target: TimelineHitTarget::Marker(marker.descriptor.id),
            rect: marker.hit_rect,
            time,
            state: marker.descriptor.state,
        })
}

pub(crate) fn compare_snap_candidates(
    left: TimelineSnapCandidate,
    right: TimelineSnapCandidate,
) -> std::cmp::Ordering {
    snap_source_rank(left.source)
        .cmp(&snap_source_rank(right.source))
        .then_with(|| left.time.seconds().total_cmp(&right.time.seconds()))
        .then_with(|| left.target.cmp(&right.target))
}

pub(crate) fn snap_source_rank(source: TimelineSnapSource) -> u8 {
    match source {
        TimelineSnapSource::Frame => 0,
        TimelineSnapSource::Playhead => 1,
        TimelineSnapSource::RangeBoundary => 2,
        TimelineSnapSource::ItemBoundary => 3,
        TimelineSnapSource::Marker => 4,
        TimelineSnapSource::Keyframe => 5,
        TimelineSnapSource::None => 6,
    }
}

pub(crate) fn clamped_timeline_drag_range(
    anchor_time: TimelineTime,
    current_time: TimelineTime,
    bounds: TimelineRange,
) -> TimelineRange {
    TimelineRange::new(
        clamp_timeline_time(anchor_time, bounds),
        clamp_timeline_time(current_time, bounds),
    )
    .sanitized()
}

pub(crate) fn clamp_timeline_time(time: TimelineTime, bounds: TimelineRange) -> TimelineTime {
    let bounds = bounds.sanitized();
    TimelineTime::from_seconds(
        time.sanitized()
            .seconds()
            .clamp(bounds.start.seconds(), bounds.end.seconds()),
    )
}

pub(crate) fn offset_timeline_range(range: TimelineRange, delta: TimelineTime) -> TimelineRange {
    let range = range.sanitized();
    let delta = delta.sanitized().seconds();
    TimelineRange::seconds(range.start.seconds() + delta, range.end.seconds() + delta)
}

pub(crate) fn item_start_trim_rect(rect: Rect, trim_handle_width: f32) -> Rect {
    let width = trim_handle_width.min(rect.width).max(0.0);
    Rect::new(rect.x, rect.y, width, rect.height)
}

pub(crate) fn item_end_trim_rect(rect: Rect, trim_handle_width: f32) -> Rect {
    let width = trim_handle_width.min(rect.width).max(0.0);
    Rect::new(rect_max_x(rect) - width, rect.y, width, rect.height)
}

pub(crate) fn validate_timeline_descriptor(
    descriptor: &TimelineDescriptor,
) -> Result<(), TimelineDescriptorError> {
    let mut lane_ids = BTreeSet::new();
    for lane in &descriptor.lanes {
        if !lane_ids.insert(lane.id) {
            return Err(TimelineDescriptorError::DuplicateLaneId { id: lane.id });
        }
    }

    let mut item_ids = BTreeSet::new();
    for item in &descriptor.items {
        if !item_ids.insert(item.id) {
            return Err(TimelineDescriptorError::DuplicateItemId { id: item.id });
        }
    }

    let mut marker_ids = BTreeSet::new();
    for marker in &descriptor.markers {
        if !marker_ids.insert(marker.id) {
            return Err(TimelineDescriptorError::DuplicateMarkerId { id: marker.id });
        }
    }

    let mut keyframe_ids = BTreeSet::new();
    for keyframe in &descriptor.keyframes {
        if !keyframe_ids.insert(keyframe.id) {
            return Err(TimelineDescriptorError::DuplicateKeyframeId { id: keyframe.id });
        }
    }

    for item in &descriptor.items {
        if !lane_ids.contains(&item.lane) {
            return Err(TimelineDescriptorError::UnknownItemLane {
                item: item.id,
                lane: item.lane,
            });
        }
    }

    for keyframe in &descriptor.keyframes {
        if !item_ids.contains(&keyframe.item) {
            return Err(TimelineDescriptorError::UnknownKeyframeItem {
                keyframe: keyframe.id,
                item: keyframe.item,
            });
        }
    }

    Ok(())
}

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

pub(crate) fn apply_timeline_semantic_state(
    node: &mut SemanticNode,
    state: TimelineDescriptorState,
) {
    node.state.disabled = state.disabled;
    node.state.selected = state.selected;
    if state.read_only {
        node.description = Some("Read-only".to_owned());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimelineLaneWindow {
    content_extent: f32,
    max_scroll_offset: f32,
    clamped_scroll_offset: f32,
    visible_range: Range<usize>,
    materialized_range: Range<usize>,
}

impl TimelineLaneWindow {
    fn empty() -> Self {
        Self {
            content_extent: 0.0,
            max_scroll_offset: 0.0,
            clamped_scroll_offset: 0.0,
            visible_range: 0..0,
            materialized_range: 0..0,
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn timeline_lane_window(
    lane_count: usize,
    row_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
    overscan: usize,
) -> TimelineLaneWindow {
    let Some(row_height) = finite_positive(row_height) else {
        return TimelineLaneWindow::empty();
    };
    let Some(viewport_height) = finite_positive(viewport_height) else {
        return TimelineLaneWindow::empty();
    };
    if lane_count == 0 {
        return TimelineLaneWindow::empty();
    }

    let content_extent = finite_product_usize(lane_count, row_height);
    let max_scroll_offset = (content_extent - viewport_height).max(0.0);
    let clamped_scroll_offset = finite_f32_non_negative(scroll_offset).min(max_scroll_offset);
    let first = ((clamped_scroll_offset / row_height).floor() as usize).min(lane_count);
    let visible_end = ((clamped_scroll_offset + viewport_height) / row_height).ceil() as usize;
    let visible_end = visible_end.min(lane_count).max(first);
    let visible_range = first..visible_end;
    let materialized_visible = ((viewport_height / row_height).ceil() as usize)
        .saturating_add(1)
        .min(lane_count);
    let materialized_start = first.saturating_sub(overscan);
    let materialized_end = first
        .saturating_add(materialized_visible)
        .saturating_add(overscan)
        .min(lane_count);

    TimelineLaneWindow {
        content_extent,
        max_scroll_offset,
        clamped_scroll_offset,
        visible_range,
        materialized_range: materialized_start..materialized_end,
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn finite_product_usize(count: usize, extent: f32) -> f32 {
    if extent.is_finite() {
        (count as f32 * extent).max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn timeline_lane_rect(
    bounds: Rect,
    row_height: f32,
    scroll_offset: f32,
    row_index: usize,
) -> Rect {
    Rect::new(
        bounds.x,
        finite_sum(
            bounds.y,
            finite_sum(row_index_to_offset(row_index, row_height), -scroll_offset),
        ),
        bounds.width,
        row_height,
    )
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn row_index_to_offset(row_index: usize, row_height: f32) -> f32 {
    row_index as f32 * row_height
}

pub(crate) fn intersect_rect(rect: Rect, bounds: Rect) -> Option<Rect> {
    let rect = finite_rect(rect);
    let bounds = finite_rect(bounds);
    let left = rect.x.max(bounds.x);
    let top = rect.y.max(bounds.y);
    let right = rect_max_x(rect).min(rect_max_x(bounds));
    let bottom = rect_max_y(rect).min(rect_max_y(bounds));
    (right > left && bottom > top).then(|| Rect::new(left, top, right - left, bottom - top))
}

pub(crate) fn centered_rect(center_x: f32, center_y: f32, width: f32, height: f32) -> Rect {
    let width = finite_f32_non_negative(width);
    let height = finite_f32_non_negative(height);
    Rect::new(
        center_x - width * 0.5,
        center_y - height * 0.5,
        width,
        height,
    )
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

pub(crate) fn finite_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_f32_or_zero(rect.x),
        finite_f32_or_zero(rect.y),
        finite_f32_non_negative(rect.width),
        finite_f32_non_negative(rect.height),
    )
}

pub(crate) fn sanitize_point(point: Point) -> Point {
    Point::new(finite_f32_or_zero(point.x), finite_f32_or_zero(point.y))
}

pub(crate) fn finite_positive(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0).then_some(value)
}

pub(crate) fn finite_sum(a: f32, b: f32) -> f32 {
    let sum = f64::from(finite_f32_or_zero(a)) + f64::from(finite_f32_or_zero(b));
    finite_f64_to_f32(sum)
}

pub(crate) fn rect_max_x(rect: Rect) -> f32 {
    finite_sum(rect.x, rect.width)
}

pub(crate) fn rect_max_y(rect: Rect) -> f32 {
    finite_sum(rect.y, rect.height)
}

pub(crate) fn finite_f32_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn finite_f32_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn finite_f64_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn finite_f64_to_f32(value: f64) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

pub(crate) fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn f64_to_i64_saturating(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    value.clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

pub(crate) fn round_frame(value: f64, rounding: TimelineFrameRounding) -> i64 {
    let rounded = match rounding {
        TimelineFrameRounding::Floor => value.floor(),
        TimelineFrameRounding::Ceil => value.ceil(),
        TimelineFrameRounding::Nearest => value.round(),
        TimelineFrameRounding::Truncate => value.trunc(),
    };
    f64_to_i64_saturating(rounded)
}

pub(crate) fn nice_frame_step(min_frames: i64) -> i64 {
    let min_frames = min_frames.max(1);
    let mut magnitude = 1_i64;
    while magnitude.saturating_mul(10) < min_frames {
        magnitude = magnitude.saturating_mul(10);
    }

    for multiplier in [1_i64, 2, 5, 10] {
        let step = magnitude.saturating_mul(multiplier);
        if step >= min_frames {
            return step.max(1);
        }
    }
    magnitude.saturating_mul(10).max(1)
}

pub(crate) fn floor_to_step(value: i64, step: i64) -> i64 {
    value.div_euclid(step.max(1)).saturating_mul(step.max(1))
}

pub(crate) fn ceil_to_step(value: i64, step: i64) -> i64 {
    let step = step.max(1);
    let floor = floor_to_step(value, step);
    if floor == value {
        floor
    } else {
        floor.saturating_add(step)
    }
}

pub(crate) fn tick_count(start_frame: i64, end_frame: i64, step: i64) -> usize {
    let step = step.max(1);
    let first = floor_to_step(start_frame, step);
    let last = ceil_to_step(end_frame, step);
    if last < first {
        0
    } else {
        let span = i128::from(last) - i128::from(first);
        let count = span / i128::from(step) + 1;
        usize::try_from(count).unwrap_or(usize::MAX)
    }
}
