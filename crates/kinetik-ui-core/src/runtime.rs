//! UI frame runtime boundary types.

mod focus;
mod output;
mod primitive_stack;
mod types;
mod ui;

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests;

pub use output::FrameOutput;
pub use types::{
    CursorShape, FrameContext, FrameWarning, PlatformRequest, RepaintRequest, TimeInfo,
    ViewportInfo,
};
pub use ui::Ui;
