//! Core runtime types for Kinetik UI.
//!
//! This crate owns platform-independent UI concepts. It must not depend on
//! windowing, renderer, or operating-system APIs.

pub mod geometry;
pub mod identity;
pub mod input;
pub mod layout;
pub mod memory;
pub mod runtime;
pub mod units;

pub use geometry::{Point, Rect, Size, Vec2};
pub use identity::{DuplicateWidgetId, IdStack, WidgetId};
pub use input::{
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PointerButtonState,
    PointerInput, TextInputEvent, UiInput,
};
pub use layout::{
    Alignment, Axis, Insets, LayoutItem, Measurement, SeparatorKind, SizeRule, column_layout,
    fit_box, pad_rect, row_layout, stack_layout,
};
pub use memory::UiMemory;
pub use runtime::{FrameContext, FrameOutput, RepaintRequest, TimeInfo, ViewportInfo};
pub use units::{PhysicalPoint, PhysicalSize, ScaleFactor};

/// Returns the crate name.
#[must_use]
pub const fn crate_name() -> &'static str {
    "kinetik-ui-core"
}
