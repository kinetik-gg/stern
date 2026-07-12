//! Minimal application-owned Winit loop using the public Vello presenter.

use std::sync::Arc;

use kinetik_ui_core::{PhysicalSize as CorePhysicalSize, ScaleFactor, Size, ViewportInfo};
use kinetik_ui_render::{RenderFrameInput, RenderResources};
use kinetik_ui_vello_winit::{
    VelloPresentStatus, VelloPresenterConfig, VelloRecoveryKind, VelloRedrawGuidance,
    VelloResizeOutcome, VelloWindowPresenter,
};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

struct OneWindowApp {
    presenter: VelloWindowPresenter,
    window: Option<Arc<Window>>,
    resources: RenderResources,
}

impl OneWindowApp {
    fn new() -> Result<Self, kinetik_ui_vello_winit::VelloPresenterError> {
        Ok(Self {
            presenter: VelloWindowPresenter::new(VelloPresenterConfig::new())?,
            window: None,
            resources: RenderResources::new(),
        })
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
        let raw_size = window.inner_size();
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

        let scale = ScaleFactor::new(window.scale_factor());
        let logical_size = logical_size(raw_size, scale);
        let viewport = ViewportInfo::new(
            logical_size,
            CorePhysicalSize::new(raw_size.width, raw_size.height),
            scale,
        );
        let report = match self.presenter.present(RenderFrameInput {
            viewport,
            primitives: &[],
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
                event_loop.set_control_flow(ControlFlow::wait_duration(delay));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn logical_size(raw: winit::dpi::PhysicalSize<u32>, scale: ScaleFactor) -> Size {
    Size::new(
        (f64::from(raw.width) / scale.value()) as f32,
        (f64::from(raw.height) / scale.value()) as f32,
    )
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
                Window::default_attributes().with_title("Kinetik UI Vello presenter example"),
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
            Ok(_) => window.request_redraw(),
            Err(error) => {
                eprintln!("presenter resume failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.presenter.suspend();
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
            WindowEvent::Resized(size) => match self.presenter.resize(size) {
                Ok(VelloResizeOutcome::RecoveryRequired(
                    VelloRecoveryKind::CreateSurface
                    | VelloRecoveryKind::RecreateSurface
                    | VelloRecoveryKind::RebuildDevice,
                )) => {
                    if let Some(window) = self.window.clone() {
                        self.recover(&window);
                    }
                }
                Ok(_) => {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                Err(error) => eprintln!("presenter resize failed: {error}"),
            },
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
