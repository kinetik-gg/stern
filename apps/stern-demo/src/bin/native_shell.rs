//! Public-facade native shell spike for the Stern integration demo.
use std::sync::Arc;
use std::time::Instant;

use stern::UiState;
use stern::core::{
    ActionContext, ActionDescriptor, FrameContext, FrameOutput, PlatformRequest, PointerOrder,
    Rect, WidgetId, default_dark_theme,
};
use stern::platform_winit::{
    NativeWinitShellServices, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
    WinitRepaintSchedule, WinitRepaintScheduler, frame_context_from_winit, scale_factor_from_winit,
};
use stern::render::RenderFrameInput;
use stern::vello_winit::{
    VelloPresentStatus, VelloPresenterConfig, VelloPresenterError, VelloRedrawGuidance,
    VelloResizeOutcome, VelloWindowPresenter,
};
use stern::widgets::dock::{DockScene, DockSceneConfig};
use stern::widgets::{
    ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, Dock, DockNode, Frame, FrameId, FrameTab,
    MenuBar, MenuBarMenu, MenuBarMenuId, Panel, PanelId, StatusBar, StatusItem, StatusItemId,
    StatusItemKind, TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

const TITLE: &str = "Stern Public Native Shell";
fn shell_dock() -> Dock {
    Dock::new(DockNode::Frame(Frame::new(
        FrameId::from_raw(2),
        vec![Panel::new(PanelId::from_raw(21), "Editor")],
    )))
}

fn build_shell_frame(
    state: &mut UiState,
    dock: &Dock,
    context: FrameContext,
) -> Result<FrameOutput, stern::core::PointerPlanError> {
    let size = context.viewport.logical_size;
    let width = size.width.max(1.0);
    let height = size.height.max(1.0);
    let menu_id = MenuBarMenuId::from_raw(1);
    let group_id = ToolbarGroupId::from_raw(1);
    let status_id = StatusItemId::from_raw(1);
    let action = ActionDescriptor::new("shell.refresh", "Refresh");
    let menu = MenuBar::from_menus([MenuBarMenu::from_actions(menu_id, "File", [action.clone()])]);
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        group_id,
        "Workspace",
        [action.clone()],
    )]);
    let tabs = TabStrip::from_tabs([FrameTab {
        panel: PanelId::from_raw(21),
        title: "Workspace".to_owned(),
        active: true,
        close_visible: false,
        draggable: false,
    }]);
    let status = StatusBar::from_items([StatusItem::new(
        status_id,
        "Renderer",
        "Ready",
        StatusItemKind::Ready,
    )]);
    let chrome = ChromeScene::new(
        ChromeSceneConfig::new(
            WidgetId::from_key("native-shell-chrome"),
            Rect::new(0.0, 0.0, width, 28.0),
            Rect::new(0.0, 28.0, width, 28.0),
            Rect::new(0.0, 56.0, width, 28.0),
            Rect::new(0.0, (height - 24.0).max(84.0), width, 24.0),
            ActionContext::Editor,
        )
        .with_widths([
            (ChromeSceneItemKey::Menu(menu_id), 56.0),
            (
                ChromeSceneItemKey::Toolbar {
                    group: group_id,
                    action: action.id.clone(),
                },
                76.0,
            ),
            (ChromeSceneItemKey::Tab(PanelId::from_raw(21)), 112.0),
            (ChromeSceneItemKey::Status(status_id), 96.0),
        ]),
        &menu,
        &toolbar,
        &tabs,
        &status,
    );
    let dock = DockScene::new(
        DockSceneConfig::new(
            WidgetId::from_key("native-shell-dock"),
            Rect::new(0.0, 84.0, width, (height - 108.0).max(0.0)),
        ),
        dock,
    );
    let theme = default_dark_theme();
    let mut ui = state.begin_frame(context, &theme);
    ui.push_platform_request(PlatformRequest::SetWindowTitle(TITLE.to_owned()));
    ui.resolve_pointer_targets(|plan| {
        let next = dock.declare_pointer_targets(plan, PointerOrder::new(1));
        let _ = chrome.declare_pointer_targets(plan, next);
    })?;
    let _ = ui.chrome_scene(&chrome);
    let _ = ui.dock_scene(&dock, |ui, panel| {
        ui.label_keyed(
            ("panel-content", panel.panel.raw()),
            panel.rect.inset(12.0),
            format!("{} panel", panel.title),
        );
    });
    Ok(ui.finish_output())
}

struct NativeShell {
    presenter: VelloWindowPresenter,
    window: Option<Arc<Window>>,
    state: UiState,
    dock: Dock,
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
            state: UiState::new(),
            dock: shell_dock(),
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
        let output = match build_shell_frame(&mut self.state, &self.dock, context) {
            Ok(output) => output,
            Err(error) => {
                self.fail(event_loop, format!("pointer plan failed: {error:?}"));
                return;
            }
        };
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
        let resources = self.state.text_render_resources();
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
                .with_title(TITLE)
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
fn test_context() -> FrameContext {
    stern::core::FrameContext::new(
        stern::core::ViewportInfo::new(
            stern::core::Size::new(960.0, 640.0),
            stern::core::PhysicalSize::new(960, 640),
            stern::core::ScaleFactor::ONE,
        ),
        stern::core::UiInput::default(),
        stern::core::TimeInfo::default(),
    )
}

#[cfg(test)]
#[test]
fn native_shell_composes_public_chrome_dock_frame_and_panel() {
    let output = build_shell_frame(&mut UiState::new(), &shell_dock(), test_context()).unwrap();
    let roles = output
        .semantics
        .nodes()
        .iter()
        .map(|node| &node.role)
        .collect::<Vec<_>>();

    for role in [
        stern::core::SemanticRole::Dock,
        stern::core::SemanticRole::Frame,
        stern::core::SemanticRole::Panel,
        stern::core::SemanticRole::TabList,
    ] {
        assert!(roles.contains(&&role), "missing {role:?}");
    }
    for role in ["menu-bar", "toolbar", "status-bar"] {
        assert!(roles.contains(&&stern::core::SemanticRole::Custom(role.to_owned())));
    }
    assert!(!output.primitives.is_empty());
    assert!(output.warnings.is_empty());
}

#[cfg(test)]
#[derive(Default)]
struct FakeWindow {
    cursor: Option<winit::window::CursorIcon>,
    title: Option<String>,
    ime_allowed: bool,
    ime_rect: Option<Rect>,
}

#[cfg(test)]
impl stern::platform_winit::WinitWindowOps for FakeWindow {
    fn set_cursor(&mut self, cursor: winit::window::CursorIcon) {
        self.cursor = Some(cursor);
    }
    fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_owned());
    }
    fn set_ime_allowed(&mut self, allowed: bool) {
        self.ime_allowed = allowed;
    }
    fn set_ime_cursor_area(&mut self, rect: Rect) {
        self.ime_rect = Some(rect);
    }
}

#[cfg(test)]
#[test]
fn native_shell_translates_one_frame_platform_batch() {
    let output = build_shell_frame(&mut UiState::new(), &shell_dock(), test_context()).unwrap();
    let mut window = FakeWindow::default();
    let applied =
        WinitPlatformRequests::from_frame_output(&output).apply_to_window_ops(&mut window);

    assert_eq!(window.title.as_deref(), Some(TITLE));
    assert!(window.cursor.is_some());
    assert!(!window.ime_allowed);
    assert_eq!(window.ime_rect, None);
    assert!(applied.shell().is_empty());
    assert_eq!(applied.repaint(), output.repaint);
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
