use std::sync::Arc;

use kinetik_ui_render::RenderFrameInput;
use kinetik_ui_vello::VelloRenderer;
use vello::{
    Renderer, RendererOptions,
    util::{RenderContext, RenderSurface},
    wgpu::{
        CommandEncoderDescriptor, CurrentSurfaceTexture, SurfaceTexture, TextureViewDescriptor,
    },
};
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowId},
};

use crate::{
    PresenterDevice, PresenterDeviceScope, VelloAttachOutcome, VelloAttachmentStatus,
    VelloPresentReport, VelloPresentStatus, VelloPresenterConfig, VelloPresenterError,
    VelloPresenterStatus, VelloRecoveryKind, VelloRecoveryOutcome, VelloRedrawGuidance,
    VelloResizeOutcome, VelloSuspendOutcome,
    device::{
        CurrentDeviceEventOutcome, DeviceAuthority, DeviceInbox, classify_current_device_events,
    },
    frame::{
        AcquiredFrame, DriveFailure, DrivenFrame, PresentOperations, drive_present,
        report_for_driven,
    },
    lifecycle::{
        DeviceRecoveryBuild, DeviceRecoveryTeardown, DropAction, Extent, LifecycleState,
        ResizePlan, ResumePlan, drive_device_recovery_build, drive_device_recovery_teardown,
    },
};

struct GpuState {
    renderer: Renderer,
    context: RenderContext,
    dev_id: usize,
}

struct PresenterControl<W> {
    authority: DeviceAuthority,
    lifecycle: LifecycleState<W>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceEventEffect {
    None,
    DropGpuForRecovery,
}

impl<W: Copy + Eq> PresenterControl<W> {
    fn new() -> Result<Self, VelloPresenterError> {
        Ok(Self {
            authority: DeviceAuthority::new()?,
            lifecycle: LifecycleState::new(),
        })
    }

    fn apply_device_events(
        &mut self,
        events: crate::device::CurrentDeviceEvents,
    ) -> Result<DeviceEventEffect, VelloPresenterError> {
        match classify_current_device_events(events) {
            CurrentDeviceEventOutcome::Lost => {
                if self.authority.invalidate()? {
                    self.lifecycle.mark_device_lost();
                    Ok(DeviceEventEffect::DropGpuForRecovery)
                } else {
                    Ok(DeviceEventEffect::None)
                }
            }
            CurrentDeviceEventOutcome::Actionable(error) => Err(error),
            CurrentDeviceEventOutcome::None => Ok(DeviceEventEffect::None),
        }
    }

    fn usable_scope(&self) -> Option<PresenterDeviceScope> {
        if self.lifecycle.attachment_status() == VelloAttachmentStatus::Detached
            || self.lifecycle.recovery().is_some()
        {
            None
        } else {
            self.authority.scope()
        }
    }

    fn with_validated_scope<R>(
        &self,
        scope: &PresenterDeviceScope,
        use_scope: impl FnOnce(PresenterDeviceScope) -> Result<R, VelloPresenterError>,
    ) -> Result<R, VelloPresenterError> {
        self.authority.validate(scope)?;
        if self.lifecycle.attachment_status() == VelloAttachmentStatus::Detached
            || self.lifecycle.recovery().is_some()
        {
            return Err(VelloPresenterError::DeviceUnavailable);
        }
        use_scope(scope.clone())
    }

    fn select_surface_device(
        &mut self,
        current_device: usize,
        selected_device: usize,
    ) -> Result<(bool, PresenterDeviceScope), VelloPresenterError> {
        if current_device == selected_device {
            return Ok((
                false,
                self.authority
                    .scope()
                    .ok_or(VelloPresenterError::DeviceUnavailable)?,
            ));
        }
        Ok((true, self.authority.replace()?))
    }

