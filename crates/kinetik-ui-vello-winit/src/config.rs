use std::time::Duration;

use kinetik_ui_core::Color;
use vello::{AaConfig, wgpu::PresentMode};

use crate::{InvalidColorChannel, VelloPresenterError};

/// Configuration retained across presenter surface and device recovery.
#[derive(Debug, Clone)]
pub struct VelloPresenterConfig {
    present_mode: PresentMode,
    antialiasing_method: AaConfig,
    base_color: Color,
    timeout_retry: Duration,
}

impl VelloPresenterConfig {
    /// Creates the supported low-latency alpha configuration.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            present_mode: PresentMode::AutoNoVsync,
            antialiasing_method: AaConfig::Msaa16,
            base_color: Color::rgb(11.0 / 255.0, 12.0 / 255.0, 13.0 / 255.0),
            timeout_retry: Duration::from_millis(16),
        }
    }

    /// Replaces the automatic presentation mode.
    ///
    /// # Errors
    ///
    /// Returns [`VelloPresenterError::Validation`] for a non-automatic mode.
    pub fn with_present_mode(mut self, mode: PresentMode) -> Result<Self, VelloPresenterError> {
        if !matches!(mode, PresentMode::AutoVsync | PresentMode::AutoNoVsync) {
            return Err(VelloPresenterError::Validation {
                message: "only AutoVsync and AutoNoVsync are supported".into(),
            });
        }
        self.present_mode = mode;
        Ok(self)
    }

    /// Replaces the Vello antialiasing method.
    #[must_use]
    pub const fn with_antialiasing_method(mut self, method: AaConfig) -> Self {
        self.antialiasing_method = method;
        self
    }

    /// Replaces the straight-sRGB, straight-alpha base color.
    ///
    /// # Errors
    ///
    /// Returns [`VelloPresenterError::InvalidBaseColor`] for the first channel
    /// that is non-finite or outside `0.0..=1.0`.
    pub fn with_base_color(mut self, color: Color) -> Result<Self, VelloPresenterError> {
        validate_color(color)?;
        self.base_color = color;
        Ok(self)
    }

    /// Replaces the bounded retry delay returned after a surface timeout.
    ///
    /// # Errors
    ///
    /// A zero delay is rejected because it would become an unbounded busy loop.
    pub fn with_timeout_retry(mut self, delay: Duration) -> Result<Self, VelloPresenterError> {
        if delay.is_zero() {
            return Err(VelloPresenterError::Validation {
                message: "surface timeout retry delay must be non-zero".into(),
            });
        }
        self.timeout_retry = delay;
        Ok(self)
    }

    /// Returns the automatic presentation mode.
    #[must_use]
    pub const fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    /// Returns the Vello antialiasing method.
    #[must_use]
    pub const fn antialiasing_method(&self) -> AaConfig {
        self.antialiasing_method
    }

    /// Returns the straight-sRGB, straight-alpha base color.
    #[must_use]
    pub const fn base_color(&self) -> Color {
        self.base_color
    }

    /// Returns the bounded surface-timeout retry delay.
    #[must_use]
    pub const fn timeout_retry(&self) -> Duration {
        self.timeout_retry
    }

    pub(crate) fn render_params(&self, width: u32, height: u32) -> vello::RenderParams {
        vello::RenderParams {
            base_color: vello::peniko::Color::new([
                self.base_color.r,
                self.base_color.g,
                self.base_color.b,
                self.base_color.a,
            ]),
            width,
            height,
            antialiasing_method: self.antialiasing_method,
        }
    }
}

impl Default for VelloPresenterConfig {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_color(color: Color) -> Result<(), VelloPresenterError> {
    for (channel, value) in [
        (InvalidColorChannel::Red, color.r),
        (InvalidColorChannel::Green, color.g),
        (InvalidColorChannel::Blue, color.b),
        (InvalidColorChannel::Alpha, color.a),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(VelloPresenterError::InvalidBaseColor { channel });
        }
    }
    Ok(())
}
