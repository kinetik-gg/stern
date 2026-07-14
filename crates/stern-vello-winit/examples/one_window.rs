//! Minimal application-owned Winit loop with a native GPU texture producer.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use stern_core::{
    FrameContext, Primitive, Rect, Size, TextureId, TexturePrimitive, TimeInfo, Ui, UiMemory,
    ViewportInfo,
};
use stern_render::{RenderFrameInput, RenderImageSampling, RenderResources, TextureResource};
use stern_vello_winit::{
    PresenterDeviceScope, VelloNativeTextureRegistration, VelloPresentStatus, VelloPresenterConfig,
    VelloPresenterError, VelloRecoveryKind, VelloRedrawGuidance, VelloResizeOutcome,
    VelloWindowPresenter, wgpu,
};
use stern_winit::{WinitInputAdapter, scale_factor_from_winit, viewport_from_winit};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const PRODUCER_TEXTURE_ID: TextureId = TextureId::from_raw(1);
const PRODUCER_EXTENT: u32 = 256;
const PRODUCER_LOGICAL_EXTENT: f32 = 256.0;
const PRODUCER_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
enum WindowMetricsWork {
    ResizePresenterForRedraw(PhysicalSize<u32>),
    AwaitAuthoritativeResized,
}

#[derive(Debug, Default)]
struct WindowMetricsCoordinator {
    viewport: Option<ViewportInfo>,
}

impl WindowMetricsCoordinator {
    const fn viewport(&self) -> Option<ViewportInfo> {
        self.viewport
    }

    fn authoritative_resized(
        &mut self,
        input: &mut WinitInputAdapter,
        physical_size: PhysicalSize<u32>,
        scale_factor: f64,
    ) -> WindowMetricsWork {
        let viewport = viewport_from_winit(physical_size, scale_factor);
        input.set_scale_factor(viewport.scale_factor);
        self.viewport = Some(viewport);
        WindowMetricsWork::ResizePresenterForRedraw(physical_size)
    }

    fn scale_factor_changed(
        &mut self,
        input: &mut WinitInputAdapter,
        scale_factor: f64,
        mut request_inner_size: impl FnMut(PhysicalSize<u32>) -> bool,
    ) -> WindowMetricsWork {
        let scale_factor = scale_factor_from_winit(scale_factor);
        input.set_scale_factor(scale_factor);

        let Some(previous) = self.viewport else {
            return WindowMetricsWork::AwaitAuthoritativeResized;
        };
        let selected_size = LogicalSize::<f64>::new(
            f64::from(previous.logical_size.width),
            f64::from(previous.logical_size.height),
        )
        .to_physical::<u32>(scale_factor.value());
        if !request_inner_size(selected_size) {
            self.viewport = None;
            return WindowMetricsWork::AwaitAuthoritativeResized;
        }

        let viewport = viewport_from_winit(selected_size, scale_factor.value());
        self.viewport = Some(viewport);
        WindowMetricsWork::ResizePresenterForRedraw(selected_size)
    }

    fn suspend(&mut self) {
        self.viewport = None;
    }
}

struct NativeProducer {
    scope: PresenterDeviceScope,
    texture: wgpu::Texture,
    registration: VelloNativeTextureRegistration,
    revision: u64,
}

struct OneWindowApp {
    presenter: VelloWindowPresenter,
    window: Option<Arc<Window>>,
    input: WinitInputAdapter,
    memory: UiMemory,
    metrics: WindowMetricsCoordinator,
    resources: RenderResources,
    producer: Option<NativeProducer>,
}

impl OneWindowApp {
    fn new() -> Result<Self, VelloPresenterError> {
        let mut resources = RenderResources::new();
        resources.register_texture(producer_resource());
        Ok(Self {
            presenter: VelloWindowPresenter::new(VelloPresenterConfig::new())?,
            window: None,
            input: WinitInputAdapter::default(),
            memory: UiMemory::new(),
            metrics: WindowMetricsCoordinator::default(),
            resources,
            producer: None,
        })
    }

    fn handle_authoritative_window_metrics(
        &mut self,
        window: &Window,
        raw_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        let work = self
            .metrics
            .authoritative_resized(&mut self.input, raw_size, scale_factor);
        self.handle_window_metrics_work(window, work);
    }

