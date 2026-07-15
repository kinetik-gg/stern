//! Core runtime types for Stern.
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
pub mod liveness;
pub mod memory;
pub mod observers;
pub mod perf;
pub mod render;
pub mod runtime;
pub mod test_harness;
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
    DebugOverlay, DiagnosticCategory, DiagnosticLocation, DiagnosticSeverity, FrameDiagnostic,
    PrimitiveInspection, PrimitiveKind, inspect_primitives, primitive_bounds, primitive_kind,
};
pub use geometry::{Point, Rect, Size, Vec2};
pub use identity::{DuplicateWidgetId, IdStack, WidgetId};
pub use input::{
    ClipboardText, InputStreamConflict, InputWheelDelta, Key, KeyEvent, KeyState, KeyboardInput,
    Modifiers, MouseButton, PhysicalKey, PointerButtonState, PointerInput, TextInputEvent,
    TextRange, UiInput, UiInputEvent,
};
pub use interaction::{
    CapturedDomainDragGesture, CapturedSelectionGesture, DomainDragGestureAction,
    DomainDragGesturePhase, DropTargetResponse, InteractionState, OrderedTextInputEvent, Response,
    ScrollResponse, SelectionGestureAction, SelectionGesturePhase, clamp_scroll_offset,
    context_menu_trigger, context_menu_trigger_transformed, draggable, draggable_transformed,
    drop_target, drop_target_transformed, focusable, focusable_transformed, hit_test,
    hit_test_transformed, max_scroll_offset, pressable, pressable_transformed, scrollable,
    scrollable_transformed, selectable, selectable_transformed, tooltip_trigger,
    tooltip_trigger_transformed,
};
pub use layout::{
    Alignment, Axis, Insets, LayoutItem, Measurement, SeparatorKind, SizeRule, column_layout,
    fit_box, grid_layout, pad_rect, rect_from_size, row_layout, split_leading, split_trailing,
    stack_layout,
};
#[allow(deprecated)]
pub use liveness::LivenessGeneration;
pub use liveness::{
    LivenessIncarnation, LivenessRegistry, LivenessRemovalStatus, LivenessTargetId, LivenessToken,
    LivenessUpdateStatus,
};
pub use memory::{PointerRoute, PointerRoutes, TextInputOwnerMode, UiMemory};
pub use observers::{
    ObserverDelivery, ObserverDeliverySkipReason, ObserverDeliveryStatus, ObserverDrain,
    ObserverNotification, ObserverNotificationId, ObserverPublishStatus, ObserverRegistry,
    ObserverSkippedDelivery, ObserverSubscriptionHandle, ObserverSubscriptionId,
};
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
    CursorShape, FrameContext, FrameOutput, FrameWarning, PlatformRequest, PointerOrder,
    PointerPlanError, PointerTarget, PointerTargetPlan, RepaintRequest, TimeInfo, Ui, ViewportInfo,
};
pub use test_harness::{
    FrameTrace, HarnessPhase, ScriptedInput, ScriptedKeyEvent, SettlePendingCause, SettleResult,
    UiTestHarness,
};
pub use theme::{
    AccentColors, BorderColors, ButtonRecipe, ButtonVariant, CheckRecipe, ComponentState,
    ContentColors, ControlMetrics, DurationScale, ElevationLevel, ElevationScale, FocusColors,
    FocusStrokeScale, FontToken, OpacityScale, OverlayColors, PanelRecipe, RadiusScale, RowRecipe,
    SelectionColors, SemanticColor, SeparatorRecipe, ShadowRecipe, SliderRecipe, SpacingScale,
    StatusColorFamilyColors, StatusColors, StrokeScale, SurfaceColors, TabRecipe, TextFieldRecipe,
    TextRecipe, TextRole, Theme, ThemeColors, ToggleRecipe, TypographyScale, default_dark_theme,
};
pub use units::{PhysicalPoint, PhysicalRect, PhysicalSize, ScaleFactor};

/// Returns the crate name.
#[must_use]
pub const fn crate_name() -> &'static str {
    "stern-core"
}
