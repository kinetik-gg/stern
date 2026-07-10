//! UI frame runtime boundary types.

mod focus;
mod output;
mod pointer;
mod primitive_stack;
pub(crate) mod spatial;
mod types;
mod ui;

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests;

pub use output::FrameOutput;
pub use pointer::{PointerOrder, PointerPlanError, PointerTarget, PointerTargetPlan};
pub use types::{
    CursorShape, FrameContext, FrameWarning, PlatformRequest, RepaintRequest, TimeInfo,
    ViewportInfo,
};
pub use ui::Ui;