    fn handle_window_metrics_work(&mut self, window: &Window, work: WindowMetricsWork) {
        let WindowMetricsWork::ResizePresenterForRedraw(raw_size) = work else {
            return;
        };
        match self.presenter.resize(raw_size) {
            Ok(VelloResizeOutcome::RecoveryRequired(
                VelloRecoveryKind::CreateSurface
                | VelloRecoveryKind::RecreateSurface
                | VelloRecoveryKind::RebuildDevice,
            )) => self.recover(window),
            Ok(VelloResizeOutcome::ZeroSized | VelloResizeOutcome::Detached) => {}
            Ok(_) => window.request_redraw(),
            Err(error) => eprintln!("presenter resize failed: {error}"),
        }
    }

    fn recover(&mut self, window: &Window) {
        match pollster::block_on(self.presenter.recover()) {
            Ok(_) => window.request_redraw(),
            Err(error) => eprintln!("presenter recovery failed: {error}"),
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop, window: &Arc<Window>) {
        // Any delivered redraw supersedes an older timeout; only a new
        // `Later` result below may arm the next one-shot deadline.
        event_loop.set_control_flow(ControlFlow::Wait);
        let Some(viewport) = self.metrics.viewport() else {
            return;
        };
        let raw_size = winit::dpi::PhysicalSize::new(
            viewport.physical_size.width,
            viewport.physical_size.height,
        );
        match self.presenter.resize(raw_size) {
            Ok(VelloResizeOutcome::RecoveryRequired(_)) => {
                self.recover(window);
                return;
            }
            Ok(VelloResizeOutcome::ZeroSized | VelloResizeOutcome::Detached) => {
                event_loop.set_control_flow(ControlFlow::Wait);
                return;
            }
            Ok(VelloResizeOutcome::Unchanged | VelloResizeOutcome::Resized) => {}
            Ok(_) => return,
            Err(error) => {
                eprintln!("presenter resize failed: {error}");
                return;
            }
        }

        let logical_size = viewport.logical_size;
        let scope = match self.presenter.device_scope() {
            Ok(Some(scope)) => scope,
            Ok(None) => {
                self.recover(window);
                return;
            }
            Err(error) => {
                eprintln!("presenter device access failed: {error}");
                return;
            }
        };
        if let Err(error) = self.advance_producer(&scope) {
            eprintln!("native texture producer failed: {error}");
            return;
        }
        let context = FrameContext::new(viewport, self.input.input().clone(), TimeInfo::default());
        let mut ui = Ui::begin_frame(context, &mut self.memory);
        ui.push_primitive(producer_primitive(logical_size));
        let output = ui.end_frame();
        self.input.begin_frame();
        let report = match self.presenter.present(RenderFrameInput {
            viewport,
            primitives: &output.primitives,
            resources: &self.resources,
        }) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("presenter frame failed: {error}");
                return;
            }
        };

        if matches!(
            report.status(),
            VelloPresentStatus::SurfaceLost
                | VelloPresentStatus::SurfaceRecoveryRequired
                | VelloPresentStatus::DeviceRecoveryRequired
        ) {
            self.recover(window);
            return;
        }
        match report.redraw() {
            VelloRedrawGuidance::NextFrame => {
                window.request_redraw();
            }
            VelloRedrawGuidance::Later(delay) => {
                event_loop
                    .set_control_flow(ControlFlow::wait_duration(delay.min(PRODUCER_INTERVAL)));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::wait_duration(PRODUCER_INTERVAL));
            }
        }
    }

    fn advance_producer(
        &mut self,
        current_scope: &PresenterDeviceScope,
    ) -> Result<(), VelloPresenterError> {
        if self
            .producer
            .as_ref()
            .is_some_and(|producer| producer.scope != *current_scope)
        {
            self.producer = None;
        }

        if self.producer.is_none() {
            let revision = 1;
            let texture = self
                .presenter
                .with_device(current_scope, |presenter_device| {
                    let texture = presenter_device
                        .device()
                        .create_texture(&producer_texture_descriptor());
                    populate_producer_texture(
                        presenter_device.device(),
                        presenter_device.queue(),
                        &texture,
                        revision,
                    );
                    texture
                })?;
            let registration = self.presenter.register_native_texture(
                current_scope,
                &producer_resource(),
                &texture,
                revision,
            )?;
            self.producer = Some(NativeProducer {
                scope: current_scope.clone(),
                texture,
                registration,
                revision,
            });
            return Ok(());
        }

        let producer = self.producer.as_ref().expect("producer was initialized");
        let next_revision = producer.revision.saturating_add(1);
        self.presenter
            .with_device(current_scope, |presenter_device| {
                populate_producer_texture(
                    presenter_device.device(),
                    presenter_device.queue(),
                    &producer.texture,
                    next_revision,
                );
            })?;
        let _ = self
            .presenter
            .update_native_texture(&producer.registration, next_revision)?;
        self.producer
            .as_mut()
            .expect("producer was initialized")
            .revision = next_revision;
        Ok(())
    }
}

