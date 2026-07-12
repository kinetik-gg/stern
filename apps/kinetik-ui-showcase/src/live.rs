use std::{fmt, sync::Arc, time::Instant};

use kinetik_ui::{
    core::{FrameOutput, RepaintRequest, ViewportInfo},
    platform_winit::{
        NativeWinitShellServices, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        WinitRepaintSchedule, WinitRepaintScheduler, WinitShellFailure, WinitShellOutcome,
        frame_context_from_winit, scale_factor_from_winit,
    },
    render::{RenderFrameInput, RenderResources},
    render_vello::VelloRenderer,
};
use kinetik_ui_showcase::app::{ShowcaseApp, ShowcasePage};
use vello::{
    AaConfig, RenderParams, Renderer, RendererOptions,
    peniko::Color as VelloColor,
    util::{RenderContext, RenderSurface},
    wgpu::{CommandEncoderDescriptor, CurrentSurfaceTexture, PresentMode, TextureViewDescriptor},
};
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
const MIN_WIDTH: u32 = 1;
const MIN_HEIGHT: u32 = 1;

pub(crate) fn run(page: Option<ShowcasePage>) -> Result<(), winit::error::EventLoopError> {
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        event_loop_builder.with_dpi_aware(true);
    }
    let event_loop = event_loop_builder.build()?;
    let mut app = LiveShowcase::new(page);
    event_loop.run_app(&mut app)
}

struct LiveShowcase {
    app: ShowcaseApp,
    page: Option<ShowcasePage>,
    input: WinitInputAdapter,
    clock: WinitFrameClock,
    started: Instant,
    modifiers: ModifiersState,
    window: Option<Arc<Window>>,
    renderer: Option<LiveVelloRenderer>,
    accepting_input: bool,
    repaint: WinitRepaintScheduler,
    shell: NativeWinitShellServices,
}

impl LiveShowcase {
    fn new(page: Option<ShowcasePage>) -> Self {
        Self {
            app: ShowcaseApp::new(),
            page,
            input: WinitInputAdapter::default(),
            clock: WinitFrameClock::new(),
            started: Instant::now(),
            modifiers: ModifiersState::empty(),
            window: None,
            renderer: None,
            accepting_input: false,
            repaint: WinitRepaintScheduler::new(),
            shell: NativeWinitShellServices::new(),
        }
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

    fn resize_renderer(&mut self, size: PhysicalSize<u32>) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(sanitize_physical_size(size));
        }
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
        let size = sanitize_physical_size(window.inner_size());
        let scale_factor = window.scale_factor();
        let time = self.clock.tick(self.started.elapsed());
        let input = self.frame_input_snapshot(scale_factor);
        let context = frame_context_from_winit(size, scale_factor, input, time);
        let viewport = context.viewport;

        self.app.update_with_context(context);
        let resources = self.app.render_resources();
        let requests = WinitPlatformRequests::from_frame_output(self.app.output());
        let applied = requests.apply_to_window(&window);
        let (shell_requests, frame_repaint) = applied.into_parts();
        let shell_outcome = shell_requests.execute(&mut self.shell);
        let has_shell_input = shell_outcome.has_input_response();

        window.pre_present_notify();
        let retry_surface_redraw = if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(size);
            match renderer.render(self.app.output(), resources, viewport) {
                Ok(()) => false,
                Err(LiveRenderError::Surface(status)) => {
                    handle_surface_status(status, renderer, size);
                    surface_status_requests_redraw(status)
                }
                Err(error) => {
                    eprintln!("showcase render error: {error}");
                    event_loop.exit();
                    false
                }
            }
        } else {
            eprintln!("showcase render error: renderer unavailable");
            event_loop.exit();
            false
        };

        let failures = roll_platform_frame(&mut self.input, shell_outcome);
        for failure in failures {
            eprintln!("showcase shell error: {failure}");
        }
        self.accepting_input = true;

        let repaint = if retry_surface_redraw {
            frame_repaint.merge(RepaintRequest::NextFrame)
        } else {
            frame_repaint
        };
        self.repaint
            .replace_frame_request(repaint, has_shell_input, Instant::now());
        drive_repaint_scheduler(&mut self.repaint, event_loop, &window, Instant::now());
    }
}

