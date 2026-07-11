//! Visually neutral interaction primitives.

mod drag_select;
mod focus;
mod hit;
mod overlay;
mod press;
mod response;
mod scroll;
#[cfg(test)]
mod tests;

pub(crate) use drag_select::captured_selection_gesture_with_ordinals;
pub use drag_select::{
    CapturedSelectionGesture, OrderedTextInputEvent, SelectionGestureAction, SelectionGesturePhase,
    draggable, draggable_transformed, selectable, selectable_transformed,
};
pub use focus::{focusable, focusable_transformed};
pub use hit::{hit_test, hit_test_transformed};
pub use overlay::{
    context_menu_trigger, context_menu_trigger_transformed, drop_target, drop_target_transformed,
    tooltip_trigger, tooltip_trigger_transformed,
};
pub use press::{pressable, pressable_transformed};
pub use response::{DropTargetResponse, InteractionState, Response, ScrollResponse};
pub use scroll::{clamp_scroll_offset, max_scroll_offset, scrollable, scrollable_transformed};

use hit::HitTarget;

const DRAG_THRESHOLD_SQUARED: f32 = 16.0;

pub(crate) fn crosses_drag_threshold(origin: crate::Point, position: crate::Point) -> bool {
    let x = position.x - origin.x;
    let y = position.y - origin.y;
    let displacement_squared = x.mul_add(x, y * y);
    displacement_squared.is_finite() && displacement_squared >= DRAG_THRESHOLD_SQUARED
}

pub(crate) fn canonical_pointer_fenced(input: &crate::UiInput) -> bool {
    !input.events.is_empty()
        && input.events.iter().any(|event| {
            matches!(
                event,
                crate::UiInputEvent::PointerReleaseAll { .. }
                    | crate::UiInputEvent::WindowFocusChanged(false)
            )
        })
}