fn producer_resource() -> TextureResource {
    TextureResource {
        id: PRODUCER_TEXTURE_ID,
        size: Size::new(PRODUCER_LOGICAL_EXTENT, PRODUCER_LOGICAL_EXTENT),
        sampling: RenderImageSampling::Pixelated,
        snapshot: None,
    }
}

fn producer_texture_descriptor() -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("stern-one-window-producer"),
        size: wgpu::Extent3d {
            width: PRODUCER_EXTENT,
            height: PRODUCER_EXTENT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }
}

fn populate_producer_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    revision: u64,
) {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("stern-one-window-producer-encoder"),
    });
    let attachments = [Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(producer_color(revision)),
            store: wgpu::StoreOp::Store,
        },
    })];
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("stern-one-window-producer-clear"),
            color_attachments: &attachments,
            ..Default::default()
        });
    }
    queue.submit([encoder.finish()]);
}

fn producer_color(revision: u64) -> wgpu::Color {
    match revision % 4 {
        0 => wgpu::Color {
            r: 0.95,
            g: 0.22,
            b: 0.35,
            a: 1.0,
        },
        1 => wgpu::Color {
            r: 0.16,
            g: 0.52,
            b: 0.96,
            a: 1.0,
        },
        2 => wgpu::Color {
            r: 0.16,
            g: 0.78,
            b: 0.48,
            a: 1.0,
        },
        _ => wgpu::Color {
            r: 0.96,
            g: 0.68,
            b: 0.16,
            a: 1.0,
        },
    }
}

fn producer_primitive(logical_size: Size) -> Primitive {
    let inset_x = if logical_size.width > 48.0 { 24.0 } else { 0.0 };
    let inset_y = if logical_size.height > 48.0 {
        24.0
    } else {
        0.0
    };
    Primitive::Texture(TexturePrimitive {
        texture: PRODUCER_TEXTURE_ID,
        rect: Rect::new(
            inset_x,
            inset_y,
            (logical_size.width - inset_x * 2.0).max(1.0),
            (logical_size.height - inset_y * 2.0).max(1.0),
        ),
        source_size: Size::new(PRODUCER_LOGICAL_EXTENT, PRODUCER_LOGICAL_EXTENT),
    })
}