impl ApplicationHandler for LiveShowcase {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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

        if let Some(page) = self.page.take() {
            self.app.set_page(page);
        }
        self.input
            .set_scale_factor(scale_factor_from_winit(window.scale_factor()));
        let size = sanitize_physical_size(window.inner_size());
        let renderer = match pollster::block_on(LiveVelloRenderer::new(Arc::clone(&window), size)) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("failed to initialize Vello renderer: {error}");
                event_loop.exit();
                return;
            }
        };

        self.renderer = Some(renderer);
        self.window = Some(window);
        self.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.id() != window_id)
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(focused) => {
                self.input.set_window_focused(focused);
                self.request_interactive_redraw();
            }
            WindowEvent::Resized(size) => {
                self.resize_renderer(size);
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

struct LiveVelloRenderer {
    context: RenderContext,
    surface: RenderSurface<'static>,
    toolkit: VelloRenderer,
    renderer: Renderer,
}

impl LiveVelloRenderer {
    async fn new(window: Arc<Window>, size: PhysicalSize<u32>) -> Result<Self, vello::Error> {
        let size = sanitize_physical_size(size);
        let mut context = RenderContext::new();
        let surface = context
            .create_surface(window, size.width, size.height, live_present_mode())
            .await?;
        let device = &context.devices[surface.dev_id].device;
        let renderer = Renderer::new(device, RendererOptions::default())?;
        Ok(Self {
            context,
            surface,
            toolkit: VelloRenderer::new(),
            renderer,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.resize_surface(size, SurfaceResizeMode::IfChanged);
    }

    fn reconfigure(&mut self, size: PhysicalSize<u32>) {
        self.resize_surface(size, SurfaceResizeMode::Force);
    }

    fn resize_surface(&mut self, size: PhysicalSize<u32>, mode: SurfaceResizeMode) {
        let size = sanitize_physical_size(size);
        let current = PhysicalSize::new(self.surface.config.width, self.surface.config.height);
        if !surface_resize_required(current, size, mode) {
            return;
        }
        self.context
            .resize_surface(&mut self.surface, size.width, size.height);
    }

    fn render(
        &mut self,
        frame: &FrameOutput,
        resources: &RenderResources,
        viewport: ViewportInfo,
    ) -> Result<(), LiveRenderError> {
        self.resize(PhysicalSize::new(
            viewport.physical_size.width,
            viewport.physical_size.height,
        ));
        let output = self.toolkit.submit_frame(RenderFrameInput {
            viewport,
            primitives: &frame.primitives,
            resources,
        });
        if !output.diagnostics.is_empty() {
            eprintln!("showcase renderer diagnostics: {:?}", output.diagnostics);
        }

        let device_handle = &self.context.devices[self.surface.dev_id];
        let surface_extent =
            PhysicalSize::new(self.surface.config.width, self.surface.config.height);
        if !viewport_surface_extents_match(viewport, surface_extent) {
            eprintln!(
                "showcase surface extent drift: viewport={}x{} surface={}x{}",
                viewport.physical_size.width,
                viewport.physical_size.height,
                surface_extent.width,
                surface_extent.height
            );
            return Err(LiveRenderError::Surface(SurfaceStatus::Outdated));
        }
        let width = surface_extent.width;
        let height = surface_extent.height;
        self.renderer.render_to_texture(
            &device_handle.device,
            &device_handle.queue,
            self.toolkit.scene(),
            &self.surface.target_view,
            &RenderParams {
                base_color: VelloColor::from_rgb8(11, 12, 13),
                width,
                height,
                antialiasing_method: live_antialiasing_method(),
            },
        )?;

        let mut surface_is_suboptimal = false;
        let surface_texture = match self.surface.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                surface_is_suboptimal = true;
                texture
            }
            CurrentSurfaceTexture::Timeout => {
                return Err(LiveRenderError::Surface(SurfaceStatus::Timeout));
            }
            CurrentSurfaceTexture::Occluded => {
                return Err(LiveRenderError::Surface(SurfaceStatus::Occluded));
            }
            CurrentSurfaceTexture::Outdated => {
                return Err(LiveRenderError::Surface(SurfaceStatus::Outdated));
            }
            CurrentSurfaceTexture::Lost => {
                return Err(LiveRenderError::Surface(SurfaceStatus::Lost));
            }
            CurrentSurfaceTexture::Validation => {
                return Err(LiveRenderError::Surface(SurfaceStatus::Validation));
            }
        };

        if !blit_extents_match(
            PhysicalSize::new(width, height),
            PhysicalSize::new(
                surface_texture.texture.width(),
                surface_texture.texture.height(),
            ),
        ) {
            return Err(LiveRenderError::Surface(SurfaceStatus::Outdated));
        }

        let view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());
        let mut encoder = device_handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("kinetik-ui-showcase-blit"),
            });
        self.surface.blitter.copy(
            &device_handle.device,
            &mut encoder,
            &self.surface.target_view,
            &view,
        );
        device_handle.queue.submit([encoder.finish()]);
        surface_texture.present();
        if surface_is_suboptimal {
            return Err(LiveRenderError::Surface(SurfaceStatus::Outdated));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum SurfaceStatus {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

#[derive(Debug)]
enum LiveRenderError {
    Render(vello::Error),
    Surface(SurfaceStatus),
}

impl fmt::Display for LiveRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => write!(formatter, "{error}"),
            Self::Surface(status) => write!(formatter, "surface status: {status:?}"),
        }
    }
}