    fn complete_device_rebuild<T>(
        &mut self,
        extent: Extent,
        candidate: Result<T, VelloPresenterError>,
    ) -> Result<(T, PresenterDeviceScope), VelloPresenterError> {
        let candidate = candidate?;
        let scope = self.authority.activate();
        self.lifecycle.mark_surface_ready(extent);
        Ok((candidate, scope))
    }
}

enum PresentAttempt {
    Driven(DrivenFrame),
    DeviceLost,
}

/// Presenter for one live Vello surface attached to one Winit window.
pub struct VelloWindowPresenter {
    config: VelloPresenterConfig,
    control: PresenterControl<WindowId>,
    surface: Option<RenderSurface<'static>>,
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    inbox: Option<DeviceInbox>,
    toolkit: VelloRenderer,
}

impl VelloWindowPresenter {
    /// Creates a detached presenter without initializing a GPU or surface.
    ///
    /// # Errors
    ///
    /// Returns [`VelloPresenterError::GenerationExhausted`] if a unique opaque
    /// presenter identity cannot be allocated.
    pub fn new(config: VelloPresenterConfig) -> Result<Self, VelloPresenterError> {
        Ok(Self {
            config,
            control: PresenterControl::new()?,
            surface: None,
            window: None,
            gpu: None,
            inbox: None,
            toolkit: VelloRenderer::new(),
        })
    }

    /// Returns the retained configuration used for recovery.
    #[must_use]
    pub const fn config(&self) -> &VelloPresenterConfig {
        &self.config
    }

    /// Returns the attached window, if any.
    #[must_use]
    pub fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }

    /// Returns the exact attached window ID, if any.
    #[must_use]
    pub fn window_id(&self) -> Option<WindowId> {
        self.control.lifecycle.window()
    }

    /// Returns whether the presenter currently owns this exact window.
    #[must_use]
    pub fn accepts_window(&self, window_id: WindowId) -> bool {
        self.control.lifecycle.window() == Some(window_id)
    }

    /// Returns a non-mutating status snapshot.
    ///
    /// Call [`Self::device_scope`] before native work when callback visibility
    /// must be current.
    #[must_use]
    pub fn status(&self) -> VelloPresenterStatus {
        let scope = self.control.usable_scope();
        VelloPresenterStatus::new(
            self.control.lifecycle.attachment_status(),
            self.control.lifecycle.recovery(),
            scope,
        )
    }

    /// Polls callback inboxes and returns the current usable device scope.
    ///
    /// # Errors
    ///
    /// Returns an actionable callback, overflow, or generation error before
    /// exposing a scope.
    pub fn device_scope(&mut self) -> Result<Option<PresenterDeviceScope>, VelloPresenterError> {
        self.poll_device_events()?;
        Ok(self.control.usable_scope())
    }

