use core::fmt;

use stern_vello_winit::VelloPresenterError;
use winit::error::EventLoopError;

use crate::config::AppConfigError;

/// Failure returned by [`crate::run`].
#[derive(Debug)]
#[non_exhaustive]
pub enum AppError {
    /// The application configuration failed validation.
    Config(AppConfigError),
    /// The window presenter failed to initialize, present, or recover.
    Presenter(VelloPresenterError),
    /// The platform event loop could not be created or run.
    EventLoop(EventLoopError),
    /// The platform window could not be created.
    Window(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "invalid application config: {error}"),
            Self::Presenter(error) => write!(formatter, "presenter failed: {error}"),
            Self::EventLoop(error) => write!(formatter, "event loop failed: {error}"),
            Self::Window(message) => write!(formatter, "window creation failed: {message}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Presenter(error) => Some(error),
            Self::EventLoop(error) => Some(error),
            Self::Window(_) => None,
        }
    }
}

impl From<AppConfigError> for AppError {
    fn from(error: AppConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<VelloPresenterError> for AppError {
    fn from(error: VelloPresenterError) -> Self {
        Self::Presenter(error)
    }
}

impl From<EventLoopError> for AppError {
    fn from(error: EventLoopError) -> Self {
        Self::EventLoop(error)
    }
}
