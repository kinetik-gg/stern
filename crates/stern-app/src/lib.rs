//! Application shell and event-loop runner for Stern.
//!
//! This crate delivers the eframe-class runner described in the workspace
//! gap ledger: it will own the winit event loop, window lifecycle, input
//! adaptation, retained UI state, and the Vello presenter's
//! resize/recovery/present state machine so applications keep only their
//! domain state. This skeleton slice carries the validated [`AppConfig`] and
//! the [`AppError`] surface; the runner core lands on top of it.

mod config;
mod error;

pub use config::{AppConfig, AppConfigError};
pub use error::AppError;
pub use stern_vello_winit::{
    VelloPresentStatus, VelloPresenterConfig, VelloPresenterError, VelloRecoveryKind,
};
