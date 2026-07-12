use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use kinetik_ui::vello_winit::wgpu::PresentMode;
use kinetik_ui::{
    core::{Color, RepaintRequest},
    platform_winit::{
        NativeWinitShellServices, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        WinitRepaintSchedule, WinitRepaintScheduler, WinitShellFailure, WinitShellOutcome,
        frame_context_from_winit, scale_factor_from_winit,
    },
    render::RenderFrameInput,
    vello_winit::{
        AaConfig, VelloPresentStatus, VelloPresenterConfig, VelloPresenterError,
        VelloRecoveryOutcome, VelloRedrawGuidance, VelloWindowPresenter,
    },
};
use kinetik_ui_showcase::app::{ShowcaseApp, ShowcasePage};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const DEFAULT_WIDTH: f64 = 1440.0;
const DEFAULT_HEIGHT: f64 = 900.0;

pub(crate) fn run(page: Option<ShowcasePage>) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        event_loop_builder.with_dpi_aware(true);
    }
    let event_loop = event_loop_builder.build()?;
    let mut app = LiveShowcase::new(page)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct LiveShowcase {
    app: ShowcaseApp,
    page: Option<ShowcasePage>,
    input: WinitInputAdapter,
    clock: WinitFrameClock,
    started: Instant,
    modifiers: ModifiersState,
    window: Option<Arc<Window>>,
    presenter: VelloWindowPresenter,
    accepting_input: bool,
    repaint: WinitRepaintScheduler,
    shell: NativeWinitShellServices,
}

impl LiveShowcase {
    fn new(page: Option<ShowcasePage>) -> Result<Self, VelloPresenterError> {
        let presenter = VelloWindowPresenter::new(live_presenter_config()?)?;
        Ok(Self {
            app: ShowcaseApp::new(),
            page,
            input: WinitInputAdapter::default(),
            clock: WinitFrameClock::new(),
            started: Instant::now(),
            modifiers: ModifiersState::empty(),
            window: None,
            presenter,
            accepting_input: false,
            repaint: WinitRepaintScheduler::new(),
            shell: NativeWinitShellServices::new(),
        })
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn request_immediate_redraw(&mut self) {
        self.repaint.request_immediate();
        if self.repaint.take_redraw_request(Instant::now()) {
            self.request_redraw();
        }
    }

    fn request_interactive_redraw(&mut self) {
        self.request_immediate_redraw();
    }

    fn resize_presenter(&mut self, size: PhysicalSize<u32>) -> Result<(), VelloPresenterError> {
        let _ = self.presenter.resize(raw_presenter_size(size))?;
        Ok(())
    }

    fn reset_resume_input_state(&mut self) {
        self.accepting_input = false;
        self.input.set_window_focused(false);
        self.input.begin_frame();
    }

    fn frame_input_snapshot(&mut self, scale_factor: f64) -> kinetik_ui::core::UiInput {
        self.input
            .set_scale_factor(scale_factor_from_winit(scale_factor));
        self.input.input().clone()
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let time = self.clock.tick(self.started.elapsed());
        let input = self.frame_input_snapshot(scale_factor);
        let context = frame_context_from_winit(size, scale_factor, input, time);
        let viewport = context.viewport;

        self.app.update_with_context(context);
        let requests = WinitPlatformRequests::from_frame_output(self.app.output());
        let applied = requests.apply_to_window(&window);
        let (shell_requests, application_repaint) = applied.into_parts();
        let shell_outcome = shell_requests.execute(&mut self.shell);
        let has_shell_input = shell_outcome.has_input_response();

        let present_result = {
            let resources = self.app.render_resources();
            self.presenter.present(RenderFrameInput {
                viewport,
                primitives: &self.app.output().primitives,
                resources,
            })
        };
        let decision = match present_result {
            Ok(report) => {
                if let Some(output) = report.frame_output()
                    && !output.diagnostics.is_empty()
                {
                    eprintln!("showcase renderer diagnostics: {:?}", output.diagnostics);
                }
                if present_status_requires_recovery(report.status()) {
                    SettlementDecision::Recover
                } else {
                    SettlementDecision::Schedule(application_repaint.merge(
                        repaint_for_presenter_guidance(report.status(), report.redraw()),
                    ))
                }
            }
            Err(error) => {
                eprintln!("showcase presenter error: {error}");
                SettlementDecision::Exit
            }
        };

        let mut settlement = LiveFrameSettlement {
            input: &mut self.input,
            shell_outcome,
            presenter: &mut self.presenter,
            repaint: &mut self.repaint,
            event_loop,
            window: &window,
            accepting_input: &mut self.accepting_input,
            application_repaint,
            has_shell_input,
            now: Instant::now(),
        };
        settle_live_frame(&mut settlement, decision);
    }
}

impl ApplicationHandler for LiveShowcase {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = if let Some(window) = self.window.clone() {
            window
        } else {
            self.reset_resume_input_state();
            let attributes = Window::default_attributes()
                .with_title("Kinetik Forge - Kinetik UI")
                .with_inner_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
                .with_min_inner_size(LogicalSize::new(720.0, 480.0));
            let window = match event_loop.create_window(attributes) {
                Ok(window) => Arc::new(window),
                Err(error) => {
                    eprintln!("failed to create showcase window: {error}");
                    event_loop.exit();
                    return;
                }
            };
            self.window = Some(Arc::clone(&window));
            window
        };

