use super::{HitTarget, InteractionState, Response};
use crate::{Key, KeyState, Rect, Transform, UiInput, UiMemory, WidgetId};

/// Resolves neutral press/click behavior.
pub fn pressable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    pressable_with_hit_target(id, rect, HitTarget::Rect, input, memory, disabled)
}

/// Resolves neutral press/click behavior with transformed local-space hit testing.
pub fn pressable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    pressable_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
    )
}

pub(super) fn pressable_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let pointer_cancelled = memory.pointer_interaction_cancelled();
    let hovered =
        !pointer_cancelled && !disabled && hit_target.routed_hit_test(id, rect, input, memory);

    if hovered {
        memory.set_hovered(id);
    }

    if !disabled && hovered && input.pointer.primary.pressed {
        memory.activate(id);
        memory.press(id);
        memory.capture_pointer(id);
    }

    if !disabled && hovered && input.pointer.secondary.pressed {
        memory.press_secondary(id);
    }

    let active = memory.is_active(id);
    let pressed = memory.is_pressed(id) && input.pointer.primary.down;
    let keyboard_activated = !disabled && keyboard_activation_pressed(id, input, memory);
    let clicked =
        (!pointer_cancelled && !disabled && active && hovered && input.pointer.primary.released)
            || keyboard_activated;
    let double_clicked = clicked && input.pointer.click_count >= 2;
    let secondary_clicked = !pointer_cancelled
        && !disabled
        && hovered
        && memory.is_secondary_pressed(id)
        && input.pointer.secondary.released;

    let released_active_primary = active && input.pointer.primary.released;
    if released_active_primary {
        memory.finish_drag(id);
        memory.clear_interaction();
    }

    if input.pointer.secondary.released {
        memory.release_secondary(id);
    }

    Response {
        clicked,
        double_clicked,
        secondary_clicked,
        keyboard_activated,
        ..Response::new(
            id,
            rect,
            InteractionState {
                hovered,
                focused: memory.is_focused(id),
                active: active && !released_active_primary,
                pressed: pressed && !released_active_primary,
                disabled,
                selected: false,
            },
        )
    }
}

fn keyboard_activation_pressed(id: WidgetId, input: &UiInput, memory: &UiMemory) -> bool {
    memory.is_focused(id)
        && input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && !event.repeat
                && event.modifiers.is_empty()
                && matches!(event.key, Key::Enter | Key::Space)
                && !(memory.owns_text_input(id) && matches!(event.key, Key::Space))
        })
}
