use std::time::Instant;

use kinetik_ui_core::RepaintRequest;

/// Pure Winit repaint schedule resolved from one frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WinitRepaintSchedule {
    /// Wait for external input.
    #[default]
    Idle,
    /// Request one redraw immediately.
    Immediate,
    /// Wait until the exact deadline.
    At(Instant),
    /// Continuously request redraws until a later frame replaces the schedule.
    Continuous,
}

/// Stateful one-frame replacement policy for Winit redraw scheduling.
#[derive(Debug, Default)]
pub struct WinitRepaintScheduler {
    schedule: WinitRepaintSchedule,
}

impl WinitRepaintScheduler {
    /// Creates an idle scheduler.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schedule: WinitRepaintSchedule::Idle,
        }
    }

    /// Returns the current schedule.
    #[must_use]
    pub const fn schedule(&self) -> WinitRepaintSchedule {
        self.schedule
    }

    /// Replaces any obsolete schedule with the current frame's request.
    ///
    /// Targeted shell input promotes an otherwise idle frame to an immediate
    /// repaint so the response is consumed promptly.
    pub fn replace_frame_request(
        &mut self,
        request: RepaintRequest,
        has_shell_input: bool,
        now: Instant,
    ) -> WinitRepaintSchedule {
        let request = if has_shell_input {
            request.merge(RepaintRequest::NextFrame)
        } else {
            request
        };
        self.schedule = match request {
            RepaintRequest::None => WinitRepaintSchedule::Idle,
            RepaintRequest::NextFrame => WinitRepaintSchedule::Immediate,
            RepaintRequest::After(delay) => now
                .checked_add(delay)
                .map_or(WinitRepaintSchedule::Idle, WinitRepaintSchedule::At),
            RepaintRequest::Continuous => WinitRepaintSchedule::Continuous,
        };
        self.schedule
    }

    /// Replaces the current schedule with one immediate redraw.
    pub fn request_immediate(&mut self) {
        self.schedule = WinitRepaintSchedule::Immediate;
    }

    /// Returns whether a redraw should be requested now.
    ///
    /// Immediate and fired-deadline schedules clear exactly once. Continuous
    /// remains active until the next frame replaces it.
    pub fn take_redraw_request(&mut self, now: Instant) -> bool {
        match self.schedule {
            WinitRepaintSchedule::Immediate => {
                self.schedule = WinitRepaintSchedule::Idle;
                true
            }
            WinitRepaintSchedule::At(deadline) if now >= deadline => {
                self.schedule = WinitRepaintSchedule::Idle;
                true
            }
            WinitRepaintSchedule::Continuous => true,
            WinitRepaintSchedule::Idle | WinitRepaintSchedule::At(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WinitRepaintSchedule, WinitRepaintScheduler};
    use kinetik_ui_core::RepaintRequest;
    use std::time::{Duration, Instant};

    #[test]
    fn frame_requests_replace_obsolete_schedules() {
        let now = Instant::now();
        let mut scheduler = WinitRepaintScheduler::new();

        assert_eq!(
            scheduler.replace_frame_request(
                RepaintRequest::After(Duration::from_secs(2)),
                false,
                now,
            ),
            WinitRepaintSchedule::At(now + Duration::from_secs(2))
        );
        assert_eq!(
            scheduler.replace_frame_request(RepaintRequest::NextFrame, false, now),
            WinitRepaintSchedule::Immediate
        );
        assert_eq!(
            scheduler.replace_frame_request(RepaintRequest::None, false, now),
            WinitRepaintSchedule::Idle
        );
    }

    #[test]
    fn shell_input_promotes_idle_frame_to_immediate() {
        let now = Instant::now();
        let mut scheduler = WinitRepaintScheduler::new();

        assert_eq!(
            scheduler.replace_frame_request(RepaintRequest::None, true, now),
            WinitRepaintSchedule::Immediate
        );
    }

    #[test]
    fn deadline_and_immediate_requests_fire_once() {
        let now = Instant::now();
        let mut scheduler = WinitRepaintScheduler::new();
        scheduler.replace_frame_request(
            RepaintRequest::After(Duration::from_millis(10)),
            false,
            now,
        );

        assert!(!scheduler.take_redraw_request(now + Duration::from_millis(9)));
        assert!(scheduler.take_redraw_request(now + Duration::from_millis(10)));
        assert!(!scheduler.take_redraw_request(now + Duration::from_millis(11)));

        scheduler.request_immediate();
        assert!(scheduler.take_redraw_request(now));
        assert!(!scheduler.take_redraw_request(now));
    }

    #[test]
    fn continuous_remains_until_replaced() {
        let now = Instant::now();
        let mut scheduler = WinitRepaintScheduler::new();
        scheduler.replace_frame_request(RepaintRequest::Continuous, false, now);

        assert!(scheduler.take_redraw_request(now));
        assert!(scheduler.take_redraw_request(now));
        assert_eq!(scheduler.schedule(), WinitRepaintSchedule::Continuous);
        scheduler.replace_frame_request(RepaintRequest::None, false, now);
        assert!(!scheduler.take_redraw_request(now));
    }

    #[test]
    fn unrepresentable_deadline_fails_closed_without_panicking() {
        let now = Instant::now();
        let mut scheduler = WinitRepaintScheduler::new();

        assert_eq!(
            scheduler.replace_frame_request(RepaintRequest::After(Duration::MAX), false, now),
            WinitRepaintSchedule::Idle
        );
    }
}
