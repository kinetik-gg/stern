//! Winit platform adapter for Kinetik UI.

mod accessibility;
mod conversions;
mod input;
mod requests;
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
pub use requests::{
    WinitPlatformRequests, WinitShellRequests, WinitTextInputRequest, WinitWindowOps,
};
pub use time::WinitFrameClock;
pub use viewport::{frame_context_from_winit, scale_factor_from_winit, viewport_from_winit};
