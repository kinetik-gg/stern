//! Visually neutral interaction primitives.

use crate::{Rect, UiInput, UiMemory, Vec2, WidgetId};

/// Interaction state flags for a widget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct InteractionState {
    /// Pointer is over the widget.
    pub hovered: bool,
    /// Widget has keyboard focus.
    pub focused: bool,
    /// Widget is active for pointer/modal interaction.
    pub active: bool,
    /// Widget is currently pressed.
    pub pressed: bool,
    /// Widget is disabled.
    pub disabled: bool,
    /// Widget is selected.
    pub selected: bool,
}

/// Common response returned by interaction primitives and components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Response {
    /// Widget identity.
    pub id: WidgetId,
    /// Widget bounds.
    pub rect: Rect,
    /// Interaction state.
    pub state: InteractionState,
    /// Whether the widget was clicked this frame.
    pub clicked: bool,
    /// Whether the widget was double-clicked this frame.
    pub double_clicked: bool,
    /// Whether the secondary pointer button clicked this frame.
    pub secondary_clicked: bool,
    /// Whether the widget was dragged this frame.
    pub dragged: bool,
    /// Drag delta in logical units.
    pub drag_delta: Vec2,
}

impl Response {
    /// Creates a response with default event flags.
    #[must_use]
    pub const fn new(id: WidgetId, rect: Rect, state: InteractionState) -> Self {
        Self {
            id,
            rect,
            state,
            clicked: false,
            double_clicked: false,
            secondary_clicked: false,
            dragged: false,
            drag_delta: Vec2::ZERO,
        }
    }
}

/// Returns true when pointer input is inside a rectangle.
#[must_use]
pub fn hit_test(rect: Rect, input: &UiInput) -> bool {
    input
        .pointer
        .position
        .is_some_and(|position| rect.contains_point(position))
}

/// Resolves neutral press/click behavior.
pub fn pressable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let hovered = !disabled && hit_test(rect, input);

    if hovered {
        memory.hovered = Some(id);
    }

    if !disabled && hovered && input.pointer.primary.pressed {
        memory.active = Some(id);
        memory.pressed = Some(id);
    }

    let active = memory.active == Some(id);
    let pressed = memory.pressed == Some(id) && input.pointer.primary.down;
    let clicked = !disabled && active && hovered && input.pointer.primary.released;
    let double_clicked = clicked && input.pointer.click_count >= 2;
    let secondary_clicked = !disabled && hovered && input.pointer.secondary.released;

    if active && input.pointer.primary.released {
        memory.clear_interaction();
    }

    Response {
        clicked,
        double_clicked,
        secondary_clicked,
        ..Response::new(
            id,
            rect,
            InteractionState {
                hovered,
                focused: memory.focused == Some(id),
                active,
                pressed,
                disabled,
                selected: false,
            },
        )
    }
}

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
        memory.focused = Some(id);
        response.state.focused = true;
    }

    response
}

/// Resolves neutral draggable behavior.
pub fn draggable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable(id, rect, input, memory, disabled);
    let active = memory.active == Some(id);

    response.dragged =
        !disabled && active && input.pointer.primary.down && input.pointer.delta != Vec2::ZERO;
    response.drag_delta = if response.dragged {
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

#[cfg(test)]
mod tests {
    use super::{draggable, focusable, hit_test, pressable, selectable};
    use crate::{Point, PointerButtonState, PointerInput, Rect, UiInput, UiMemory, Vec2, WidgetId};

    fn input_at(x: f32, y: f32) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    #[test]
    fn hit_testing_uses_rect_containment() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);

        assert!(hit_test(rect, &input_at(5.0, 5.0)));
        assert!(!hit_test(rect, &input_at(10.0, 5.0)));
    }

    #[test]
    fn pressable_tracks_hover_press_and_click() {
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();

        let mut input = input_at(5.0, 5.0);
        input.pointer.primary = PointerButtonState::new(true, true, false);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.state.hovered);
        assert!(response.state.active);
        assert!(response.state.pressed);

        input.pointer.primary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.clicked);
        assert_eq!(memory.active, None);
    }

    #[test]
    fn pressable_detects_double_and_secondary_clicks() {
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        input.pointer.click_count = 2;
        pressable(id, rect, &input, &mut memory, false);
        input.pointer.primary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.double_clicked);

        input.pointer.secondary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.secondary_clicked);
    }

    #[test]
    fn disabled_pressable_does_not_click() {
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);
        input.pointer.primary = PointerButtonState::new(false, false, true);

        let response = pressable(id, rect, &input, &mut memory, true);

        assert!(response.state.disabled);
        assert!(!response.clicked);
        assert!(!response.state.hovered);
    }

    #[test]
    fn focusable_sets_focus_on_click() {
        let id = WidgetId::from_key("field");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        focusable(id, rect, &input, &mut memory, false);
        input.pointer.primary = PointerButtonState::new(false, false, true);
        let response = focusable(id, rect, &input, &mut memory, false);

        assert!(response.state.focused);
        assert_eq!(memory.focused, Some(id));
    }

    #[test]
    fn draggable_reports_delta_while_active() {
        let id = WidgetId::from_key("handle");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        draggable(id, rect, &input, &mut memory, false);

        input.pointer.primary = PointerButtonState::new(true, false, false);
        input.pointer.position = Some(Point::new(20.0, 20.0));
        input.pointer.delta = Vec2::new(15.0, 15.0);
        let response = draggable(id, rect, &input, &mut memory, false);

        assert!(response.dragged);
        assert_eq!(response.drag_delta, Vec2::new(15.0, 15.0));
    }

    #[test]
    fn selectable_preserves_selected_state() {
        let id = WidgetId::from_key("row");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();

        let response = selectable(id, rect, &input_at(5.0, 5.0), &mut memory, true, false);

        assert!(response.state.selected);
        assert!(response.state.hovered);
    }
}
