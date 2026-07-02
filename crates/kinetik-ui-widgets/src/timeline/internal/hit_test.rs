#[allow(clippy::wildcard_imports)]
use super::*;

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
