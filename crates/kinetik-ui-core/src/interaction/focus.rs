use super::{Response, pressable, pressable_transformed};
use crate::{Rect, Transform, UiInput, UiMemory, WidgetId};

/// Resolves neutral focus behavior.
pub fn focusable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable(id, rect, input, memory, disabled);

    if response.clicked {
        memory.focus(id);
        response.state.focused = true;
    }

    response
}

/// Resolves neutral focus behavior with transformed local-space hit testing.
pub fn focusable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable_transformed(id, rect, local_to_screen, input, memory, disabled);

    if response.clicked {
        memory.focus(id);
        response.state.focused = true;
    }

    response
}
