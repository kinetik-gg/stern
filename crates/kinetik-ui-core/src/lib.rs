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
    AccessibilityAdapter, AccessibilityNode, AccessibilitySnapshot, FocusTraversal, SemanticAction,
    SemanticActionKind, SemanticNode, SemanticRole, SemanticState, SemanticTree, SemanticTreeError,
    SemanticValue,
};
pub use actions::{
    ActionBinding, ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation,
    ActionPriority, ActionQueue, ActionRouter, ActionRoutingContext, ActionSource, ActionState,
    Shortcut,
};
pub use debug::{
    DebugOverlay, PrimitiveInspection, PrimitiveKind, inspect_primitives, primitive_bounds,
    primitive_kind,
};
pub use geometry::{Point, Rect, Size, Vec2};
pub use identity::{DuplicateWidgetId, IdStack, WidgetId};
pub use input::{
    ClipboardText, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PhysicalKey,
    PointerButtonState, PointerInput, TextInputEvent, TextRange, UiInput,
};
pub use interaction::{
    DropTargetResponse, InteractionState, Response, ScrollResponse, clamp_scroll_offset,
    context_menu_trigger, draggable, drop_target, focusable, hit_test, max_scroll_offset,
    pressable, scrollable, selectable, tooltip_trigger,
};
pub use layout::{
    Alignment, Axis, Insets, LayoutItem, Measurement, SeparatorKind, SizeRule, column_layout,
    fit_box, pad_rect, rect_from_size, row_layout, split_leading, split_trailing, stack_layout,
};
pub use memory::UiMemory;
pub use perf::{
    AllocationBudget, AllocationUsage, BudgetStatus, FrameCounters, FrameMetrics, FrameTimings,
};
pub use render::{
    Brush, ClipId, Color, CornerRadius, GradientBuildError, GradientStop, IconId, ImageId,
    ImagePrimitive, LayerId, LinePrimitive, LinearGradient, MAX_GRADIENT_STOPS, PathElement,
    PathPrimitive, Primitive, RectPrimitive, ShadowPrimitive, Stroke, TextLayoutId, TextPrimitive,
    TextureId, TexturePrimitive, Transform,
};
pub use runtime::{
    CursorShape, FrameContext, FrameOutput, FrameWarning, PlatformRequest, RepaintRequest,
    TimeInfo, Ui, ViewportInfo,
};
pub use theme::{
    ButtonRecipe, ButtonVariant, CheckRecipe, ComponentState, ControlMetrics, DurationScale,
    ElevationScale, FontToken, OpacityScale, PanelRecipe, RadiusScale, RowRecipe, SemanticColor,
    SeparatorRecipe, ShadowRecipe, SliderRecipe, SpacingScale, TabRecipe, TextFieldRecipe,
    TextRecipe, TextRole, Theme, ThemeColors, ToggleRecipe, TypographyScale, default_dark_theme,
};
pub use units::{PhysicalPoint, PhysicalRect, PhysicalSize, ScaleFactor};

/// Returns the crate name.
#[must_use]
pub const fn crate_name() -> &'static str {
    "kinetik-ui-core"
}
