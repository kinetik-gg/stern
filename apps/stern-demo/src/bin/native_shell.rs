//! Public native host for the real Stern integration demo.
use std::sync::Arc;
use std::time::Instant;

use stern::platform_winit::{
    NativeWinitShellServices, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
    WinitRepaintSchedule, WinitRepaintScheduler, frame_context_from_winit, scale_factor_from_winit,
};
use stern::render::RenderFrameInput;
use stern::vello_winit::{
    VelloPresentStatus, VelloPresenterConfig, VelloPresenterError, VelloRedrawGuidance,
    VelloResizeOutcome, VelloWindowPresenter,
};
use stern_demo::{DEMO_TITLE, DemoApp};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

struct NativeShell {
    presenter: VelloWindowPresenter,
    window: Option<Arc<Window>>,
    app: DemoApp,
    input: WinitInputAdapter,
    modifiers: ModifiersState,
    clock: WinitFrameClock,
    repaint: WinitRepaintScheduler,
    shell: NativeWinitShellServices,
    started: Instant,
    smoke: bool,
    presented: bool,
    failure: Option<String>,
}

impl NativeShell {
    fn new(smoke: bool) -> Result<Self, VelloPresenterError> {
        Ok(Self {
            presenter: VelloWindowPresenter::new(VelloPresenterConfig::new())?,
            window: None,
            app: DemoApp::new(),
            input: WinitInputAdapter::default(),
            modifiers: ModifiersState::default(),
            clock: WinitFrameClock::new(),
            repaint: WinitRepaintScheduler::new(),
            shell: NativeWinitShellServices::new(),
            started: Instant::now(),
            smoke,
            presented: false,
            failure: None,
        })
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl std::fmt::Display) {
        self.failure = Some(error.to_string());
        event_loop.exit();
    }

    fn resize(&mut self, event_loop: &ActiveEventLoop, size: winit::dpi::PhysicalSize<u32>) {
        match self.presenter.resize(size) {
            Ok(VelloResizeOutcome::RecoveryRequired(_)) => {
                if let Err(error) = pollster::block_on(self.presenter.recover()) {
                    self.fail(event_loop, error);
                }
            }
            Ok(_) => {}
            Err(error) => self.fail(event_loop, error),
        }
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
        let output = self.app.frame(context);
        self.input.begin_frame();
        let applied = WinitPlatformRequests::from_frame_output(&output).apply_to_window(window);
        let (shell, repaint) = applied.into_parts();
        let outcome = shell.execute_for_window(window, &mut self.shell);
        let has_shell_input = !outcome.results().is_empty();
        for failure in self.input.apply_shell_outcome(outcome) {
            eprintln!("shell operation failed: {:?}", failure.operation);
        }
        self.repaint
            .replace_frame_request(repaint, has_shell_input, Instant::now());
        let resources = self.app.render_resources();
        let report = match self.presenter.present(RenderFrameInput {
            viewport,
            primitives: &output.primitives,
            resources: &resources,
        }) {
            Ok(report) => report,
            Err(error) => {
                self.fail(event_loop, error);
                return;
            }
        };
        if matches!(
            report.status(),
            VelloPresentStatus::Presented | VelloPresentStatus::PresentedSuboptimal
        ) {
            self.presented = true;
            if self.smoke {
                println!("native-shell-smoke=pass status={:?}", report.status());
                event_loop.exit();
                return;
            }
        }
        if matches!(
            report.status(),
            VelloPresentStatus::SurfaceRecoveryRequired
                | VelloPresentStatus::DeviceRecoveryRequired
                | VelloPresentStatus::SurfaceLost
        ) && let Err(error) = pollster::block_on(self.presenter.recover())
        {
            self.fail(event_loop, error);
            return;
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

impl ApplicationHandler for NativeShell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(DEMO_TITLE)
                .with_inner_size(LogicalSize::new(960.0, 640.0)),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, error);
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
            Err(error) => self.fail(event_loop, error),
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|argument| argument == "--smoke-exit-after-present");
    let event_loop = EventLoop::new()?;
    let mut app = NativeShell::new(smoke)?;
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if smoke && !app.presented {
        return Err("native shell exited without a successful present".into());
    }
    Ok(())
}

#[cfg(test)]
fn test_context(input: stern::core::UiInput) -> stern::core::FrameContext {
    stern::core::FrameContext::new(
        stern::core::ViewportInfo::new(
            stern::core::Size::new(960.0, 640.0),
            stern::core::PhysicalSize::new(960, 640),
            stern::core::ScaleFactor::ONE,
        ),
        input,
        stern::core::TimeInfo::default(),
    )
}

#[cfg(test)]
#[test]
fn native_shell_hosts_real_edit_and_graph_workspaces() {
    use stern::core::{Point, PointerButtonState, PointerInput, SemanticRole, UiInput};
    use stern_demo::DemoWorkspace;

    fn workspace_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(point),
                primary: PointerButtonState::new(down, pressed, released),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    let mut app = DemoApp::new();
    let edit = app.frame(test_context(UiInput::default()));
    assert_eq!(app.workspace(), DemoWorkspace::Edit);
    assert!(edit.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Dock && node.label.as_deref() == Some("Editor dock")
    }));
    let graph = edit
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.role == SemanticRole::IconButton
                && node.label.as_deref() == Some("Graph Workspace")
        })
        .expect("Graph workspace action")
        .bounds
        .center();
    let _ = app.frame(test_context(workspace_input(graph, true, true, false)));
    let switched = app.frame(test_context(workspace_input(graph, false, false, true)));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
    let mut actions = switched.actions.clone();
    assert!(
        actions
            .drain()
            .any(|invocation| invocation.action_id.as_str() == "workspace.graph")
    );
    let graph = app.frame(test_context(UiInput::default()));
    assert!(
        graph.semantics.nodes().iter().any(|node| {
            matches!(&node.role, SemanticRole::Custom(role) if role == "node-graph")
        })
    );
}

#[cfg(test)]
#[test]
fn native_shell_presenter_constructs_detached_without_gpu() {
    let presenter = VelloWindowPresenter::new(VelloPresenterConfig::new()).unwrap();

    assert_eq!(
        presenter.status().attachment(),
        stern::vello_winit::VelloAttachmentStatus::Detached
    );
    assert!(presenter.status().device_scope().is_none());
}