        if let Some(page) = self.page.take() {
            self.app.set_page(page);
        }
        self.input
            .set_scale_factor(scale_factor_from_winit(window.scale_factor()));
        if let Err(error) = pollster::block_on(self.presenter.resume(Arc::clone(&window))) {
            eprintln!("failed to resume Vello presenter: {error}");
            event_loop.exit();
            return;
        }
        self.request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.presenter.suspend();
        self.window = None;
        self.reset_resume_input_state();
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
            WindowEvent::Focused(focused) => {
                self.input.set_window_focused(focused);
                self.request_interactive_redraw();
            }
            WindowEvent::Resized(size) => {
                if let Err(error) = self.resize_presenter(size) {
                    eprintln!("showcase presenter resize error: {error}");
                    event_loop.exit();
                    return;
                }
                self.request_interactive_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.input
                    .set_scale_factor(scale_factor_from_winit(scale_factor));
                self.request_interactive_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.pointer_moved(position);
                self.request_interactive_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.pointer_left();
                self.request_interactive_redraw();
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.mouse_button_at(button, state, Instant::now());
                self.request_interactive_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.mouse_wheel(delta);
                self.request_interactive_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.input.set_modifiers(self.modifiers);
                self.request_interactive_redraw();
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
                self.request_interactive_redraw();
            }
            WindowEvent::Ime(event) => {
                self.input.ime_event(event);
                self.request_interactive_redraw();
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return;
        };
        drive_repaint_scheduler(&mut self.repaint, event_loop, &window, Instant::now());
    }
}

trait LiveFrameSettlementOperations {
    fn roll_platform_frame(&mut self);
    fn recover(&mut self) -> Result<RepaintRequest, ()>;
    fn schedule(&mut self, repaint: RepaintRequest);
    fn exit(&mut self);
}

struct LiveFrameSettlement<'a> {
    input: &'a mut WinitInputAdapter,
    shell_outcome: WinitShellOutcome,
    presenter: &'a mut VelloWindowPresenter,
    repaint: &'a mut WinitRepaintScheduler,
    event_loop: &'a ActiveEventLoop,
    window: &'a Window,
    accepting_input: &'a mut bool,
    application_repaint: RepaintRequest,
    has_shell_input: bool,
    now: Instant,
}

