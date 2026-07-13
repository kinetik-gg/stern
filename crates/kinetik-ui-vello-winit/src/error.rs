use std::fmt;

use kinetik_ui_core::TextureId;

/// A channel rejected while validating the presenter's base color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidColorChannel {
    /// Red channel.
    Red,
    /// Green channel.
    Green,
    /// Blue channel.
    Blue,
    /// Alpha channel.
    Alpha,
}

/// Kind of error reported by wgpu's uncaptured-error callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresenterGpuErrorKind {
    /// The device ran out of memory.
    OutOfMemory,
    /// A validation rule was violated.
    Validation,
    /// The backend reported an internal error.
    Internal,
}

/// Preserved uncaptured wgpu error information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenterGpuError {
    kind: PresenterGpuErrorKind,
    message: String,
}

impl PresenterGpuError {
    pub(crate) fn new(kind: PresenterGpuErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the stable error category.
    #[must_use]
    pub const fn kind(&self) -> PresenterGpuErrorKind {
        self.kind
    }

    /// Returns the backend-provided message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Reason a native texture cannot be registered or replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloNativeTextureValidationError {
    /// Replacement resource ID differs from the registered texture ID.
    ResourceIdMismatch,
    /// Native width or height is zero.
    ZeroExtent,
    /// Neutral resource width or height is not finite, positive, and integral.
    NonIntegralResourceExtent,
    /// Neutral resource extent differs from the native texture extent.
    ResourceExtentMismatch,
    /// Native texture format is not `Rgba8Unorm`.
    UnsupportedFormat,
    /// Native texture usage does not contain `COPY_SRC`.
    MissingCopySourceUsage,
    /// Native texture is not two-dimensional.
    UnsupportedDimension,
    /// Native texture has more than one array layer.
    UnsupportedArrayLayers,
    /// Native texture has more than one mip level.
    UnsupportedMipLevels,
    /// Native texture is multisampled.
    UnsupportedSampleCount,
}

/// Error returned by the Vello/Winit presenter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloPresenterError {
    /// Presenter or GPU initialization failed.
    Initialization {
        /// Diagnostic message.
        message: String,
    },
    /// Caller-provided configuration or frame data is invalid.
    Validation {
        /// Diagnostic message.
        message: String,
    },
    /// A base-color channel was non-finite or outside `0.0..=1.0`.
    InvalidBaseColor {
        /// Rejected channel.
        channel: InvalidColorChannel,
    },
    /// Vello failed while rendering the encoded scene.
    Render {
        /// Diagnostic message.
        message: String,
    },
    /// Surface or whole-device recovery failed.
    Recovery {
        /// Diagnostic message.
        message: String,
    },
    /// No current device is available for borrowing.
    DeviceUnavailable,
    /// The checked presenter identity or generation could not advance.
    GenerationExhausted,
    /// A resume call supplied a different window while one was attached.
    WrongWindow,
    /// A device scope belongs to another presenter.
    ForeignPresenterScope,
    /// A device scope belongs to a replaced device generation.
    StaleDeviceScope,
    /// The current device reported an uncaptured GPU error.
    UncapturedGpu(PresenterGpuError),
    /// The bounded uncaptured-error inbox overflowed.
    UncapturedErrorOverflow {
        /// Number of callback events known to have been dropped.
        dropped: u64,
    },
    /// The neutral texture ID is already registered.
    NativeTextureAlreadyRegistered {
        /// Conflicting texture ID.
        texture: TextureId,
    },
    /// The neutral texture ID has no active registration.
    NativeTextureNotRegistered {
        /// Missing texture ID.
        texture: TextureId,
    },
    /// The registration belongs to stale presenter, device, or ID state.
    StaleNativeTextureRegistration {
        /// Stale texture ID.
        texture: TextureId,
    },
    /// The requested content revision moved backward.
    NativeTextureRevisionRegressed {
        /// Registered texture ID.
        texture: TextureId,
        /// Current accepted revision.
        current: u64,
        /// Rejected requested revision.
        requested: u64,
    },
    /// Native texture metadata violates the supported contract.
    InvalidNativeTexture {
        /// Rejected texture ID.
        texture: TextureId,
        /// Stable validation reason.
        reason: VelloNativeTextureValidationError,
    },
    /// The checked native registration generation could not advance.
    NativeTextureGenerationExhausted,
}

impl VelloPresenterError {
    pub(crate) fn initialization(error: impl fmt::Display) -> Self {
        Self::Initialization {
            message: error.to_string(),
        }
    }

    pub(crate) fn recovery(error: impl fmt::Display) -> Self {
        Self::Recovery {
            message: error.to_string(),
        }
    }

    pub(crate) fn render(error: impl fmt::Display) -> Self {
        Self::Render {
            message: error.to_string(),
        }
    }
}

impl fmt::Display for VelloPresenterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialization { message } => {
                write!(formatter, "presenter initialization failed: {message}")
            }
            Self::Validation { message } => {
                write!(formatter, "presenter validation failed: {message}")
            }
            Self::InvalidBaseColor { channel } => {
                write!(
                    formatter,
                    "invalid presenter base-color channel: {channel:?}"
                )
            }
            Self::Render { message } => write!(formatter, "Vello render failed: {message}"),
            Self::Recovery { message } => write!(formatter, "presenter recovery failed: {message}"),
            Self::DeviceUnavailable => formatter.write_str("presenter device is unavailable"),
            Self::GenerationExhausted => {
                formatter.write_str("presenter identity or device generation exhausted")
            }
            Self::WrongWindow => formatter.write_str("presenter is attached to a different window"),
            Self::ForeignPresenterScope => {
                formatter.write_str("device scope belongs to another presenter")
            }
            Self::StaleDeviceScope => {
                formatter.write_str("device scope belongs to a stale device generation")
            }
            Self::UncapturedGpu(error) => {
                write!(
                    formatter,
                    "uncaptured {:?} GPU error: {}",
                    error.kind(),
                    error.message()
                )
            }
            Self::UncapturedErrorOverflow { dropped } => {
                write!(
                    formatter,
                    "uncaptured GPU error inbox dropped {dropped} event(s)"
                )
            }
            Self::NativeTextureAlreadyRegistered { texture } => {
                write!(
                    formatter,
                    "native texture {} is already registered",
                    texture.raw()
                )
            }
            Self::NativeTextureNotRegistered { texture } => {
                write!(
                    formatter,
                    "native texture {} is not registered",
                    texture.raw()
                )
            }
            Self::StaleNativeTextureRegistration { texture } => {
                write!(
                    formatter,
                    "native texture {} registration is stale",
                    texture.raw()
                )
            }
            Self::NativeTextureRevisionRegressed {
                texture,
                current,
                requested,
            } => write!(
                formatter,
                "native texture {} revision regressed from {current} to {requested}",
                texture.raw()
            ),
            Self::InvalidNativeTexture { texture, reason } => write!(
                formatter,
                "native texture {} is invalid: {reason:?}",
                texture.raw()
            ),
            Self::NativeTextureGenerationExhausted => {
                formatter.write_str("native texture registration generation exhausted")
            }
        }
    }
}

impl std::error::Error for VelloPresenterError {}
