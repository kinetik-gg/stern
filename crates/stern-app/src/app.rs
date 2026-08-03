use stern_core::ActionQueue;
use stern_render::RenderResources;
use stern_vello_winit::{VelloPresentStatus, VelloRecoveryKind};
use stern_widgets::Ui;

use crate::shell::ShellCtx;

/// What forced an automatic GPU recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryTrigger {
    /// A window resize reported that surface or device recovery is required.
    Resize(VelloRecoveryKind),
    /// A presentation attempt reported that recovery is required.
    Present(VelloPresentStatus),
}

/// One automatic GPU recovery attempt surfaced to [`App::on_recovery`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryReport {
    trigger: RecoveryTrigger,
    recovered: bool,
}

impl RecoveryReport {
    /// Creates a report for one recovery attempt.
    #[must_use]
    pub const fn new(trigger: RecoveryTrigger, recovered: bool) -> Self {
        Self { trigger, recovered }
    }

    /// Returns what forced the recovery attempt.
    #[must_use]
    pub const fn trigger(&self) -> RecoveryTrigger {
        self.trigger
    }

    /// Returns whether the attempt restored a presentable state.
    ///
    /// A failed attempt is fatal: the runner exits and [`crate::run`] returns
    /// the presenter error after this hook observes the report.
    #[must_use]
    pub const fn recovered(&self) -> bool {
        self.recovered
    }
}

/// One windowed Stern application hosted by [`crate::run`].
///
/// The runner owns the event loop, window, input adaptation, retained UI
/// state (memory, theme, and shaped-text caching), platform-request
/// application, repaint scheduling, and the presenter's resize, recovery, and
/// present state machine. The application owns only its domain state and the
/// per-frame composition expressed through these hooks. Every hook except
/// [`Self::frame`] has a default no-op implementation.
pub trait App {
    /// Composes one frame of widgets and records shell intent.
    fn frame(&mut self, ui: &mut Ui<'_>, shell: &mut ShellCtx);

    /// Executes action invocations emitted by the frame that just finished.
    ///
    /// The queue contains widget- and shortcut-emitted invocations in frame
    /// order. Intent recorded on `shell` here (close, focus, requests,
    /// repaint) still applies to the same frame. When the queue is non-empty
    /// the runner schedules at least one follow-up frame so state changes
    /// become visible.
    fn on_actions(&mut self, _actions: &mut ActionQueue, _shell: &mut ShellCtx) {}

    /// Registers application-owned renderer resources for the frame.
    ///
    /// Shaped-text layout resources are registered by the runner; register
    /// only domain resources such as textures and images here.
    fn register_resources(&mut self, _resources: &mut RenderResources) {}

    /// Observes the typed status of one presentation attempt.
    fn on_present(&mut self, _status: VelloPresentStatus) {}

    /// Observes one automatic GPU recovery attempt.
    ///
    /// The runner always recovers automatically and silently; this hook only
    /// reports what happened.
    fn on_recovery(&mut self, _report: &RecoveryReport) {}

    /// Called once when the event loop is exiting.
    fn on_exit(&mut self) {}
}

impl<T: App + ?Sized> App for &mut T {
    fn frame(&mut self, ui: &mut Ui<'_>, shell: &mut ShellCtx) {
        (**self).frame(ui, shell);
    }

    fn on_actions(&mut self, actions: &mut ActionQueue, shell: &mut ShellCtx) {
        (**self).on_actions(actions, shell);
    }

    fn register_resources(&mut self, resources: &mut RenderResources) {
        (**self).register_resources(resources);
    }

    fn on_present(&mut self, status: VelloPresentStatus) {
        (**self).on_present(status);
    }

    fn on_recovery(&mut self, report: &RecoveryReport) {
        (**self).on_recovery(report);
    }

    fn on_exit(&mut self) {
        (**self).on_exit();
    }
}

#[cfg(test)]
mod tests {
    use stern_core::{
        ActionQueue, FrameContext, PhysicalSize, Rect, ScaleFactor, Size, TimeInfo, UiInput,
        UiMemory, ViewportInfo, default_dark_theme,
    };
    use stern_widgets::Ui;

    use super::App;
    use crate::shell::ShellCtx;

    struct CountingApp {
        frames: u32,
        action_batches: u32,
    }

    impl App for CountingApp {
        fn frame(&mut self, ui: &mut Ui<'_>, shell: &mut ShellCtx) {
            ui.label(Rect::new(0.0, 0.0, 120.0, 18.0), "counting");
            shell.request_repaint();
            self.frames += 1;
        }

        fn on_actions(&mut self, _actions: &mut ActionQueue, _shell: &mut ShellCtx) {
            self.action_batches += 1;
        }
    }

    #[test]
    fn mutable_references_forward_every_hook_to_the_underlying_app() {
        let theme = default_dark_theme();
        let context = FrameContext::new(
            ViewportInfo::new(
                Size::new(320.0, 240.0),
                PhysicalSize::new(320, 240),
                ScaleFactor::ONE,
            ),
            UiInput::default(),
            TimeInfo::default(),
        );
        let mut memory = UiMemory::new();
        let mut app = CountingApp {
            frames: 0,
            action_batches: 0,
        };
        let mut shell = ShellCtx::new(ScaleFactor::ONE);

        {
            let mut ui = Ui::begin_frame(context, &mut memory, &theme);
            let mut by_reference: &mut CountingApp = &mut app;
            App::frame(&mut by_reference, &mut ui, &mut shell);
            App::on_actions(&mut by_reference, &mut ActionQueue::new(), &mut shell);
            let _ = ui.finish_output();
        }

        assert_eq!(app.frames, 1);
        assert_eq!(app.action_batches, 1);
        assert_eq!(shell.repaint(), stern_core::RepaintRequest::NextFrame);
    }
}
