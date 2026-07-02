#[allow(clippy::wildcard_imports)]
use super::*;

/// Timeline time in seconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct TimelineTime {
    seconds: f64,
}

impl TimelineTime {
    /// The timeline origin.
    pub const ZERO: Self = Self::from_seconds(0.0);

    /// Creates timeline time from seconds.
    #[must_use]
    pub const fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }

    /// Returns raw seconds.
    #[must_use]
    pub const fn seconds(self) -> f64 {
        self.seconds
    }

    /// Returns a copy with non-finite seconds replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::from_seconds(finite_f64_or_zero(self.seconds))
    }
}

/// Integer frame position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimelineFrame(i64);

impl TimelineFrame {
    /// Creates a frame position from raw frame bits.
    #[must_use]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Returns the raw frame index.
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

/// Frame rounding policy for converting continuous time to integer frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineFrameRounding {
    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round to the nearest frame, with half values away from zero.
    Nearest,
    /// Round toward zero.
    Truncate,
}

/// Rational frame-rate metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineFrameRate {
    /// Frames-per-second numerator.
    pub numerator: u32,
    /// Frames-per-second denominator.
    pub denominator: u32,
}

impl TimelineFrameRate {
    /// Creates rational frame-rate metadata.
    #[must_use]
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Creates an integer frame rate.
    #[must_use]
    pub const fn integer(frames_per_second: u32) -> Self {
        Self::new(frames_per_second, 1)
    }

    /// Returns deterministic non-zero frame-rate metadata.
    #[must_use]
    pub const fn sanitized(self) -> Self {
        Self {
            numerator: if self.numerator == 0 {
                24
            } else {
                self.numerator
            },
            denominator: if self.denominator == 0 {
                1
            } else {
                self.denominator
            },
        }
    }

    /// Returns frames per second as a finite number.
    #[must_use]
    pub fn frames_per_second(self) -> f64 {
        let rate = self.sanitized();
        f64::from(rate.numerator) / f64::from(rate.denominator)
    }

    /// Returns seconds per frame.
    #[must_use]
    pub fn seconds_per_frame(self) -> f64 {
        1.0 / self.frames_per_second()
    }

    /// Converts a frame position to timeline time.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn frame_to_time(self, frame: TimelineFrame) -> TimelineTime {
        TimelineTime::from_seconds(frame.raw() as f64 * self.seconds_per_frame()).sanitized()
    }

    /// Converts timeline time to an integer frame with the requested rounding policy.
    #[must_use]
    pub fn time_to_frame(
        self,
        time: TimelineTime,
        rounding: TimelineFrameRounding,
    ) -> TimelineFrame {
        let frame = time.sanitized().seconds() * self.frames_per_second();
        TimelineFrame::from_raw(round_frame(frame, rounding))
    }

    pub(crate) fn rounded_display_fps(self) -> i64 {
        let fps = self.frames_per_second().round();
        f64_to_i64_saturating(fps).max(1)
    }
}

impl Default for TimelineFrameRate {
    fn default() -> Self {
        Self::integer(24)
    }
}

/// Finite normalized timeline time range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineRange {
    /// Start time.
    pub start: TimelineTime,
    /// End time.
    pub end: TimelineTime,
}

impl TimelineRange {
    /// Creates a timeline range.
    #[must_use]
    pub const fn new(start: TimelineTime, end: TimelineTime) -> Self {
        Self { start, end }
    }

    /// Creates a timeline range from seconds.
    #[must_use]
    pub const fn seconds(start: f64, end: f64) -> Self {
        Self::new(
            TimelineTime::from_seconds(start),
            TimelineTime::from_seconds(end),
        )
    }

    /// Returns a finite range with ascending endpoints.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let start = self.start.sanitized().seconds();
        let end = self.end.sanitized().seconds();
        Self::seconds(start.min(end), start.max(end))
    }

    /// Returns range duration in seconds.
    #[must_use]
    pub fn duration_seconds(self) -> f64 {
        let range = self.sanitized();
        (range.end.seconds() - range.start.seconds()).max(0.0)
    }

    /// Returns true when the range has no positive duration.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.duration_seconds() <= 0.0
    }

    /// Computes content width in logical pixels at the supplied zoom.
    #[must_use]
    pub fn content_width(self, zoom: TimelineZoom) -> f32 {
        finite_f64_to_f32(self.duration_seconds() * f64::from(zoom.sanitized().pixels_per_second))
    }
}
