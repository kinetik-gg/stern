#[allow(clippy::wildcard_imports)]
use super::*;

/// Timeline zoom in logical pixels per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineZoom {
    /// Logical pixels per timeline second.
    pub pixels_per_second: f32,
}

impl TimelineZoom {
    /// Creates timeline zoom metadata.
    #[must_use]
    pub const fn new(pixels_per_second: f32) -> Self {
        Self { pixels_per_second }
    }

    /// Returns a deterministic clamped zoom.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            pixels_per_second: sanitize_timeline_zoom(self.pixels_per_second),
        }
    }

    /// Sets zoom with deterministic clamping.
    pub fn set_pixels_per_second(&mut self, pixels_per_second: f32) {
        self.pixels_per_second = sanitize_timeline_zoom(pixels_per_second);
    }
}

impl Default for TimelineZoom {
    fn default() -> Self {
        Self::new(DEFAULT_TIMELINE_PIXELS_PER_SECOND)
    }
}

/// Timeline viewport scale and horizontal scroll state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineScale {
    /// UI logical x coordinate of the viewport/ruler origin.
    pub origin_x: f32,
    /// Viewport width in logical units.
    pub viewport_width: f32,
    /// Timeline content range.
    pub content_range: TimelineRange,
    /// Logical pixels per second.
    pub zoom: TimelineZoom,
    /// Horizontal scroll offset in logical pixels from `content_range.start`.
    pub scroll_offset: f32,
}

impl TimelineScale {
    /// Creates timeline scale state.
    #[must_use]
    pub const fn new(
        origin_x: f32,
        viewport_width: f32,
        content_range: TimelineRange,
        zoom: TimelineZoom,
        scroll_offset: f32,
    ) -> Self {
        Self {
            origin_x,
            viewport_width,
            content_range,
            zoom,
            scroll_offset,
        }
    }

    /// Returns a copy with finite coordinates, clamped zoom, and clamped scroll.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let content_range = self.content_range.sanitized();
        let zoom = self.zoom.sanitized();
        let viewport_width = finite_f32_non_negative(self.viewport_width);
        let max_scroll_offset = max_timeline_scroll_offset(content_range, zoom, viewport_width);
        Self {
            origin_x: finite_f32_or_zero(self.origin_x),
            viewport_width,
            content_range,
            zoom,
            scroll_offset: clamp_timeline_scroll_offset(self.scroll_offset, max_scroll_offset),
        }
    }

    /// Returns the maximum valid scroll offset in logical pixels.
    #[must_use]
    pub fn max_scroll_offset(self) -> f32 {
        let scale = self.sanitized();
        max_timeline_scroll_offset(scale.content_range, scale.zoom, scale.viewport_width)
    }

    /// Returns the visible time range represented by this scale.
    #[must_use]
    pub fn visible_range(self) -> TimelineRange {
        let scale = self.sanitized();
        let seconds_per_pixel = 1.0 / f64::from(scale.zoom.pixels_per_second);
        let start = scale.content_range.start.seconds()
            + f64::from(scale.scroll_offset) * seconds_per_pixel;
        let end = start + f64::from(scale.viewport_width) * seconds_per_pixel;
        TimelineRange::seconds(start, end.min(scale.content_range.end.seconds())).sanitized()
    }

    /// Converts timeline time to UI logical screen x.
    #[must_use]
    pub fn time_to_screen_x(self, time: TimelineTime) -> f32 {
        let scale = self.sanitized();
        let content_seconds = time.sanitized().seconds() - scale.content_range.start.seconds();
        finite_f64_to_f32(
            f64::from(scale.origin_x - scale.scroll_offset)
                + content_seconds * f64::from(scale.zoom.pixels_per_second),
        )
    }

    /// Converts UI logical screen x to timeline time.
    #[must_use]
    pub fn screen_x_to_time(self, x: f32) -> TimelineTime {
        let scale = self.sanitized();
        let content_x = finite_f32_or_zero(x) - scale.origin_x + scale.scroll_offset;
        TimelineTime::from_seconds(
            scale.content_range.start.seconds()
                + f64::from(content_x) / f64::from(scale.zoom.pixels_per_second),
        )
        .sanitized()
    }

    /// Converts a frame position to UI logical screen x.
    #[must_use]
    pub fn frame_to_screen_x(self, frame_rate: TimelineFrameRate, frame: TimelineFrame) -> f32 {
        self.time_to_screen_x(frame_rate.frame_to_time(frame))
    }

    /// Converts UI logical screen x to a frame position.
    #[must_use]
    pub fn screen_x_to_frame(
        self,
        frame_rate: TimelineFrameRate,
        x: f32,
        rounding: TimelineFrameRounding,
    ) -> TimelineFrame {
        frame_rate.time_to_frame(self.screen_x_to_time(x), rounding)
    }

    /// Returns a new scale whose zoom changes while preserving the timeline time under `anchor_x`.
    #[must_use]
    pub fn zoom_around_anchor(self, anchor_x: f32, zoom: TimelineZoom) -> TimelineZoomAnchorResult {
        zoom_timeline_scale_around_anchor(self, anchor_x, zoom)
    }
}

/// Result of changing timeline zoom around a pointer or viewport anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineZoomAnchorResult {
    /// Scale after zoom and scroll clamping.
    pub scale: TimelineScale,
    /// Timeline time that was under the anchor before zooming.
    pub anchor_time: TimelineTime,
    /// Sanitized screen-space anchor.
    pub anchor_x: f32,
}
