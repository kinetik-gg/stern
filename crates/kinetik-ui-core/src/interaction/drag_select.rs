use super::hit::HitTarget;
use super::press::pressable_with_hit_target;
use super::{Response, pressable, pressable_transformed};
use crate::{Rect, Transform, UiInput, UiMemory, Vec2, WidgetId};

/// Resolves neutral draggable behavior.
pub fn draggable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    draggable_with_hit_target(id, rect, HitTarget::Rect, input, memory, disabled)
}

/// Resolves neutral draggable behavior with transformed local-space hit testing.
pub fn draggable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    draggable_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
    )
}

fn draggable_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable_with_hit_target(id, rect, hit_target, input, memory, disabled);
    let active = memory.is_active(id);

    response.dragged =
        !disabled && active && input.pointer.primary.down && input.pointer.delta != Vec2::ZERO;
    response.drag_delta = if response.dragged {
        memory.start_drag(id);
        input.pointer.delta
    } else {
        Vec2::ZERO
    };
    response.state.active = active;

    response
}

/// Resolves neutral selectable behavior.
pub fn selectable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    selected: bool,
    disabled: bool,
) -> Response {
    let mut response = pressable(id, rect, input, memory, disabled);
    response.state.selected = selected;
    response
}

/// Resolves neutral selectable behavior with transformed local-space hit testing.
pub fn selectable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    selected: bool,
    disabled: bool,
) -> Response {
    let mut response = pressable_transformed(id, rect, local_to_screen, input, memory, disabled);
    response.state.selected = selected;
    response
}
