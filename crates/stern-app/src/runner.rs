use std::sync::Arc;
use std::time::Instant;

use stern_core::UiMemory;
use stern_render::{RenderFrameInput, RenderResources};
use stern_text::TextLayoutStore;
use stern_vello_winit::{
    VelloPresentStatus, VelloRedrawGuidance, VelloResizeOutcome, VelloWindowPresenter,
};
use stern_widgets::Ui;
use stern_winit::{
    NativeWinitShellServices, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
    WinitRepaintSchedule, WinitRepaintScheduler, frame_context_from_winit, scale_factor_from_winit,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

use crate::app::{App, RecoveryReport, RecoveryTrigger};
use crate::config::AppConfig;
use crate::error::AppError;
use crate::shell::{ShellCtx, ShellFocusRequest};

/// Runs one windowed application until its event loop exits.
///
/// The runner validates the configuration, creates the window and presenter,
/// and drives [`App::frame`] plus the remaining [`App`] hooks from the
/// runner-owned winit event loop. It reuses the existing presenter
/// resize/recovery/present state machine and recovers from GPU loss
/// automatically, surfacing each attempt through [`App::on_recovery`].
///
/// # Errors
///
/// Returns an [`AppError`] when the configuration is invalid, the event loop
/// or window cannot be created, or the presenter fails to initialize,
/// present, or recover.
pub fn run(config: AppConfig, app: impl App) -> Result<(), AppError> {
    config.validate()?;
    let presenter = VelloWindowPresenter::new(config.presenter_config().clone())?;
    let event_loop = EventLoop::new()?;
    let mut runner = Runner {
        app,
        config,
        presenter,
        window: None,
        input: WinitInputAdapter::default(),
        modifiers: ModifiersState::default(),
        clock: WinitFrameClock::new(),
        repaint: WinitRepaintScheduler::new(),
        services: NativeWinitShellServices::new(),
        memory: UiMemory::new(),
        text_layouts: TextLayoutStore::new(),
        started: Instant::now(),
        failure: None,
    };
    event_loop.run_app(&mut runner)?;
    match runner.failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Returns whether a present status requires an automatic recovery attempt.
const fn present_status_requires_recovery(status: VelloPresentStatus) -> bool {
    matches!(
        status,
        VelloPresentStatus::SurfaceLost
            | VelloPresentStatus::SurfaceRecoveryRequired
            | VelloPresentStatus::DeviceRecoveryRequired
    )
}

struct Runner<A: App> {
    app: A,
    config: AppConfig,
    presenter: VelloWindowPresenter,
    window: Option<Arc<Window>>,
    input: WinitInputAdapter,
    modifiers: ModifiersState,
    clock: WinitFrameClock,
    repaint: WinitRepaintScheduler,
    services: NativeWinitShellServices,
    memory: UiMemory,
    text_layouts: TextLayoutStore,
    started: Instant,
    failure: Option<AppError>,
}

impl<A: App> Runner<A> {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: AppError) {
        if self.failure.is_none() {
            self.failure = Some(error);
        }
        event_loop.exit();
    }

    fn resize(&mut self, event_loop: &ActiveEventLoop, size: PhysicalSize<u32>) {
        match self.presenter.resize(size) {
            Ok(VelloResizeOutcome::RecoveryRequired(kind)) => {
                self.recover(event_loop, RecoveryTrigger::Resize(kind));
            }
            Ok(_) => {}
            Err(error) => self.fail(event_loop, AppError::Presenter(error)),
        }
    }

    fn recover(&mut self, event_loop: &ActiveEventLoop, trigger: RecoveryTrigger) {
        let recovered = match pollster::block_on(self.presenter.recover()) {
            Ok(_) => {
                self.repaint.request_immediate();
                true
            }
            Err(error) => {
                self.fail(event_loop, AppError::Presenter(error));
                false
            }
        };
        self.app
            .on_recovery(&RecoveryReport::new(trigger, recovered));
    }

    fn schedule(&mut self, event_loop: &ActiveEventLoop, window: &Window) {
        match self.repaint.schedule() {
            WinitRepaintSchedule::Idle => event_loop.set_control_flow(ControlFlow::Wait),
            WinitRepaintSchedule::Immediate => window.request_redraw(),
            WinitRepaintSchedule::At(deadline) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            WinitRepaintSchedule::Continuous => {
                event_loop.set_control_flow(ControlFlow::Poll);
                window.request_redraw();
            }
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop, window: &Window) {
        let context = frame_context_from_winit(
            window.inner_size(),
            window.scale_factor(),
            self.input.input().clone(),
            self.clock.tick(self.started.elapsed()),
        );
        let viewport = context.viewport;
        let mut shell = ShellCtx::new(viewport.scale_factor);
        let mut output = {
            let mut ui = Ui::begin_frame_with_text_layouts(
                context,
                &mut self.memory,
                self.config.theme(),
                &mut self.text_layouts,
            );
            self.app.frame(&mut ui, &mut shell);
            ui.finish_output()
        };
        let had_actions = !output.actions.is_empty();
        self.app.on_actions(&mut output.actions, &mut shell);
        shell.merge_into(&mut output);
        match shell.take_focus_request() {
            Some(ShellFocusRequest::Focus(target)) => self.memory.focus(target),
            Some(ShellFocusRequest::Clear) => self.memory.clear_focus(),
            None => {}
        }
        self.input.begin_frame();
        let applied = WinitPlatformRequests::from_frame_output(&output).apply_to_window(window);
        let (shell_work, repaint) = applied.into_parts();
        let outcome = shell_work.execute_for_window(window, &mut self.services);
        let has_shell_input = !outcome.results().is_empty();
        for failure in self.input.apply_shell_outcome(outcome) {
            eprintln!("stern-app: shell operation failed: {:?}", failure.operation);
        }
        self.repaint
            .replace_frame_request(repaint, has_shell_input || had_actions, Instant::now());
        if shell.close_requested() {
            event_loop.exit();
            return;
        }
        let mut resources = RenderResources::new();
        resources.register_text_layouts(self.text_layouts.layouts());
        self.app.register_resources(&mut resources);
        let report = match self.presenter.present(RenderFrameInput {
            viewport,
            primitives: &output.primitives,
            resources: &resources,
        }) {
            Ok(report) => report,
            Err(error) => {
                self.fail(event_loop, AppError::Presenter(error));
                return;
            }
        };
        let status = report.status();
        self.app.on_present(status);
        if present_status_requires_recovery(status) {
            self.recover(event_loop, RecoveryTrigger::Present(status));
        }
        match report.redraw() {
            VelloRedrawGuidance::NextFrame => self.repaint.request_immediate(),
            VelloRedrawGuidance::Later(delay) => {
                event_loop.set_control_flow(ControlFlow::wait_duration(delay));
            }
            _ => {}
        }
        self.schedule(event_loop, window);
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let initial_size = self.config.initial_size();
        let mut attributes = Window::default_attributes()
            .with_title(self.config.title())
            .with_inner_size(LogicalSize::new(
                f64::from(initial_size.width),
                f64::from(initial_size.height),
            ));
        if let Some(min_size) = self.config.min_size() {
            attributes = attributes.with_min_inner_size(LogicalSize::new(
                f64::from(min_size.width),
                f64::from(min_size.height),
            ));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, AppError::Window(error.to_string()));
                return;
            }
        };
        self.input
            .set_scale_factor(scale_factor_from_winit(window.scale_factor()));
        match pollster::block_on(self.presenter.resume(Arc::clone(&window))) {
            Ok(_) => {
                self.window = Some(Arc::clone(&window));
                self.resize(event_loop, window.inner_size());
                window.request_redraw();
            }
            Err(error) => self.fail(event_loop, AppError::Presenter(error)),
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.presenter.suspend();
        self.window = None;
        self.clock.reset();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.presenter.accepts_window(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(event_loop, size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => self
                .input
                .set_scale_factor(scale_factor_from_winit(scale_factor)),
            WindowEvent::CursorMoved { position, .. } => self.input.pointer_moved(position),
            WindowEvent::CursorLeft { .. } => self.input.pointer_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.mouse_button_at(button, state, Instant::now());
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.mouse_wheel(delta),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.input.set_modifiers(self.modifiers);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.keyboard_event_with_physical_key_and_text(
                    &event.logical_key,
                    &event.physical_key,
                    event.state,
                    self.modifiers,
                    event.repeat,
                    event.text.as_deref(),
                );
            }
            WindowEvent::Ime(event) => self.input.ime_event(event),
            WindowEvent::Focused(focused) => self.input.set_window_focused(focused),
            WindowEvent::RedrawRequested => {
                if let Some(window) = self.window.clone() {
                    self.redraw(event_loop, &window);
                }
                return;
            }
            _ => return,
        }
        if let Some(window) = &self.window {
            self.repaint.request_immediate();
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.repaint.take_redraw_request(Instant::now())
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.on_exit();
    }
}

#[cfg(test)]
mod tests {
    use stern_vello_winit::VelloPresentStatus;

    use super::present_status_requires_recovery;

    #[test]
    fn only_lost_surface_and_recovery_statuses_trigger_automatic_recovery() {
        for status in [
            VelloPresentStatus::SurfaceLost,
            VelloPresentStatus::SurfaceRecoveryRequired,
            VelloPresentStatus::DeviceRecoveryRequired,
        ] {
            assert!(present_status_requires_recovery(status), "{status:?}");
        }
        for status in [
            VelloPresentStatus::Presented,
            VelloPresentStatus::PresentedSuboptimal,
            VelloPresentStatus::FrameExtentOutdated,
            VelloPresentStatus::AcquiredExtentOutdated,
            VelloPresentStatus::Timeout,
            VelloPresentStatus::Occluded,
            VelloPresentStatus::Outdated,
            VelloPresentStatus::Detached,
            VelloPresentStatus::ZeroSized,
        ] {
            assert!(!present_status_requires_recovery(status), "{status:?}");
        }
    }
}
