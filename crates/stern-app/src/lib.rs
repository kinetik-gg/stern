//! Application shell and event-loop runner for Stern.
//!
//! [`run`] owns the winit event loop, window lifecycle, input adaptation,
//! platform-request application, repaint scheduling, retained UI state
//! (memory, theme, and shaped-text caching), and the Vello presenter's
//! resize/recovery/present state machine. An application implements [`App`]
//! and keeps only its domain state:
//!
//! ```no_run
//! use stern_app::{App, AppConfig, AppError, ShellCtx, run};
//! use stern_core::Rect;
//! use stern_widgets::Ui;
//!
//! struct Hello;
//!
//! impl App for Hello {
//!     fn frame(&mut self, ui: &mut Ui<'_>, _shell: &mut ShellCtx) {
//!         ui.label(Rect::new(24.0, 24.0, 240.0, 24.0), "Hello, Stern!");
//!     }
//! }
//!
//! fn main() -> Result<(), AppError> {
//!     run(AppConfig::new("Hello"), Hello)
//! }
//! ```
//!
//! The manual, application-owned event loop remains supported through
//! `stern-vello-winit` for advanced hosts; this crate is the ergonomic
//! default path.

mod app;
mod config;
mod error;
mod runner;
mod shell;

pub use app::{App, RecoveryReport, RecoveryTrigger};
pub use config::{AppConfig, AppConfigError};
pub use error::AppError;
pub use runner::run;
pub use shell::{ShellCtx, ShellFocusRequest};
pub use stern_vello_winit::{
    VelloPresentStatus, VelloPresenterConfig, VelloPresenterError, VelloRecoveryKind,
};
