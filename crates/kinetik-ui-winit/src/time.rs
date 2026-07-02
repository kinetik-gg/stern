use std::time::Duration;

use kinetik_ui_core::TimeInfo;
/// Deterministic frame clock helper for winit application shells.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WinitFrameClock {
    previous: Option<Duration>,
    frame_index: u64,
}

impl WinitFrameClock {
    /// Creates an empty frame clock.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            previous: None,
            frame_index: 0,
        }
    }

    /// Advances the clock and returns frame time information.
    ///
    /// The first frame reports a zero delta. If a later timestamp moves
    /// backwards, the delta is clamped to zero instead of underflowing.
    pub fn tick(&mut self, now: Duration) -> TimeInfo {
        let delta = self
            .previous
            .map_or(Duration::ZERO, |previous| now.saturating_sub(previous));
        let time = TimeInfo::new(now, delta, self.frame_index);
        self.previous = Some(now);
        self.frame_index = self.frame_index.saturating_add(1);
        time
    }

    /// Clears the previous timestamp and restarts frame numbering.
    pub fn reset(&mut self) {
        self.previous = None;
        self.frame_index = 0;
    }
}
