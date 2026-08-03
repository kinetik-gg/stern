use core::fmt;

use stern_core::{Size, Theme, default_dark_theme};
use stern_vello_winit::{VelloPresenterConfig, wgpu::PresentMode};

/// Default initial logical window size used by [`AppConfig::new`].
const DEFAULT_INITIAL_SIZE: Size = Size::new(960.0, 640.0);

/// Deterministic validation failure for an [`AppConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AppConfigError {
    /// The initial logical size is non-finite or not strictly positive.
    InvalidInitialSize,
    /// The minimum logical size is non-finite or not strictly positive.
    InvalidMinSize,
    /// The minimum logical size exceeds the initial logical size.
    MinSizeExceedsInitialSize,
}

impl fmt::Display for AppConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInitialSize => "initial logical size must be finite and strictly positive",
            Self::InvalidMinSize => "minimum logical size must be finite and strictly positive",
            Self::MinSizeExceedsInitialSize => {
                "minimum logical size must not exceed the initial logical size"
            }
        })
    }
}

impl std::error::Error for AppConfigError {}

/// Startup configuration consumed by [`crate::run`].
///
/// The configuration covers the window (title, initial and minimum logical
/// size), presentation options (vsync and the full presenter configuration),
/// and the retained [`Theme`] used for every composed frame.
#[derive(Debug, Clone)]
pub struct AppConfig {
    title: String,
    initial_size: Size,
    min_size: Option<Size>,
    theme: Theme,
    presenter: VelloPresenterConfig,
}

impl AppConfig {
    /// Creates a configuration with a 960x640 logical window, no minimum
    /// size, the default dark theme, and the default presenter options.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            initial_size: DEFAULT_INITIAL_SIZE,
            min_size: None,
            theme: default_dark_theme(),
            presenter: VelloPresenterConfig::new(),
        }
    }

    /// Replaces the initial logical window size.
    #[must_use]
    pub const fn with_initial_size(mut self, size: Size) -> Self {
        self.initial_size = size;
        self
    }

    /// Replaces the minimum logical window size.
    #[must_use]
    pub const fn with_min_size(mut self, size: Size) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Replaces the retained theme used for every composed frame.
    #[must_use]
    pub const fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Selects between the automatic vsync and low-latency present modes.
    #[must_use]
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        let mode = if vsync {
            PresentMode::AutoVsync
        } else {
            PresentMode::AutoNoVsync
        };
        if let Ok(presenter) = self.presenter.clone().with_present_mode(mode) {
            self.presenter = presenter;
        }
        self
    }

    /// Replaces the complete presenter configuration.
    #[must_use]
    pub fn with_presenter_config(mut self, presenter: VelloPresenterConfig) -> Self {
        self.presenter = presenter;
        self
    }

    /// Returns the window title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the initial logical window size.
    #[must_use]
    pub const fn initial_size(&self) -> Size {
        self.initial_size
    }

    /// Returns the minimum logical window size, when configured.
    #[must_use]
    pub const fn min_size(&self) -> Option<Size> {
        self.min_size
    }

    /// Returns the retained theme.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Returns the presenter configuration.
    #[must_use]
    pub const fn presenter_config(&self) -> &VelloPresenterConfig {
        &self.presenter
    }

    /// Validates the configuration deterministically without a window or GPU.
    ///
    /// # Errors
    ///
    /// Returns the first [`AppConfigError`] in declaration order: an invalid
    /// initial size, an invalid minimum size, then a minimum size that
    /// exceeds the initial size.
    pub fn validate(&self) -> Result<(), AppConfigError> {
        if !valid_logical_size(self.initial_size) {
            return Err(AppConfigError::InvalidInitialSize);
        }
        if let Some(min_size) = self.min_size {
            if !valid_logical_size(min_size) {
                return Err(AppConfigError::InvalidMinSize);
            }
            if min_size.width > self.initial_size.width
                || min_size.height > self.initial_size.height
            {
                return Err(AppConfigError::MinSizeExceedsInitialSize);
            }
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new("Stern application")
    }
}

fn valid_logical_size(size: Size) -> bool {
    size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0
}

#[cfg(test)]
mod tests {
    use stern_core::Size;
    use stern_vello_winit::wgpu::PresentMode;

    use super::{AppConfig, AppConfigError, DEFAULT_INITIAL_SIZE};

    #[test]
    fn default_config_is_valid_and_low_latency() {
        let config = AppConfig::new("Test");

        assert_eq!(config.title(), "Test");
        assert_eq!(config.initial_size(), DEFAULT_INITIAL_SIZE);
        assert_eq!(config.min_size(), None);
        assert_eq!(
            config.presenter_config().present_mode(),
            PresentMode::AutoNoVsync
        );
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn vsync_toggle_switches_between_automatic_present_modes() {
        let config = AppConfig::new("Test").with_vsync(true);
        assert_eq!(
            config.presenter_config().present_mode(),
            PresentMode::AutoVsync
        );

        let config = config.with_vsync(false);
        assert_eq!(
            config.presenter_config().present_mode(),
            PresentMode::AutoNoVsync
        );
    }

    #[test]
    fn validation_rejects_non_finite_and_non_positive_initial_sizes() {
        for size in [
            Size::new(0.0, 480.0),
            Size::new(640.0, 0.0),
            Size::new(-640.0, 480.0),
            Size::new(f32::NAN, 480.0),
            Size::new(640.0, f32::INFINITY),
        ] {
            assert_eq!(
                AppConfig::new("Test").with_initial_size(size).validate(),
                Err(AppConfigError::InvalidInitialSize),
                "{size:?}"
            );
        }
    }

    #[test]
    fn validation_rejects_invalid_and_oversized_minimum_sizes() {
        let config = AppConfig::new("Test").with_initial_size(Size::new(640.0, 480.0));

        for size in [Size::new(0.0, 10.0), Size::new(f32::NAN, 10.0)] {
            assert_eq!(
                config.clone().with_min_size(size).validate(),
                Err(AppConfigError::InvalidMinSize),
                "{size:?}"
            );
        }
        for size in [Size::new(641.0, 480.0), Size::new(640.0, 481.0)] {
            assert_eq!(
                config.clone().with_min_size(size).validate(),
                Err(AppConfigError::MinSizeExceedsInitialSize),
                "{size:?}"
            );
        }
        assert_eq!(
            config.with_min_size(Size::new(640.0, 480.0)).validate(),
            Ok(())
        );
    }
}
