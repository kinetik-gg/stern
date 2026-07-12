use std::time::Duration;

use kinetik_ui_render::RenderFrameOutput;

use crate::PresenterDeviceScope;

/// Current window attachment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloAttachmentStatus {
    /// No window is retained.
    Detached,
    /// A window is retained but its raw physical extent contains zero.
    ZeroSized,
    /// A non-zero window and configured surface are ready.
    Presentable,
    /// A non-zero window is retained while async recovery is required.
    RecoveryPending,
}

/// Pending recovery authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloRecoveryKind {
    /// A surface must be created for the retained window.
    CreateSurface,
    /// A lost surface must be recreated from the retained window.
    RecreateSurface,
    /// The complete device/context/surface path must be rebuilt.
    RebuildDevice,
}

/// Snapshot of presenter attachment, recovery, and device-scope state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VelloPresenterStatus {
    attachment: VelloAttachmentStatus,
    recovery: Option<VelloRecoveryKind>,
    device_scope: Option<PresenterDeviceScope>,
}

impl VelloPresenterStatus {
    pub(crate) const fn new(
        attachment: VelloAttachmentStatus,
        recovery: Option<VelloRecoveryKind>,
        device_scope: Option<PresenterDeviceScope>,
    ) -> Self {
        Self {
            attachment,
            recovery,
            device_scope,
        }
    }

    /// Returns the attachment state.
    #[must_use]
    pub const fn attachment(&self) -> VelloAttachmentStatus {
        self.attachment
    }

    /// Returns the pending recovery kind, if any.
    #[must_use]
    pub const fn recovery(&self) -> Option<VelloRecoveryKind> {
        self.recovery
    }

    /// Returns the current usable device scope, if any.
    #[must_use]
    pub const fn device_scope(&self) -> Option<&PresenterDeviceScope> {
        self.device_scope.as_ref()
    }
}

/// Result of attaching or redundantly resuming a window.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloAttachOutcome {
    /// A zero-sized window was retained without creating GPU resources.
    AttachedZeroSized,
    /// A non-zero window and surface were initialized.
    AttachedPresentable {
        /// Scope of the initialized presenter device.
        device_scope: PresenterDeviceScope,
    },
    /// The same already-attached window was observed again.
    AlreadyAttached,
}

/// Result of suspending the presenter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloSuspendOutcome {
    /// Surface ownership was dropped before window ownership.
    Suspended,
    /// The presenter was already detached.
    AlreadyDetached,
}

/// Result of applying a raw physical window size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloResizeOutcome {
    /// No window is attached.
    Detached,
    /// The requested size and state were already current.
    Unchanged,
    /// The raw extent is zero and no surface is configured.
    ZeroSized,
    /// The existing surface was configured exactly once at the new extent.
    Resized,
    /// Async surface or device recovery is required before presentation.
    RecoveryRequired(VelloRecoveryKind),
}

/// Result of one asynchronous recovery attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloRecoveryOutcome {
    /// There was no pending recovery.
    NotNeeded,
    /// Recovery remains pending until a window is attached.
    DeferredDetached(VelloRecoveryKind),
    /// Recovery remains pending until a non-zero extent is observed.
    DeferredZeroSized(VelloRecoveryKind),
    /// A surface was created or recreated.
    SurfaceReady {
        /// Whether selecting the compatible device changed the device scope.
        device_changed: bool,
        /// Current device scope after recovery.
        device_scope: PresenterDeviceScope,
    },
    /// The complete context/device/renderer/surface path was rebuilt.
    DeviceRebuilt {
        /// New device scope; every older scope is stale.
        device_scope: PresenterDeviceScope,
    },
}

/// Typed result of one synchronous presentation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloPresentStatus {
    /// The frame was rendered and presented once.
    Presented,
    /// The acquired suboptimal frame was presented and reconfigure was queued.
    PresentedSuboptimal,
    /// The submitted frame extent was stale; acquisition was skipped.
    FrameExtentOutdated,
    /// The acquired texture extent disagreed and was dropped before configure.
    AcquiredExtentOutdated,
    /// Surface acquisition timed out before scene encoding.
    Timeout,
    /// The window was occluded before scene encoding.
    Occluded,
    /// The surface was reconfigured after an outdated acquisition.
    Outdated,
    /// The surface was lost and recreation is pending.
    SurfaceLost,
    /// Surface creation or recreation must complete before another attempt.
    SurfaceRecoveryRequired,
    /// Whole-device recovery is pending.
    DeviceRecoveryRequired,
    /// No window is attached.
    Detached,
    /// The attached raw physical extent contains zero.
    ZeroSized,
}

/// Repaint scheduling guidance returned by the presenter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloRedrawGuidance {
    /// Preserve or merge the application's normal frame request.
    UseApplicationRequest,
    /// Request exactly one subsequent frame through the application scheduler.
    NextFrame,
    /// Retry through the scheduler after a bounded delay.
    Later(Duration),
    /// Wait for an external window or visibility event.
    ExternalEvent,
    /// Wait for a resize whose raw physical extent is non-zero.
    NonZeroResize,
    /// Do not automatically retry an actionable failure.
    None,
}

/// Diagnostics and scheduling result of one presentation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VelloPresentReport {
    status: VelloPresentStatus,
    redraw: VelloRedrawGuidance,
    frame_output: Option<RenderFrameOutput>,
}

impl VelloPresentReport {
    pub(crate) const fn new(
        status: VelloPresentStatus,
        redraw: VelloRedrawGuidance,
        frame_output: Option<RenderFrameOutput>,
    ) -> Self {
        Self {
            status,
            redraw,
            frame_output,
        }
    }

    /// Returns the typed attempt status.
    #[must_use]
    pub const fn status(&self) -> VelloPresentStatus {
        self.status
    }

    /// Returns repaint scheduling guidance.
    #[must_use]
    pub const fn redraw(&self) -> VelloRedrawGuidance {
        self.redraw
    }

    /// Returns renderer diagnostics only when scene encoding occurred.
    #[must_use]
    pub const fn frame_output(&self) -> Option<&RenderFrameOutput> {
        self.frame_output.as_ref()
    }
}
