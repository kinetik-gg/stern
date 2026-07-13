//! Concrete Vello/Winit window presentation for Kinetik UI.
//!
//! The application owns the Winit event loop, frame construction, input,
//! platform requests, and repaint scheduler. [`VelloWindowPresenter`] owns one
//! attached window's Vello/wgpu surface, device, queue, GPU renderer, and
//! acquire/render/blit/notify/present policy.

mod config;
mod device;
mod error;
mod frame;
mod lifecycle;
mod native_texture;
mod presenter;
mod result;

pub use config::VelloPresenterConfig;
pub use device::{PresenterDevice, PresenterDeviceScope};
pub use error::{
    InvalidColorChannel, PresenterGpuError, PresenterGpuErrorKind,
    VelloNativeTextureValidationError, VelloPresenterError,
};
pub use native_texture::{VelloNativeTextureRegistration, VelloNativeTextureUpdateOutcome};
pub use presenter::VelloWindowPresenter;
pub use result::{
    VelloAttachOutcome, VelloAttachmentStatus, VelloPresentReport, VelloPresentStatus,
    VelloPresenterStatus, VelloRecoveryKind, VelloRecoveryOutcome, VelloRedrawGuidance,
    VelloResizeOutcome, VelloSuspendOutcome,
};
pub use vello::{AaConfig, wgpu};

#[cfg(test)]
mod tests;