impl std::error::Error for LiveRenderError {}

impl From<vello::Error> for LiveRenderError {
    fn from(error: vello::Error) -> Self {
        Self::Render(error)
    }
}

fn handle_surface_status(
    status: SurfaceStatus,
    renderer: &mut LiveVelloRenderer,
    size: PhysicalSize<u32>,
) {
    if surface_status_forces_reconfigure(status) {
        renderer.reconfigure(size);
    }
    if matches!(status, SurfaceStatus::Validation) {
        eprintln!("surface validation error while acquiring the next frame");
    }
}

fn surface_status_requests_redraw(status: SurfaceStatus) -> bool {
    match status {
        SurfaceStatus::Timeout
        | SurfaceStatus::Outdated
        | SurfaceStatus::Lost
        | SurfaceStatus::Validation => true,
        SurfaceStatus::Occluded => false,
    }
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

fn sanitize_physical_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width.max(MIN_WIDTH), size.height.max(MIN_HEIGHT))
}

fn blit_extents_match(target: PhysicalSize<u32>, surface: PhysicalSize<u32>) -> bool {
    target.width == surface.width && target.height == surface.height
}

fn viewport_surface_extents_match(viewport: ViewportInfo, surface: PhysicalSize<u32>) -> bool {
    let expected = sanitize_physical_size(PhysicalSize::new(
        viewport.physical_size.width,
        viewport.physical_size.height,
    ));
    blit_extents_match(expected, surface)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceResizeMode {
    IfChanged,
    Force,
}

fn surface_resize_required(
    current: PhysicalSize<u32>,
    requested: PhysicalSize<u32>,
    mode: SurfaceResizeMode,
) -> bool {
    mode == SurfaceResizeMode::Force
        || current.width != requested.width
        || current.height != requested.height
}

fn surface_status_forces_reconfigure(status: SurfaceStatus) -> bool {
    matches!(status, SurfaceStatus::Outdated | SurfaceStatus::Lost)
}

pub(crate) fn live_antialiasing_method() -> AaConfig {
    crate::showcase_antialiasing_method()
}

fn live_present_mode() -> PresentMode {
    PresentMode::AutoNoVsync
}

#[cfg(test)]
mod tests {
    use super::{
        LiveShowcase, PresentMode, SurfaceResizeMode, SurfaceStatus, blit_extents_match,
        control_flow_for_repaint_schedule, immediate_redraw_control_flow, live_antialiasing_method,
        live_present_mode, roll_platform_frame, surface_resize_required,
        surface_status_forces_reconfigure, surface_status_requests_redraw,
        viewport_surface_extents_match,
    };
    use kinetik_ui::{
        core::{ClipboardText, ScaleFactor, Size, UiInputEvent, ViewportInfo, WidgetId},
        platform_winit::{
            WinitInputAdapter, WinitRepaintSchedule, WinitShellOutcome, WinitShellResult,
        },
    };
    use std::time::Instant;
    use vello::AaConfig;
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, MouseButton as WinitMouseButton};
    use winit::event_loop::ControlFlow;

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
    fn blit_extents_must_match_target_and_surface() {
        assert!(blit_extents_match(
            PhysicalSize::new(800, 600),
            PhysicalSize::new(800, 600),
        ));
        assert!(!blit_extents_match(
            PhysicalSize::new(800, 600),
            PhysicalSize::new(801, 600),
        ));
        assert!(!blit_extents_match(
            PhysicalSize::new(800, 600),
            PhysicalSize::new(800, 601),
        ));
    }

    #[test]
    fn viewport_surface_extent_drift_is_detected_before_blit() {
        let viewport = ViewportInfo::new(
            Size::new(640.0, 360.0),
            kinetik_ui::core::PhysicalSize::new(960, 540),
            ScaleFactor::new(1.5),
        );

        assert!(viewport_surface_extents_match(
            viewport,
            PhysicalSize::new(960, 540),
        ));
        assert!(!viewport_surface_extents_match(
            viewport,
            PhysicalSize::new(959, 540),
        ));
    }

    #[test]
    fn live_renderer_uses_shared_crisp_showcase_antialiasing() {
        assert_eq!(live_antialiasing_method(), AaConfig::Msaa16);
        assert_eq!(
            live_antialiasing_method(),
            crate::showcase_antialiasing_method()
        );
    }

    #[test]
    fn live_renderer_prefers_low_latency_present_mode() {
        assert_eq!(live_present_mode(), PresentMode::AutoNoVsync);
    }

    #[test]
    fn transient_surface_timeout_requests_another_redraw() {
        assert!(surface_status_requests_redraw(SurfaceStatus::Timeout));
        assert!(!surface_status_requests_redraw(SurfaceStatus::Occluded));
    }

    #[test]
    fn lost_and_outdated_surfaces_force_reconfiguration() {
        assert!(surface_status_forces_reconfigure(SurfaceStatus::Lost));
        assert!(surface_status_forces_reconfigure(SurfaceStatus::Outdated));
        assert!(!surface_status_forces_reconfigure(SurfaceStatus::Timeout));
        assert!(!surface_status_forces_reconfigure(SurfaceStatus::Occluded));
        assert!(!surface_status_forces_reconfigure(
            SurfaceStatus::Validation
        ));
    }

    #[test]
    fn forced_surface_resize_reconfigures_even_when_size_matches() {
        let current = PhysicalSize::new(800, 600);

        assert!(!surface_resize_required(
            current,
            PhysicalSize::new(800, 600),
            SurfaceResizeMode::IfChanged,
        ));
        assert!(surface_resize_required(
            current,
            PhysicalSize::new(801, 600),
            SurfaceResizeMode::IfChanged,
        ));
        assert!(surface_resize_required(
            current,
            PhysicalSize::new(800, 600),
            SurfaceResizeMode::Force,
        ));
    }

    #[test]
    fn resume_clears_stale_input_edges_before_new_window_events() {
        let mut app = LiveShowcase::new(None);
        app.input
            .mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);

        app.reset_resume_input_state();

        assert!(!app.input.input().pointer.primary.pressed);
        assert!(!app.input.input().pointer.primary.down);
        assert!(!app.accepting_input);
    }

    #[test]
    fn first_redraw_snapshot_preserves_input_edges_recorded_after_resume() {
        let mut app = LiveShowcase::new(None);
        app.reset_resume_input_state();
        app.input
            .mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);

        let input = app.frame_input_snapshot(1.5);

        assert!(input.pointer.primary.pressed);
        assert!(input.pointer.primary.down);
    }

    #[test]
    fn recoverable_frame_roll_clears_old_edges_and_preserves_shell_response() {
        assert!(surface_status_requests_redraw(SurfaceStatus::Timeout));
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
