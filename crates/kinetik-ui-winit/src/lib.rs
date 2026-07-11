//! Winit platform adapter for Kinetik UI.

mod accessibility;
mod conversions;
mod input;
mod repaint;
mod requests;
mod shell;
#[cfg(test)]
mod tests;
mod time;
mod utils;
mod viewport;

pub use accessibility::WinitAccessibilityUpdate;
pub use conversions::{
    cursor_to_winit, key_from_winit, modifiers_from_winit, physical_key_from_winit,
};
pub use input::WinitInputAdapter;
pub use repaint::{WinitRepaintSchedule, WinitRepaintScheduler};
pub use requests::{
    WinitAppliedRequests, WinitPlatformRequests, WinitTextInputRequest, WinitWindowOps,
};
pub use shell::{
    NativeWinitShellServices, WinitShellFailure, WinitShellFailureReason, WinitShellOperation,
    WinitShellOutcome, WinitShellRequest, WinitShellRequests, WinitShellResult,
    WinitShellServiceError, WinitShellServices,
};
pub use time::WinitFrameClock;
pub use viewport::{frame_context_from_winit, scale_factor_from_winit, viewport_from_winit};
