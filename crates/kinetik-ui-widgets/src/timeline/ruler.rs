#[allow(clippy::wildcard_imports)]
use super::*;

/// Ruler tick role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineRulerTickKind {
    /// Labeled primary tick.
    Major,
    /// Unlabeled subdivision tick.
    Minor,
}

/// Stable ruler tick metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineRulerTick {
    /// Tick kind.
    pub kind: TimelineRulerTickKind,
    /// Tick frame.
    pub frame: TimelineFrame,
    /// Deterministic label. Minor ticks use an empty label.
    pub label: String,
}

impl TimelineRulerTick {
    /// Returns tick time for a frame rate.
    #[must_use]
    pub fn time(&self, frame_rate: TimelineFrameRate) -> TimelineTime {
        frame_rate.frame_to_time(self.frame)
    }
}

/// Ruler tick generation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRulerTickRequest {
    /// Visible time range.
    pub visible_range: TimelineRange,
    /// Frame-rate metadata.
    pub frame_rate: TimelineFrameRate,
    /// Timeline zoom.
    pub zoom: TimelineZoom,
    /// Upper bound for emitted ticks.
    pub max_ticks: usize,
}

impl TimelineRulerTickRequest {
    /// Creates a ruler tick request.
    #[must_use]
    pub const fn new(
        visible_range: TimelineRange,
        frame_rate: TimelineFrameRate,
        zoom: TimelineZoom,
    ) -> Self {
        Self {
            visible_range,
            frame_rate,
            zoom,
            max_ticks: DEFAULT_TIMELINE_RULER_MAX_TICKS,
        }
    }

    /// Sets a maximum tick count.
    #[must_use]
    pub const fn with_max_ticks(mut self, max_ticks: usize) -> Self {
        self.max_ticks = max_ticks;
        self
    }

    /// Emits deterministic finite ruler ticks.
    #[must_use]
    pub fn ticks(self) -> Vec<TimelineRulerTick> {
        timeline_ruler_ticks(self)
    }
}

/// Computes maximum horizontal scroll offset in logical pixels.
#[must_use]
pub fn max_timeline_scroll_offset(
    content_range: TimelineRange,
    zoom: TimelineZoom,
    viewport_width: f32,
) -> f32 {
    (content_range.content_width(zoom) - finite_f32_non_negative(viewport_width)).max(0.0)
}

/// Clamps a scroll offset between zero and the supplied maximum offset.
#[must_use]
pub fn clamp_timeline_scroll_offset(scroll_offset: f32, max_scroll_offset: f32) -> f32 {
    finite_f32_non_negative(scroll_offset).min(finite_f32_non_negative(max_scroll_offset))
}

/// Clamps a pixels-per-second zoom value.
#[must_use]
pub fn sanitize_timeline_zoom(pixels_per_second: f32) -> f32 {
    if pixels_per_second.is_finite() && pixels_per_second > 0.0 {
        pixels_per_second.clamp(
            MIN_TIMELINE_PIXELS_PER_SECOND,
            MAX_TIMELINE_PIXELS_PER_SECOND,
        )
    } else {
        DEFAULT_TIMELINE_PIXELS_PER_SECOND
    }
}

/// Emits deterministic finite ruler ticks for the requested visible range.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn timeline_ruler_ticks(request: TimelineRulerTickRequest) -> Vec<TimelineRulerTick> {
    let visible = request.visible_range.sanitized();
    if visible.is_empty() || request.max_ticks == 0 {
        return Vec::new();
    }

    let frame_rate = request.frame_rate.sanitized();
    let zoom = request.zoom.sanitized();
    let min_major_frames = ((80.0 / f64::from(zoom.pixels_per_second))
        * frame_rate.frames_per_second())
    .ceil()
    .max(1.0);
    let mut major_step = nice_frame_step(f64_to_i64_saturating(min_major_frames).max(1));
    let mut minor_step = (major_step / 5).max(1);

    let start_frame = frame_rate
        .time_to_frame(visible.start, TimelineFrameRounding::Floor)
        .raw();
    let end_frame = frame_rate
        .time_to_frame(visible.end, TimelineFrameRounding::Ceil)
        .raw();

    while tick_count(start_frame, end_frame, minor_step) > request.max_ticks {
        minor_step = major_step;
        if tick_count(start_frame, end_frame, minor_step) > request.max_ticks {
            let next_major_step = nice_frame_step(major_step.saturating_mul(2));
            if next_major_step <= major_step {
                break;
            }
            major_step = next_major_step;
            minor_step = major_step;
        }
    }

    let first = floor_to_step(start_frame, minor_step);
    let last = ceil_to_step(end_frame, minor_step);
    let mut ticks = Vec::new();
    let mut frame = first;
    while frame <= last && ticks.len() < request.max_ticks {
        let kind = if frame.rem_euclid(major_step) == 0 {
            TimelineRulerTickKind::Major
        } else {
            TimelineRulerTickKind::Minor
        };
        ticks.push(TimelineRulerTick {
            kind,
            frame: TimelineFrame::from_raw(frame),
            label: if kind == TimelineRulerTickKind::Major {
                timeline_timecode_label(TimelineFrame::from_raw(frame), frame_rate)
            } else {
                String::new()
            },
        });
        if minor_step <= 0 {
            break;
        }
        let Some(next_frame) = frame.checked_add(minor_step) else {
            break;
        };
        if next_frame <= frame {
            break;
        }
        frame = next_frame;
    }
    ticks
}