    /// Borrows the exact current device and queue after validating a scope.
    ///
    /// The closure is never called for a foreign presenter, stale generation,
    /// detached presenter, pending recovery, or callback-reported failure.
    ///
    /// # Errors
    ///
    /// Returns a typed scope, device-availability, callback, overflow, or
    /// generation error.
    pub fn with_device<R>(
        &mut self,
        scope: &PresenterDeviceScope,
        use_device: impl FnOnce(PresenterDevice<'_>) -> R,
    ) -> Result<R, VelloPresenterError> {
        self.poll_device_events()?;
        self.control.with_validated_scope(scope, |validated| {
            let gpu = self
                .gpu
                .as_ref()
                .ok_or(VelloPresenterError::DeviceUnavailable)?;
            let device = &gpu.context.devices[gpu.dev_id];
            Ok(use_device(PresenterDevice::new(
                validated,
                &device.device,
                &device.queue,
            )))
        })
    }

    /// Attaches a Winit window and initializes a non-zero surface as needed.
    ///
    /// Redundant resume of the same window is idempotent. A different window is
    /// rejected while attached.
    ///
    /// # Errors
    ///
    /// Returns a typed window, callback, initialization, recovery, or
    /// generation error.
    pub async fn resume(
        &mut self,
        window: Arc<Window>,
    ) -> Result<VelloAttachOutcome, VelloPresenterError> {
        self.poll_device_events()?;
        let window_id = window.id();
        let extent = Extent::from(window.inner_size());
        match self.control.lifecycle.resume(window_id, extent)? {
            ResumePlan::AlreadyAttached => {
                let _ = self.resize(PhysicalSize::new(extent.width, extent.height))?;
                Ok(VelloAttachOutcome::AlreadyAttached)
            }
            ResumePlan::AttachedZeroSized => {
                self.window = Some(window);
                Ok(VelloAttachOutcome::AttachedZeroSized)
            }
            ResumePlan::Recover(_) => {
                self.window = Some(window);
                let outcome = self.recover().await?;
                let (VelloRecoveryOutcome::SurfaceReady { device_scope, .. }
                | VelloRecoveryOutcome::DeviceRebuilt { device_scope }) = outcome
                else {
                    return Err(VelloPresenterError::DeviceUnavailable);
                };
                Ok(VelloAttachOutcome::AttachedPresentable { device_scope })
            }
        }
    }

    /// Drops the surface before releasing presenter window ownership.
    #[must_use]
    pub fn suspend(&mut self) -> VelloSuspendOutcome {
        let plan = self.control.lifecycle.suspend();
        if plan.actions.is_empty() {
            return VelloSuspendOutcome::AlreadyDetached;
        }
        for action in plan.actions {
            match action {
                DropAction::Surface => {
                    self.surface.take();
                }
                DropAction::Window => {
                    debug_assert!(self.surface.is_none());
                    self.window.take();
                }
            }
        }
        VelloSuspendOutcome::Suspended
    }

    /// Applies one raw physical window extent without fabricating a 1x1 size.
    ///
    /// # Errors
    ///
    /// Returns callback, overflow, recovery, or generation errors before GPU
    /// configuration.
    pub fn resize(
        &mut self,
        size: PhysicalSize<u32>,
    ) -> Result<VelloResizeOutcome, VelloPresenterError> {
        self.poll_device_events()?;
        match self.control.lifecycle.resize(Extent::from(size)) {
            ResizePlan::Outcome(outcome) => Ok(outcome),
            ResizePlan::ZeroSized { drop_surface } => {
                if drop_surface {
                    self.surface.take();
                }
                Ok(VelloResizeOutcome::ZeroSized)
            }
            ResizePlan::Configure { extent, force } => {
                self.configure_existing_surface(extent, force)?;
                Ok(VelloResizeOutcome::Resized)
            }
        }
    }

    /// Creates/recreates the surface or rebuilds the complete device path once.
    ///
    /// # Errors
    ///
    /// Returns callback, initialization, recovery, or generation errors. A
    /// failed attempt remains pending and never exposes old native handles.
    pub async fn recover(&mut self) -> Result<VelloRecoveryOutcome, VelloPresenterError> {
        self.poll_device_events()?;
        let Some(kind) = self.control.lifecycle.recovery() else {
            return Ok(VelloRecoveryOutcome::NotNeeded);
        };
        let Some(window) = self.window.clone() else {
            if kind == VelloRecoveryKind::RebuildDevice {
                self.teardown_device_for_rebuild();
            }
            return Ok(VelloRecoveryOutcome::DeferredDetached(kind));
        };
        let extent = self.control.lifecycle.desired();
        if extent.is_zero() {
            if kind == VelloRecoveryKind::RebuildDevice {
                self.teardown_device_for_rebuild();
            }
            return Ok(VelloRecoveryOutcome::DeferredZeroSized(kind));
        }

        if kind == VelloRecoveryKind::RebuildDevice {
            self.teardown_device_for_rebuild();
            let (gpu, surface, inbox, scope) =
                self.create_fresh_gpu_surface(window, extent, true).await?;
            self.gpu = Some(gpu);
            self.surface = Some(surface);
            self.inbox = Some(inbox);
            self.control.lifecycle.mark_surface_ready(extent);
            return Ok(VelloRecoveryOutcome::DeviceRebuilt {
                device_scope: scope,
            });
        }

        self.surface.take();
        if self.gpu.is_none() {
            let (gpu, surface, inbox, scope) =
                self.create_fresh_gpu_surface(window, extent, false).await?;
            self.gpu = Some(gpu);
            self.surface = Some(surface);
            self.inbox = Some(inbox);
            self.control.lifecycle.mark_surface_ready(extent);
            return Ok(VelloRecoveryOutcome::SurfaceReady {
                device_changed: false,
                device_scope: scope,
            });
        }

        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let surface = gpu
            .context
            .create_surface(
                window,
                extent.width,
                extent.height,
                self.config.present_mode(),
            )
            .await
            .map_err(VelloPresenterError::recovery)?;
        let changed = surface.dev_id != gpu.dev_id;
        let scope = if changed {
            let device = &gpu.context.devices[surface.dev_id].device;
            let renderer = Renderer::new(device, RendererOptions::default())
                .map_err(VelloPresenterError::recovery)?;
            let (_, scope) = self
                .control
                .select_surface_device(gpu.dev_id, surface.dev_id)?;
            let inbox = DeviceInbox::install(device, scope.clone());
            gpu.renderer = renderer;
            gpu.dev_id = surface.dev_id;
            self.inbox = Some(inbox);
            scope
        } else {
            self.control
                .select_surface_device(gpu.dev_id, surface.dev_id)?
                .1
        };
        self.surface = Some(surface);
        self.control.lifecycle.mark_surface_ready(extent);
        Ok(VelloRecoveryOutcome::SurfaceReady {
            device_changed: changed,
            device_scope: scope,
        })
    }

    /// Performs one synchronous acquire/render/blit/notify/present attempt.
    ///
    /// # Errors
    ///
    /// Returns actionable validation, Vello render, callback, overflow, device,
    /// or generation failures. It never retries acquisition in the same call.
    pub fn present(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<VelloPresentReport, VelloPresenterError> {
        if let Some(report) = self.preflight_present()? {
            return Ok(report);
        }

        let configured = self
            .control
            .lifecycle
            .configured()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        match self.run_present_attempt(input, configured)? {
            PresentAttempt::Driven(driven) => {
                match &driven {
                    DrivenFrame::Presented {
                        suboptimal: true, ..
                    } => self.control.lifecycle.mark_reconfigure(),
                    DrivenFrame::AcquiredExtentOutdated | DrivenFrame::Outdated => {
                        self.control.lifecycle.mark_surface_ready(configured);
                    }
                    DrivenFrame::Lost => {
                        self.surface.take();
                        self.control.lifecycle.mark_surface_lost();
                    }
                    _ => {}
                }
                report_for_driven(driven, self.config.timeout_retry())
            }
            PresentAttempt::DeviceLost => Ok(VelloPresentReport::new(
                VelloPresentStatus::DeviceRecoveryRequired,
                VelloRedrawGuidance::None,
                None,
            )),
        }
    }

    fn preflight_present(&mut self) -> Result<Option<VelloPresentReport>, VelloPresenterError> {
        self.poll_device_events()?;
        if self.control.lifecycle.attachment_status() == VelloAttachmentStatus::Detached {
            return Ok(Some(VelloPresentReport::new(
                VelloPresentStatus::Detached,
                VelloRedrawGuidance::ExternalEvent,
                None,
            )));
        }
        let raw_size = self
            .window
            .as_ref()
            .ok_or(VelloPresenterError::DeviceUnavailable)?
            .inner_size();
        let report = match self.resize(raw_size)? {
            VelloResizeOutcome::ZeroSized => Some(VelloPresentReport::new(
                VelloPresentStatus::ZeroSized,
                VelloRedrawGuidance::NonZeroResize,
                None,
            )),
            VelloResizeOutcome::RecoveryRequired(VelloRecoveryKind::RebuildDevice) => {
                Some(VelloPresentReport::new(
                    VelloPresentStatus::DeviceRecoveryRequired,
                    VelloRedrawGuidance::None,
                    None,
                ))
            }
            VelloResizeOutcome::RecoveryRequired(_) => Some(VelloPresentReport::new(
                VelloPresentStatus::SurfaceRecoveryRequired,
                VelloRedrawGuidance::NextFrame,
                None,
            )),
            VelloResizeOutcome::Detached => Some(VelloPresentReport::new(
                VelloPresentStatus::Detached,
                VelloRedrawGuidance::ExternalEvent,
                None,
            )),
            VelloResizeOutcome::Unchanged | VelloResizeOutcome::Resized => None,
        };
        Ok(report)
    }

    fn run_present_attempt(
        &mut self,
        input: RenderFrameInput<'_>,
        configured: Extent,
    ) -> Result<PresentAttempt, VelloPresenterError> {
        let current_scope = self
            .control
            .authority
            .scope()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let surface = self
            .surface
            .as_ref()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let inbox = self
            .inbox
            .as_ref()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let window = self
            .window
            .as_ref()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let driven = {
            let mut operations = RealPresentOperations {
                surface,
                gpu,
                toolkit: &mut self.toolkit,
                window,
                config: &self.config,
                current_scope,
                inbox,
            };
            drive_present(&mut operations, input, configured)
        };
        match driven {
            Ok(driven) => Ok(PresentAttempt::Driven(driven)),
            Err(DriveFailure::DeviceLostAfterRender) => {
                self.transition_device_loss()?;
                Ok(PresentAttempt::DeviceLost)
            }
            Err(DriveFailure::Render(error)) => Err(VelloPresenterError::render(error)),
            Err(DriveFailure::Actionable(error)) => Err(error),
        }
    }

    async fn create_fresh_gpu_surface(
        &mut self,
        window: Arc<Window>,
        extent: Extent,
        rebuilding: bool,
    ) -> Result<
        (
            GpuState,
            RenderSurface<'static>,
            DeviceInbox,
            PresenterDeviceScope,
        ),
        VelloPresenterError,
    > {
        let mut build = RealDeviceRecoveryBuild {
            window,
            extent,
            present_mode: self.config.present_mode(),
            rebuilding,
        };
        let candidate = drive_device_recovery_build(&mut build).await;
        let (artifacts, scope) = if rebuilding {
            self.control.complete_device_rebuild(extent, candidate)?
        } else {
            let artifacts = candidate?;
            let scope = if self.control.authority.scope().is_some() {
                self.control.authority.replace()?
            } else {
                self.control.authority.activate()
            };
            (artifacts, scope)
        };
        let context = artifacts.context;
        let surface = artifacts.surface;
        let renderer = artifacts.renderer;
        let dev_id = artifacts.device_id;
        let device = &context.devices[dev_id].device;
        let inbox = DeviceInbox::install(device, scope.clone());
        Ok((
            GpuState {
                renderer,
                context,
                dev_id,
            },
            surface,
            inbox,
            scope,
        ))
    }

    fn configure_existing_surface(
        &mut self,
        extent: Extent,
        force: bool,
    ) -> Result<(), VelloPresenterError> {
        if extent.is_zero() {
            return Err(VelloPresenterError::Validation {
                message: "zero-sized surfaces are never configured".into(),
            });
        }
        let gpu = self
            .gpu
            .as_ref()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let surface = self
            .surface
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let current = Extent {
            width: surface.config.width,
            height: surface.config.height,
        };
        if current != extent {
            gpu.context
                .resize_surface(surface, extent.width, extent.height);
        } else if force {
            gpu.context.configure_surface(surface);
        }
        self.control.lifecycle.mark_surface_ready(extent);
        Ok(())
    }

    fn poll_device_events(&mut self) -> Result<bool, VelloPresenterError> {
        let Some(current) = self.control.authority.scope() else {
            return Ok(false);
        };
        let Some(inbox) = self.inbox.as_ref() else {
            return Ok(false);
        };
        let events = inbox.drain_current(&current);
        if self.control.apply_device_events(events)? == DeviceEventEffect::DropGpuForRecovery {
            self.teardown_device_for_rebuild();
            return Ok(true);
        }
        Ok(false)
    }

    fn transition_device_loss(&mut self) -> Result<(), VelloPresenterError> {
        let events = crate::device::CurrentDeviceEvents {
            lost: true,
            ..crate::device::CurrentDeviceEvents::default()
        };
        if self.control.apply_device_events(events)? == DeviceEventEffect::DropGpuForRecovery {
            self.teardown_device_for_rebuild();
        }
        Ok(())
    }

    fn teardown_device_for_rebuild(&mut self) {
        let (renderer, context) = match self.gpu.take() {
            Some(GpuState {
                renderer,
                context,
                dev_id: _,
            }) => (Some(renderer), Some(context)),
            None => (None, None),
        };
        let mut teardown = RealDeviceRecoveryTeardown {
            surface: self.surface.take(),
            inbox: self.inbox.take(),
            renderer,
            context,
        };
        drive_device_recovery_teardown(&mut teardown);
    }

    #[cfg(test)]
    pub(crate) fn install_test_device(
        &mut self,
        extent: Extent,
    ) -> Result<(PresenterDeviceScope, crate::device::DeviceEventSender), VelloPresenterError> {
        let _ = self.control.lifecycle.resume(WindowId::dummy(), extent)?;
        self.control.lifecycle.mark_surface_ready(extent);
        let scope = self.control.authority.activate();
        let (inbox, sender) = DeviceInbox::for_test(scope.clone());
        self.inbox = Some(inbox);
        Ok((scope, sender))
    }

    #[cfg(test)]
    pub(crate) fn select_test_surface_device(
        &mut self,
        current_device: usize,
        selected_device: usize,
    ) -> Result<(bool, PresenterDeviceScope), VelloPresenterError> {
        self.control
            .select_surface_device(current_device, selected_device)
    }

    #[cfg(test)]
    pub(crate) fn fail_test_device_rebuild(
        &mut self,
        extent: Extent,
        error: VelloPresenterError,
    ) -> Result<(), VelloPresenterError> {
        self.control
            .complete_device_rebuild(extent, Err::<(), _>(error))
            .map(|_| ())
    }
}

struct RealDeviceRecoveryTeardown {
    surface: Option<RenderSurface<'static>>,
    inbox: Option<DeviceInbox>,
    renderer: Option<Renderer>,
    context: Option<RenderContext>,
}

impl DeviceRecoveryTeardown for RealDeviceRecoveryTeardown {
    fn drop_surface(&mut self) {
        drop(self.surface.take());
    }

    fn drop_renderer(&mut self) {
        drop(self.inbox.take());
        drop(self.renderer.take());
    }

    fn drop_context(&mut self) {
        drop(self.context.take());
    }
}

struct RealDeviceRecoveryBuild {
    window: Arc<Window>,
    extent: Extent,
    present_mode: vello::wgpu::PresentMode,
    rebuilding: bool,
}

impl RealDeviceRecoveryBuild {
    fn map_error(&self, error: impl std::fmt::Display) -> VelloPresenterError {
        if self.rebuilding {
            VelloPresenterError::recovery(error)
        } else {
            VelloPresenterError::initialization(error)
        }
    }
}

impl DeviceRecoveryBuild for RealDeviceRecoveryBuild {
    type Context = RenderContext;
    type RawSurface = vello::wgpu::Surface<'static>;
    type Surface = RenderSurface<'static>;
    type Renderer = Renderer;
    type Error = VelloPresenterError;

    fn create_context(&mut self) -> Self::Context {
        RenderContext::new()
    }

    fn create_raw_surface(
        &mut self,
        context: &Self::Context,
    ) -> Result<Self::RawSurface, Self::Error> {
        context
            .instance
            .create_surface(Arc::clone(&self.window))
            .map_err(|error| self.map_error(error))
    }

    async fn select_device_queue<'a>(
        &'a mut self,
        context: &'a mut Self::Context,
        surface: &'a Self::RawSurface,
    ) -> Result<usize, Self::Error> {
        context
            .device(Some(surface))
            .await
            .ok_or_else(|| self.map_error("no compatible GPU device or queue"))
    }

    fn create_renderer(
        &mut self,
        context: &Self::Context,
        device_id: usize,
    ) -> Result<Self::Renderer, Self::Error> {
        Renderer::new(
            &context.devices[device_id].device,
            RendererOptions::default(),
        )
        .map_err(|error| self.map_error(error))
    }

    async fn create_configured_surface<'a>(
        &'a mut self,
        context: &'a mut Self::Context,
        surface: Self::RawSurface,
    ) -> Result<Self::Surface, Self::Error> {
        context
            .create_render_surface(
                surface,
                self.extent.width,
                self.extent.height,
                self.present_mode,
            )
            .await
            .map_err(|error| self.map_error(error))
    }

