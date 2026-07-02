#[allow(clippy::wildcard_imports)]
use super::*;

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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimelineLaneWindow {
    pub(crate) content_extent: f32,
    pub(crate) max_scroll_offset: f32,
    pub(crate) clamped_scroll_offset: f32,
    pub(crate) visible_range: Range<usize>,
    pub(crate) materialized_range: Range<usize>,
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
