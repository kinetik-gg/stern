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

    fn resize_renderer(&mut self, size: PhysicalSize<u32>) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(sanitize_physical_size(size));
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let size = sanitize_physical_size(window.inner_size());
        renderer.resize(size);
        let scale_factor = window.scale_factor();
        self.input
            .set_scale_factor(scale_factor_from_winit(scale_factor));
        if !self.accepting_input {
            self.input.begin_frame();
        }
        let time = self.clock.tick(self.started.elapsed());
        let input = self.input.input().clone();
        let context = frame_context_from_winit(size, scale_factor, input, time);
        let viewport = context.viewport;

        self.app.update_with_context(context);
        let resources = self.app.render_resources();
        let repaint = self.app.output().repaint;
        let mut requests = WinitPlatformRequests::from_frame_output(self.app.output());
        requests.repaint = RepaintRequest::None;
        let shell = requests.apply_to_window(window);

        window.pre_present_notify();
        match renderer.render(self.app.output(), &resources, viewport) {
            Ok(()) => {
                self.next_redraw_at = schedule_shell_repaint(
                    event_loop,
                    window,
                    repaint,
                    shell.repaint_after,
                    shell.continuous_repaint,
                );
            }
            Err(LiveRenderError::Surface(status)) => {
                handle_surface_status(status, renderer, size);
                window.request_redraw();
            }
            Err(error) => {
                eprintln!("showcase render error: {error}");
                event_loop.exit();
            }
        }

        self.input.begin_frame();
        self.accepting_input = true;
    }
}

impl ApplicationHandler for LiveShowcase {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.accepting_input = false;
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
                self.request_redraw();
            }
            WindowEvent::Resized(size) => {
                self.resize_renderer(size);
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.input
                    .set_scale_factor(scale_factor_from_winit(scale_factor));
                self.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.pointer_moved(position);
                self.request_redraw();
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.mouse_button(button, state, 1);
                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.mouse_wheel(delta);
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.keyboard_event_with_physical_key(
                    &event.logical_key,
                    &event.physical_key,
                    event.state,
                    self.modifiers,
                    event.repeat,
                );
                self.request_redraw();
            }
            WindowEvent::Ime(event) => {
                self.input.ime_event(event);
                self.request_redraw();
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
            event_loop.set_control_flow(ControlFlow::Wait);
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
            .create_surface(window, size.width, size.height, PresentMode::Fifo)
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
        let size = sanitize_physical_size(size);
        if self.surface.config.width == size.width && self.surface.config.height == size.height {
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
        let output = self.toolkit.submit_frame(RenderFrameInput {
            viewport,
            primitives: &frame.primitives,
            resources,
        });
        if !output.diagnostics.is_empty() {
            eprintln!("showcase renderer diagnostics: {:?}", output.diagnostics);
        }

        let device_handle = &self.context.devices[self.surface.dev_id];
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        debug_assert_eq!(viewport.physical_size.width, width);
        debug_assert_eq!(viewport.physical_size.height, height);
        self.renderer.render_to_texture(
            &device_handle.device,
            &device_handle.queue,
            self.toolkit.scene(),
            &self.surface.target_view,
            &RenderParams {
                base_color: VelloColor::from_rgb8(11, 12, 13),
                width,
                height,
                antialiasing_method: AaConfig::Msaa16,
            },
        )?;

        let mut surface_is_suboptimal = false;
        let surface_texture = match self.surface.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                surface_is_suboptimal = true;
                texture
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return Ok(()),
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
    match status {
        SurfaceStatus::Outdated | SurfaceStatus::Lost => renderer.resize(size),
        SurfaceStatus::Validation => {
            eprintln!("surface validation error while acquiring the next frame");
        }
    }
}

fn schedule_shell_repaint(
    event_loop: &ActiveEventLoop,
    window: &Window,
    repaint: RepaintRequest,
    delay: Option<Duration>,
    continuous: bool,
) -> Option<Instant> {
    if continuous {
        event_loop.set_control_flow(ControlFlow::Poll);
        window.request_redraw();
        return None;
    }

    let mut next_redraw_at = delay.map(|delay| Instant::now() + delay);
    match repaint {
        RepaintRequest::None => {}
        RepaintRequest::NextFrame => {
            next_redraw_at = Some(Instant::now());
        }
        RepaintRequest::After(delay) => {
            next_redraw_at = Some(Instant::now() + delay);
        }
        RepaintRequest::Continuous => {
            event_loop.set_control_flow(ControlFlow::Poll);
            window.request_redraw();
            return None;
        }
    }
    if let Some(deadline) = next_redraw_at {
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    } else {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
    next_redraw_at
}

fn sanitize_physical_size(size: PhysicalSize<u32>) -> PhysicalSize<u32> {
    PhysicalSize::new(size.width.max(MIN_WIDTH), size.height.max(MIN_HEIGHT))
}
