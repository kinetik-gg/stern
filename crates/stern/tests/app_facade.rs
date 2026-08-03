//! Facade reachability for the stern-app runner surface.

#![cfg(feature = "vello-winit")]

use stern::app::{AppConfig, AppConfigError, ShellCtx};
use stern::core::{PlatformRequest, ScaleFactor, Size};

#[test]
fn facade_exposes_validated_runner_config() {
    let config = AppConfig::new("Facade").with_initial_size(Size::new(320.0, 240.0));
    assert_eq!(config.validate(), Ok(()));

    let invalid = AppConfig::new("Facade").with_initial_size(Size::new(0.0, 240.0));
    assert_eq!(invalid.validate(), Err(AppConfigError::InvalidInitialSize));
}

#[test]
fn facade_shell_context_records_platform_request_intent() {
    let mut shell = ShellCtx::new(ScaleFactor::ONE);
    shell.set_window_title("Facade");
    shell.request_close();

    assert_eq!(
        shell.requests(),
        &[PlatformRequest::SetWindowTitle("Facade".to_owned())]
    );
    assert!(shell.close_requested());
}