impl ApplicationHandler for OneWindowApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = if let Some(window) = self.window.clone() {
            window
        } else {
            let window = match event_loop.create_window(
                Window::default_attributes().with_title("Stern Vello presenter example"),
            ) {
                Ok(window) => Arc::new(window),
                Err(error) => {
                    eprintln!("window creation failed: {error}");
                    event_loop.exit();
                    return;
                }
            };
            self.window = Some(Arc::clone(&window));
            window
        };

        match pollster::block_on(self.presenter.resume(Arc::clone(&window))) {
            Ok(_) => {
                let raw_size = window.inner_size();
                self.handle_authoritative_window_metrics(&window, raw_size, window.scale_factor());
            }
            Err(error) => {
                eprintln!("presenter resume failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.presenter.suspend();
        self.metrics.suspend();
        self.window = None;
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
            WindowEvent::Resized(size) => {
                if let Some(window) = self.window.clone() {
                    self.handle_authoritative_window_metrics(&window, size, window.scale_factor());
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                mut inner_size_writer,
            } => {
                let work = self.metrics.scale_factor_changed(
                    &mut self.input,
                    scale_factor,
                    |selected_size| match inner_size_writer.request_inner_size(selected_size) {
                        Ok(()) => true,
                        Err(error) => {
                            eprintln!("scale-change size request was ignored: {error}");
                            false
                        }
                    },
                );
                if let Some(window) = self.window.clone() {
                    self.handle_window_metrics_work(&window, work);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.pointer_moved(position);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.input.pointer_left();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.mouse_button_at(button, state, Instant::now());
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.mouse_wheel(delta);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                self.input.set_window_focused(focused);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = self.window.clone() {
                    self.redraw(event_loop, &window);
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = OneWindowApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{WindowMetricsCoordinator, WindowMetricsWork};
    use stern_core::{Point, ScaleFactor, Vec2};
    use stern_winit::WinitInputAdapter;
    use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};

    #[test]
    fn coordinator_requests_and_stores_exact_four_scale_viewports() {
        let mut input = WinitInputAdapter::new(ScaleFactor::new(0.8));
        let mut metrics = WindowMetricsCoordinator::default();
        assert_eq!(
            metrics.authoritative_resized(&mut input, PhysicalSize::new(640, 480), 0.8,),
            WindowMetricsWork::ResizePresenterForRedraw(PhysicalSize::new(640, 480))
        );
        input.pointer_moved(PhysicalPosition::new(24.0, 32.0));

        for scale in [1.0, 1.25, 1.5, 2.0] {
            input.begin_frame();
            let expected_size = LogicalSize::<f64>::new(800.0, 600.0).to_physical::<u32>(scale);
            let mut requested_size = None;
            let work = metrics.scale_factor_changed(&mut input, scale, |selected_size| {
                requested_size = Some(selected_size);
                true
            });

            assert_eq!(requested_size, Some(expected_size));
            assert_eq!(
                work,
                WindowMetricsWork::ResizePresenterForRedraw(expected_size)
            );
            let viewport = metrics.viewport().expect("accepted viewport is stored");
            assert_eq!(viewport.logical_size, stern_core::Size::new(800.0, 600.0));
            assert_eq!(viewport.physical_size.width, expected_size.width);
            assert_eq!(viewport.physical_size.height, expected_size.height);
            assert_eq!(viewport.scale_factor, ScaleFactor::new(scale));
            assert_eq!(input.input().pointer.position, None);
            assert_eq!(input.input().pointer.delta, Vec2::ZERO);

            input.pointer_moved(PhysicalPosition::new(30.0 * scale, 40.0 * scale));
            assert_eq!(input.input().pointer.position, Some(Point::new(30.0, 40.0)));
            assert_eq!(input.input().pointer.delta, Vec2::ZERO);
        }
    }

    #[test]
    fn coordinator_waits_for_resized_when_size_selection_is_unavailable() {
        let mut input = WinitInputAdapter::new(ScaleFactor::new(0.8));
        input.pointer_moved(PhysicalPosition::new(24.0, 32.0));
        let mut metrics = WindowMetricsCoordinator::default();
        let mut requested = false;

        assert_eq!(
            metrics.scale_factor_changed(&mut input, 1.25, |_| {
                requested = true;
                true
            }),
            WindowMetricsWork::AwaitAuthoritativeResized
        );
        assert!(!requested);
        assert_eq!(metrics.viewport(), None);
        assert_eq!(input.input().pointer.position, None);
        input.pointer_moved(PhysicalPosition::new(25.0, 50.0));
        assert_eq!(input.input().pointer.position, Some(Point::new(20.0, 40.0)));

        let _ = metrics.authoritative_resized(&mut input, PhysicalSize::new(1000, 750), 1.25);
        input.begin_frame();
        input.pointer_moved(PhysicalPosition::new(37.5, 60.0));
        let mut rejected_size = None;
        assert_eq!(
            metrics.scale_factor_changed(&mut input, 1.5, |selected_size| {
                rejected_size = Some(selected_size);
                false
            }),
            WindowMetricsWork::AwaitAuthoritativeResized
        );
        assert_eq!(rejected_size, Some(PhysicalSize::new(1200, 900)));
        assert_eq!(metrics.viewport(), None);
        assert_eq!(input.input().pointer.position, None);

        assert_eq!(
            metrics.authoritative_resized(&mut input, PhysicalSize::new(1200, 900), 1.5,),
            WindowMetricsWork::ResizePresenterForRedraw(PhysicalSize::new(1200, 900))
        );
        assert_eq!(
            metrics
                .viewport()
                .expect("Resized restores viewport")
                .logical_size,
            stern_core::Size::new(800.0, 600.0)
        );
    }
}