impl LiveFrameSettlementOperations for LiveFrameSettlement<'_> {
    fn roll_platform_frame(&mut self) {
        let outcome = std::mem::take(&mut self.shell_outcome);
        let failures = roll_platform_frame(self.input, outcome);
        for failure in failures {
            eprintln!("showcase shell error: {failure}");
        }
        *self.accepting_input = true;
    }

    fn recover(&mut self) -> Result<RepaintRequest, ()> {
        match pollster::block_on(self.presenter.recover()) {
            Ok(outcome) => Ok(self
                .application_repaint
                .merge(repaint_for_recovery_disposition(recovery_disposition(
                    &outcome,
                )))),
            Err(error) => {
                eprintln!("showcase presenter recovery error: {error}");
                Err(())
            }
        }
    }

    fn schedule(&mut self, repaint: RepaintRequest) {
        self.repaint
            .replace_frame_request(repaint, self.has_shell_input, self.now);
        drive_repaint_scheduler(self.repaint, self.event_loop, self.window, Instant::now());
    }

    fn exit(&mut self) {
        self.event_loop.exit();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettlementDecision {
    Schedule(RepaintRequest),
    Recover,
    Exit,
}

fn settle_live_frame<Operations>(operations: &mut Operations, decision: SettlementDecision)
where
    Operations: LiveFrameSettlementOperations,
{
    operations.roll_platform_frame();
    match decision {
        SettlementDecision::Schedule(repaint) => operations.schedule(repaint),
        SettlementDecision::Recover => match operations.recover() {
            Ok(repaint) => operations.schedule(repaint),
            Err(()) => operations.exit(),
        },
        SettlementDecision::Exit => operations.exit(),
    }
}

#[allow(clippy::match_like_matches_macro)]
fn present_status_requires_recovery(status: VelloPresentStatus) -> bool {
    match status {
        VelloPresentStatus::SurfaceLost
        | VelloPresentStatus::SurfaceRecoveryRequired
        | VelloPresentStatus::DeviceRecoveryRequired => true,
        _ => false,
    }
}

#[allow(clippy::match_same_arms)]
fn repaint_for_presenter_guidance(
    status: VelloPresentStatus,
    guidance: VelloRedrawGuidance,
) -> RepaintRequest {
    if present_status_requires_recovery(status) {
        return RepaintRequest::None;
    }
    match guidance {
        VelloRedrawGuidance::UseApplicationRequest
        | VelloRedrawGuidance::ExternalEvent
        | VelloRedrawGuidance::NonZeroResize
        | VelloRedrawGuidance::None => RepaintRequest::None,
        VelloRedrawGuidance::NextFrame => RepaintRequest::NextFrame,
        VelloRedrawGuidance::Later(delay) => RepaintRequest::After(delay),
        _ => RepaintRequest::None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryDisposition {
    Completed,
    Deferred,
}

#[allow(clippy::match_same_arms)]
fn recovery_disposition(outcome: &VelloRecoveryOutcome) -> RecoveryDisposition {
    match outcome {
        VelloRecoveryOutcome::SurfaceReady { .. } | VelloRecoveryOutcome::DeviceRebuilt { .. } => {
            RecoveryDisposition::Completed
        }
        VelloRecoveryOutcome::NotNeeded
        | VelloRecoveryOutcome::DeferredDetached(_)
        | VelloRecoveryOutcome::DeferredZeroSized(_) => RecoveryDisposition::Deferred,
        _ => RecoveryDisposition::Deferred,
    }
}

fn repaint_for_recovery_disposition(disposition: RecoveryDisposition) -> RepaintRequest {
    match disposition {
        RecoveryDisposition::Completed => RepaintRequest::NextFrame,
        RecoveryDisposition::Deferred => RepaintRequest::None,
    }
}

fn live_presenter_config() -> Result<VelloPresenterConfig, VelloPresenterError> {
    let config = VelloPresenterConfig::new()
        .with_present_mode(PresentMode::AutoNoVsync)?
        .with_antialiasing_method(AaConfig::Msaa16)
        .with_base_color(Color::rgb(11.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0))?
        .with_timeout_retry(Duration::from_millis(16))?;
    Ok(config)
}

fn raw_presenter_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    size
}

fn drive_repaint_scheduler(
    scheduler: &mut WinitRepaintScheduler,
    event_loop: &ActiveEventLoop,
    window: &Window,
    now: Instant,
) {
    if scheduler.take_redraw_request(now) {
        window.request_redraw();
    }
    event_loop.set_control_flow(control_flow_for_repaint_schedule(scheduler.schedule()));
}

fn roll_platform_frame(
    input: &mut WinitInputAdapter,
    outcome: WinitShellOutcome,
) -> Vec<WinitShellFailure> {
    input.begin_frame();
    input.apply_shell_outcome(outcome)
}

fn control_flow_for_repaint_schedule(schedule: WinitRepaintSchedule) -> ControlFlow {
    match schedule {
        WinitRepaintSchedule::Idle => ControlFlow::Wait,
        WinitRepaintSchedule::Immediate | WinitRepaintSchedule::Continuous => {
            immediate_redraw_control_flow()
        }
        WinitRepaintSchedule::At(deadline) => ControlFlow::WaitUntil(deadline),
    }
}

fn immediate_redraw_control_flow() -> ControlFlow {
    ControlFlow::Poll
}

#[cfg(test)]
mod tests {
    use super::{
        LiveFrameSettlementOperations, LiveShowcase, RecoveryDisposition, SettlementDecision,
        control_flow_for_repaint_schedule, immediate_redraw_control_flow,
        present_status_requires_recovery, raw_presenter_size, recovery_disposition,
        repaint_for_presenter_guidance, repaint_for_recovery_disposition, roll_platform_frame,
        settle_live_frame,
    };
    use kinetik_ui::vello_winit::wgpu::PresentMode;
    use kinetik_ui::{
        core::{ClipboardText, Color, RepaintRequest, UiInputEvent, WidgetId},
        platform_winit::{
            WinitInputAdapter, WinitRepaintSchedule, WinitShellOutcome, WinitShellResult,
        },
        vello_winit::{
            AaConfig, VelloAttachmentStatus, VelloPresentStatus, VelloRecoveryKind,
            VelloRecoveryOutcome, VelloRedrawGuidance,
        },
    };
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, MouseButton as WinitMouseButton};
    use winit::event_loop::ControlFlow;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SettlementTrace {
        Present,
        RollPlatformFrame,
        Recover,
        Schedule,
        Exit,
    }

    struct RecordingSettlementOperations {
        trace: Vec<SettlementTrace>,
        recovery_result: Option<Result<RepaintRequest, ()>>,
    }

    impl LiveFrameSettlementOperations for RecordingSettlementOperations {
        fn roll_platform_frame(&mut self) {
            self.trace.push(SettlementTrace::RollPlatformFrame);
        }

        fn recover(&mut self) -> Result<RepaintRequest, ()> {
            self.trace.push(SettlementTrace::Recover);
            self.recovery_result.take().unwrap_or(Err(()))
        }

        fn schedule(&mut self, _repaint: RepaintRequest) {
            self.trace.push(SettlementTrace::Schedule);
        }

        fn exit(&mut self) {
            self.trace.push(SettlementTrace::Exit);
        }
    }

    fn record_settlement(
        decision: SettlementDecision,
        recovery_result: Result<RepaintRequest, ()>,
    ) -> Vec<SettlementTrace> {
        let mut recorder = RecordingSettlementOperations {
            trace: Vec::new(),
            recovery_result: Some(recovery_result),
        };
        recorder.trace.push(SettlementTrace::Present);
        settle_live_frame(&mut recorder, decision);
        recorder.trace
    }

    #[test]
    fn live_showcase_starts_with_public_detached_presenter() {
        let app = LiveShowcase::new(None).expect("detached presenter");
        let config = app.presenter.config();

        assert!(app.window.is_none());
        assert_eq!(
            app.presenter.status().attachment(),
            VelloAttachmentStatus::Detached
        );
        assert_eq!(config.present_mode(), PresentMode::AutoNoVsync);
        assert_eq!(config.antialiasing_method(), AaConfig::Msaa16);
        assert_eq!(
            config.base_color(),
            Color::rgb(11.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0)
        );
        assert_eq!(config.timeout_retry(), Duration::from_millis(16));
    }

    #[test]
    fn presenter_guidance_maps_into_application_repaint_policy() {
        let status = VelloPresentStatus::Presented;
        let delay = Duration::from_millis(16);

        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::UseApplicationRequest),
            RepaintRequest::None
        );
        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::ExternalEvent),
            RepaintRequest::None
        );
        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::NonZeroResize),
            RepaintRequest::None
        );
        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::None),
            RepaintRequest::None
        );
        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::NextFrame),
            RepaintRequest::NextFrame
        );
        assert_eq!(
            repaint_for_presenter_guidance(status, VelloRedrawGuidance::Later(delay)),
            RepaintRequest::After(delay)
        );
        assert_eq!(
            repaint_for_presenter_guidance(
                VelloPresentStatus::SurfaceLost,
                VelloRedrawGuidance::NextFrame
            ),
            RepaintRequest::None
        );
        let timeout = repaint_for_presenter_guidance(
            VelloPresentStatus::Timeout,
            VelloRedrawGuidance::Later(delay),
        );
        assert_eq!(
            RepaintRequest::None.merge(timeout),
            RepaintRequest::After(delay)
        );
        assert_eq!(
            RepaintRequest::After(Duration::from_millis(32)).merge(timeout),
            RepaintRequest::After(delay)
        );
        assert_eq!(
            RepaintRequest::NextFrame.merge(timeout),
            RepaintRequest::NextFrame
        );
        assert_eq!(
            RepaintRequest::Continuous.merge(timeout),
            RepaintRequest::Continuous
        );
    }

    #[test]
    fn only_recovery_required_statuses_request_recovery() {
        assert!(present_status_requires_recovery(
            VelloPresentStatus::SurfaceLost
        ));
        assert!(present_status_requires_recovery(
            VelloPresentStatus::SurfaceRecoveryRequired
        ));
        assert!(present_status_requires_recovery(
            VelloPresentStatus::DeviceRecoveryRequired
        ));
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
            assert!(!present_status_requires_recovery(status));
        }
    }

    #[test]
    fn presenter_outcomes_roll_shell_once_before_recovery_or_exit() {
        assert_eq!(
            record_settlement(
                SettlementDecision::Schedule(RepaintRequest::None),
                Ok(RepaintRequest::None)
            ),
            vec![
                SettlementTrace::Present,
                SettlementTrace::RollPlatformFrame,
                SettlementTrace::Schedule
            ]
        );
        assert_eq!(
            record_settlement(SettlementDecision::Recover, Ok(RepaintRequest::NextFrame)),
            vec![
                SettlementTrace::Present,
                SettlementTrace::RollPlatformFrame,
                SettlementTrace::Recover,
                SettlementTrace::Schedule
            ]
        );
        assert_eq!(
            record_settlement(SettlementDecision::Recover, Err(())),
            vec![
                SettlementTrace::Present,
                SettlementTrace::RollPlatformFrame,
                SettlementTrace::Recover,
                SettlementTrace::Exit
            ]
        );
        assert_eq!(
            record_settlement(SettlementDecision::Exit, Ok(RepaintRequest::None)),
            vec![
                SettlementTrace::Present,
                SettlementTrace::RollPlatformFrame,
                SettlementTrace::Exit
            ]
        );
    }

    #[test]
    fn recovery_outcomes_fail_closed_without_busy_retry() {
        assert_eq!(
            repaint_for_recovery_disposition(RecoveryDisposition::Completed),
            RepaintRequest::NextFrame
        );
        assert_eq!(
            repaint_for_recovery_disposition(RecoveryDisposition::Deferred),
            RepaintRequest::None
        );
        for outcome in [
            VelloRecoveryOutcome::NotNeeded,
            VelloRecoveryOutcome::DeferredDetached(VelloRecoveryKind::CreateSurface),
            VelloRecoveryOutcome::DeferredZeroSized(VelloRecoveryKind::RebuildDevice),
        ] {
            assert_eq!(
                recovery_disposition(&outcome),
                RecoveryDisposition::Deferred
            );
            assert_eq!(
                repaint_for_recovery_disposition(recovery_disposition(&outcome)),
                RepaintRequest::None
            );
        }
    }

    #[test]
    fn raw_zero_size_is_not_sanitized() {
        let zero = PhysicalSize::new(0, 0);

        assert_eq!(raw_presenter_size(zero), zero);
    }

    #[test]
    fn suspend_clears_transient_input_authority() {
        let mut app = LiveShowcase::new(None).expect("detached presenter");
        app.input.set_window_focused(true);
        app.input
            .mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        app.accepting_input = true;

        app.reset_resume_input_state();

        assert!(!app.input.input().window_focused);
        assert!(!app.input.input().pointer.primary.pressed);
        assert!(!app.input.input().pointer.primary.down);
        assert!(!app.accepting_input);
    }

    #[test]
    fn repaint_schedules_map_to_event_loop_control_flow() {
        let now = Instant::now();

        assert!(matches!(immediate_redraw_control_flow(), ControlFlow::Poll));
        assert!(matches!(
            control_flow_for_repaint_schedule(WinitRepaintSchedule::Immediate),
            ControlFlow::Poll
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(WinitRepaintSchedule::Continuous),
            ControlFlow::Poll
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(WinitRepaintSchedule::Idle),
            ControlFlow::Wait
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(WinitRepaintSchedule::At(now)),
            ControlFlow::WaitUntil(deadline) if deadline == now
        ));
    }

    #[test]
    fn resume_clears_stale_input_edges_before_new_window_events() {
        let mut app = LiveShowcase::new(None).expect("detached presenter");
        app.input
            .mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);

        app.reset_resume_input_state();

        assert!(!app.input.input().pointer.primary.pressed);
        assert!(!app.input.input().pointer.primary.down);
        assert!(!app.accepting_input);
    }

    #[test]
    fn first_redraw_snapshot_preserves_input_edges_recorded_after_resume() {
        let mut app = LiveShowcase::new(None).expect("detached presenter");
        app.reset_resume_input_state();
        app.input
            .mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);

        let input = app.frame_input_snapshot(1.5);

        assert!(input.pointer.primary.pressed);
        assert!(input.pointer.primary.down);
    }

    #[test]
    fn recoverable_frame_roll_clears_old_edges_and_preserves_shell_response() {
        let target = WidgetId::from_key("field");
        let mut input = WinitInputAdapter::default();
        input.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
        let outcome = WinitShellOutcome::from_results([WinitShellResult::ClipboardText(
            ClipboardText::new(target, "paste"),
        )]);

        let failures = roll_platform_frame(&mut input, outcome);

        assert!(failures.is_empty());
        assert!(!input.input().pointer.primary.pressed);
        assert!(input.input().pointer.primary.down);
        assert_eq!(
            input.input().events,
            vec![UiInputEvent::ClipboardText(ClipboardText::new(
                target, "paste"
            ))]
        );
    }
}
