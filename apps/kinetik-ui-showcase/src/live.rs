use std::{
    fmt,
    sync::Arc,
    time::{Duration, Instant},
};

use kinetik_ui::{
    core::{FrameOutput, RepaintRequest, ViewportInfo},
    platform_winit::{
        WinitFrameClock, WinitInputAdapter, WinitPlatformRequests, frame_context_from_winit,
        scale_factor_from_winit,
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
    next_redraw_at: Option<Instant>,
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
            next_redraw_at: None,
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn request_immediate_redraw(&mut self) {
        self.next_redraw_at = Some(Instant::now());
        self.request_redraw();
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
        let repaint = self.app.output().repaint;
        let mut requests = WinitPlatformRequests::from_frame_output(self.app.output());
        requests.repaint = RepaintRequest::None;
        let shell = requests.apply_to_window(&window);

        window.pre_present_notify();
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        renderer.resize(size);
        let retry_surface_redraw = match renderer.render(self.app.output(), &resources, viewport) {
            Ok(()) => {
                self.next_redraw_at = schedule_shell_repaint(
                    event_loop,
                    &window,
                    repaint,
                    shell.repaint_after,
                    shell.continuous_repaint,
                );
                false
            }
            Err(LiveRenderError::Surface(status)) => {
                handle_surface_status(status, renderer, size);
                surface_status_requests_redraw(status)
            }
            Err(error) => {
                eprintln!("showcase render error: {error}");
                event_loop.exit();
                false
            }
        };
        if retry_surface_redraw {
            self.request_immediate_redraw();
        }

        self.input.begin_frame();
        self.accepting_input = true;
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
                self.input.mouse_button(button, state, 1);
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
                self.input.keyboard_event_with_physical_key(
                    &event.logical_key,
                    &event.physical_key,
                    event.state,
                    self.modifiers,
                    event.repeat,
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
        let Some(deadline) = self.next_redraw_at else {
            return;
        };

        if Instant::now() >= deadline {
            self.next_redraw_at = None;
            self.request_redraw();
            event_loop.set_control_flow(immediate_redraw_control_flow());
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
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

fn schedule_shell_repaint(
    event_loop: &ActiveEventLoop,
    window: &Window,
    repaint: RepaintRequest,
    delay: Option<Duration>,
    continuous: bool,
) -> Option<Instant> {
    let schedule = resolve_repaint_schedule(repaint, delay, continuous, Instant::now());
    event_loop.set_control_flow(control_flow_for_repaint_schedule(schedule));
    match schedule {
        RepaintSchedule::Idle => None,
        RepaintSchedule::Immediate => {
            window.request_redraw();
            Some(Instant::now())
        }
        RepaintSchedule::At(deadline) => Some(deadline),
        RepaintSchedule::Continuous => {
            window.request_redraw();
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepaintSchedule {
    Idle,
    Immediate,
    At(Instant),
    Continuous,
}

fn resolve_repaint_schedule(
    repaint: RepaintRequest,
    delay: Option<Duration>,
    continuous: bool,
    now: Instant,
) -> RepaintSchedule {
    if continuous || repaint == RepaintRequest::Continuous {
        return RepaintSchedule::Continuous;
    }
    match repaint {
        RepaintRequest::NextFrame => RepaintSchedule::Immediate,
        RepaintRequest::After(delay) => RepaintSchedule::At(now + delay),
        RepaintRequest::None => delay.map_or(RepaintSchedule::Idle, |delay| {
            RepaintSchedule::At(now + delay)
        }),
        RepaintRequest::Continuous => RepaintSchedule::Continuous,
    }
}

fn control_flow_for_repaint_schedule(schedule: RepaintSchedule) -> ControlFlow {
    match schedule {
        RepaintSchedule::Idle => ControlFlow::Wait,
        RepaintSchedule::Immediate | RepaintSchedule::Continuous => immediate_redraw_control_flow(),
        RepaintSchedule::At(deadline) => ControlFlow::WaitUntil(deadline),
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
        LiveShowcase, PresentMode, RepaintSchedule, SurfaceResizeMode, SurfaceStatus,
        blit_extents_match, control_flow_for_repaint_schedule, immediate_redraw_control_flow,
        live_antialiasing_method, live_present_mode, resolve_repaint_schedule,
        surface_resize_required, surface_status_forces_reconfigure, surface_status_requests_redraw,
        viewport_surface_extents_match,
    };
    use kinetik_ui::core::{RepaintRequest, ScaleFactor, Size, ViewportInfo};
    use std::time::{Duration, Instant};
    use vello::AaConfig;
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, MouseButton as WinitMouseButton};
    use winit::event_loop::ControlFlow;

    #[test]
    fn next_frame_repaint_requests_immediate_redraw() {
        let now = Instant::now();

        assert_eq!(
            resolve_repaint_schedule(RepaintRequest::NextFrame, None, false, now),
            RepaintSchedule::Immediate
        );
    }

    #[test]
    fn delayed_shell_repaint_preserves_deadline() {
        let now = Instant::now();

        assert_eq!(
            resolve_repaint_schedule(
                RepaintRequest::None,
                Some(Duration::from_millis(12)),
                false,
                now,
            ),
            RepaintSchedule::At(now + Duration::from_millis(12))
        );
    }

    #[test]
    fn continuous_shell_repaint_polls() {
        let now = Instant::now();

        assert_eq!(
            resolve_repaint_schedule(
                RepaintRequest::After(Duration::from_secs(1)),
                None,
                true,
                now
            ),
            RepaintSchedule::Continuous
        );
    }

    #[test]
    fn immediate_repaint_keeps_event_loop_polling_until_redraw() {
        let now = Instant::now();

        assert!(matches!(immediate_redraw_control_flow(), ControlFlow::Poll));
        assert!(matches!(
            control_flow_for_repaint_schedule(RepaintSchedule::Immediate),
            ControlFlow::Poll
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(RepaintSchedule::Continuous),
            ControlFlow::Poll
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(RepaintSchedule::Idle),
            ControlFlow::Wait
        ));
        assert!(matches!(
            control_flow_for_repaint_schedule(RepaintSchedule::At(now)),
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
    fn interactive_redraw_replaces_delayed_repaint_with_immediate_deadline() {
        let mut app = LiveShowcase::new(None);
        app.next_redraw_at = Some(Instant::now() + Duration::from_secs(1));

        let before = Instant::now();
        app.request_interactive_redraw();

        let deadline = app.next_redraw_at.expect("interactive redraw deadline");
        assert!(deadline >= before);
        assert!(deadline <= Instant::now());
    }

    #[test]
    fn immediate_redraw_replaces_delayed_repaint_with_fallback_deadline() {
        let mut app = LiveShowcase::new(None);
        app.next_redraw_at = Some(Instant::now() + Duration::from_secs(1));

        let before = Instant::now();
        app.request_immediate_redraw();

        let deadline = app.next_redraw_at.expect("immediate redraw deadline");
        assert!(deadline >= before);
        assert!(deadline <= Instant::now());
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
}