/// Returns a deterministic timecode-style label for a frame.
#[must_use]
pub fn timeline_timecode_label(frame: TimelineFrame, frame_rate: TimelineFrameRate) -> String {
    let display_fps = frame_rate.rounded_display_fps();
    let raw = frame.raw();
    let sign = if raw < 0 { "-" } else { "" };
    let frames = raw.saturating_abs();
    let frames_per_hour = display_fps.saturating_mul(3600);
    let frames_per_minute = display_fps.saturating_mul(60);
    let hours = frames / frames_per_hour;
    let minutes = (frames % frames_per_hour) / frames_per_minute;
    let seconds = (frames % frames_per_minute) / display_fps;
    let frame = frames % display_fps;

    format!("{sign}{hours:02}:{minutes:02}:{seconds:02}:{frame:02}")
}

/// Resolves one timeline time against snap candidates without mutating descriptors.
#[must_use]
pub fn timeline_snap_time(
    requested_time: TimelineTime,
    candidates: &[TimelineSnapCandidate],
    tolerance_seconds: f64,
) -> TimelineSnapMetadata {
    let requested_time = requested_time.sanitized();
    let tolerance_seconds = finite_f64_or_zero(tolerance_seconds).max(0.0);
    let mut best: Option<(f64, TimelineSnapCandidate)> = None;

    for candidate in candidates {
        let candidate = TimelineSnapCandidate {
            time: candidate.time.sanitized(),
            source: candidate.source,
            target: candidate.target,
        };
        let distance = (candidate.time.seconds() - requested_time.seconds()).abs();
        if distance > tolerance_seconds {
            continue;
        }

        let replace = best.is_none_or(|(best_distance, best_candidate)| {
            let distance_order = distance.total_cmp(&best_distance);
            distance_order.is_lt()
                || (distance_order.is_eq()
                    && compare_snap_candidates(candidate, best_candidate).is_lt())
        });
        if replace {
            best = Some((distance, candidate));
        }
    }

    best.map_or_else(
        || TimelineSnapMetadata::unsnapped(requested_time),
        |(_, candidate)| {
            TimelineSnapMetadata::snapped(
                requested_time,
                candidate.time,
                candidate.source,
                candidate.target,
            )
        },
    )
}

/// Collects deterministic snap candidates without mutating descriptors or app state.
#[must_use]
pub fn timeline_snap_candidates(
    request: TimelineSnapCandidateRequest<'_>,
) -> Vec<TimelineSnapCandidate> {
    let mut candidates = Vec::new();
    append_frame_snap_candidates(&mut candidates, request);

    if let Some(playhead_time) = request.playhead_time {
        candidates.push(TimelineSnapCandidate::new(
            playhead_time.sanitized(),
            TimelineSnapSource::Playhead,
            Some(TimelineHitTarget::Playhead(request.timeline)),
        ));
    }

    if let Some(range) = request.selection_range {
        let range = range.sanitized();
        candidates.push(TimelineSnapCandidate::new(
            range.start,
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeStartHandle(request.timeline)),
        ));
        candidates.push(TimelineSnapCandidate::new(
            range.end,
            TimelineSnapSource::RangeBoundary,
            Some(TimelineHitTarget::RangeEndHandle(request.timeline)),
        ));
    }

    for item in &request.descriptor.items {
        let range = item.time_range.sanitized();
        candidates.push(TimelineSnapCandidate::new(
            range.start,
            TimelineSnapSource::ItemBoundary,
            Some(TimelineHitTarget::ItemTrimStartHandle(item.id)),
        ));
        candidates.push(TimelineSnapCandidate::new(
            range.end,
            TimelineSnapSource::ItemBoundary,
            Some(TimelineHitTarget::ItemTrimEndHandle(item.id)),
        ));
    }

    for marker in &request.descriptor.markers {
        candidates.push(TimelineSnapCandidate::new(
            marker.time.sanitized(),
            TimelineSnapSource::Marker,
            Some(TimelineHitTarget::Marker(marker.id)),
        ));
    }

    for keyframe in &request.descriptor.keyframes {
        candidates.push(TimelineSnapCandidate::new(
            keyframe.time.sanitized(),
            TimelineSnapSource::Keyframe,
            Some(TimelineHitTarget::Keyframe(keyframe.id)),
        ));
    }

    candidates.sort_by(|left, right| {
        left.time
            .seconds()
            .total_cmp(&right.time.seconds())
            .then_with(|| compare_snap_candidates(*left, *right))
    });
    candidates
}