    fn surface_device_id(&self, surface: &Self::Surface) -> usize {
        surface.dev_id
    }

    fn device_mismatch_error(&self, selected: usize, configured: usize) -> Self::Error {
        self.map_error(format_args!(
            "fresh surface selected device slot {selected} but configured slot {configured}"
        ))
    }
}

struct RealPresentOperations<'a> {
    surface: &'a RenderSurface<'static>,
    gpu: &'a mut GpuState,
    toolkit: &'a mut VelloRenderer,
    window: &'a Arc<Window>,
    config: &'a VelloPresenterConfig,
    current_scope: PresenterDeviceScope,
    inbox: &'a DeviceInbox,
}

impl PresentOperations for RealPresentOperations<'_> {
    type Frame = SurfaceTexture;
    type RenderError = vello::Error;

    fn acquire(&mut self) -> AcquiredFrame<Self::Frame> {
        match self.surface.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => AcquiredFrame::Success(frame),
            CurrentSurfaceTexture::Suboptimal(frame) => AcquiredFrame::Suboptimal(frame),
            CurrentSurfaceTexture::Timeout => AcquiredFrame::Timeout,
            CurrentSurfaceTexture::Occluded => AcquiredFrame::Occluded,
            CurrentSurfaceTexture::Outdated => AcquiredFrame::Outdated,
            CurrentSurfaceTexture::Lost => AcquiredFrame::Lost,
            CurrentSurfaceTexture::Validation => AcquiredFrame::Validation,
        }
    }

    fn acquired_extent(&mut self, frame: &Self::Frame) -> Extent {
        Extent {
            width: frame.texture.width(),
            height: frame.texture.height(),
        }
    }

    fn drop_frame(&mut self, frame: Self::Frame) {
        drop(frame);
    }

    fn reconfigure(&mut self) {
        self.gpu.context.configure_surface(self.surface);
    }

    fn encode_scene(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> kinetik_ui_render::RenderFrameOutput {
        self.toolkit.submit_frame(input)
    }

    fn render_vello(&mut self) -> Result<(), Self::RenderError> {
        let device = &self.gpu.context.devices[self.gpu.dev_id];
        self.gpu.renderer.render_to_texture(
            &device.device,
            &device.queue,
            self.toolkit.scene(),
            &self.surface.target_view,
            &self
                .config
                .render_params(self.surface.config.width, self.surface.config.height),
        )
    }

    fn blit_submit(&mut self, frame: &Self::Frame) {
        let device = &self.gpu.context.devices[self.gpu.dev_id];
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = device
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("kinetik-ui-vello-winit-blit"),
            });
        self.surface.blitter.copy(
            &device.device,
            &mut encoder,
            &self.surface.target_view,
            &view,
        );
        device.queue.submit([encoder.finish()]);
    }

    fn pre_present_notify(&mut self) {
        self.window.pre_present_notify();
    }

    fn present(&mut self, frame: Self::Frame) {
        frame.present();
    }

    fn device_events_after_render_failure(&mut self) -> CurrentDeviceEventOutcome {
        let events = self.inbox.drain_current(&self.current_scope);
        classify_current_device_events(events)
    }
}
