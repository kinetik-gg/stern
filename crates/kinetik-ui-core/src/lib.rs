//! Core runtime types for Kinetik UI.
//!
//! This crate owns platform-independent UI concepts. It must not depend on
//! windowing, renderer, or operating-system APIs.

pub mod accessibility;
pub mod actions;
pub mod debug;
pub mod geometry;
pub mod identity;
pub mod input;
pub mod interaction;
pub mod layout;
pub mod memory;
pub mod perf;
pub mod render;
pub mod runtime;
pub mod theme;
pub mod units;

pub use accessibility::{
    AccessibilityAdapter, FocusTraversal, SemanticAction, SemanticActionKind, SemanticNode,
    SemanticRole, SemanticState, SemanticTree, SemanticValue,
};
pub use actions::{
    ActionBinding, ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation,
    ActionPriority, ActionQueue, ActionRouter, ActionSource, ActionState, Shortcut,
};
pub use debug::{
    DebugOverlay, PrimitiveInspection, PrimitiveKind, inspect_primitives, primitive_bounds,
    primitive_kind,
};
pub use geometry::{Point, Rect, Size, Vec2};
pub use identity::{DuplicateWidgetId, IdStack, WidgetId};
pub use input::{
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PointerButtonState,
    PointerInput, TextInputEvent, UiInput,
};
pub use interaction::{
    InteractionState, Response, draggable, focusable, hit_test, pressable, selectable,
};
pub use layout::{
    Alignment, Axis, Insets, LayoutItem, Measurement, SeparatorKind, SizeRule, column_layout,
    fit_box, pad_rect, row_layout, stack_layout,
};
pub use memory::UiMemory;
pub use perf::{
    AllocationBudget, AllocationUsage, BudgetStatus, FrameCounters, FrameMetrics, FrameTimings,
};
pub use render::{
    Brush, ClipId, Color, CornerRadius, ImageId, ImagePrimitive, LayerId, LinePrimitive, Primitive,
    RectPrimitive, Stroke, TextPrimitive, TextureId, TexturePrimitive, Transform,
};
pub use runtime::{FrameContext, FrameOutput, RepaintRequest, TimeInfo, ViewportInfo};
pub use theme::{
    ButtonRecipe, ComponentState, SemanticColor, SpacingScale, Theme, ThemeColors,
    default_dark_theme,
};
pub use units::{PhysicalPoint, PhysicalSize, ScaleFactor};

/// Returns the crate name.
#[must_use]
pub const fn crate_name() -> &'static str {
    "kinetik-ui-core"
}
