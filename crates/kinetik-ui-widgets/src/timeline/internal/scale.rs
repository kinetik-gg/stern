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
