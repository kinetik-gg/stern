//! Visually neutral interaction primitives.

use crate::{Key, KeyState, Rect, Size, UiInput, UiMemory, Vec2, WidgetId};

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
    /// Whether the widget was activated by keyboard this frame.
    pub keyboard_activated: bool,
    /// Whether the widget requested a context menu this frame.
    pub context_requested: bool,
    /// Whether the widget requested a tooltip this frame.
    pub tooltip_requested: bool,
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
            keyboard_activated: false,
            context_requested: false,
            tooltip_requested: false,
            drag_delta: Vec2::ZERO,
        }
    }
}

/// Scrollable behavior output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollResponse {
    /// Common interaction response for the scrollable region.
    pub response: Response,
    /// Clamped retained scroll offset after applying input.
    pub offset: Vec2,
    /// Offset delta applied during this frame.
    pub delta: Vec2,
    /// Maximum legal scroll offset.
    pub max_offset: Vec2,
}

/// Drop target behavior output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropTargetResponse {
    /// Common interaction response for the target region.
    pub response: Response,
    /// Captured drag source, if a different widget owns pointer capture.
    pub source: Option<WidgetId>,
    /// Whether a captured source was released over this target this frame.
    pub dropped: bool,
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
    let hovered = !disabled && routed_hit_test(id, rect, input, memory);

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
        (!disabled && active && hovered && input.pointer.primary.released) || keyboard_activated;
    let double_clicked = clicked && input.pointer.click_count >= 2;
    let secondary_clicked =
        !disabled && hovered && memory.is_secondary_pressed(id) && input.pointer.secondary.released;

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

/// Resolves neutral scroll behavior and stores a clamped offset in memory.
///
/// Wheel deltas follow the platform input convention. The retained scroll
/// offset increases in the opposite direction so a negative vertical wheel
/// delta, the usual "scroll down" event, moves content down by increasing the
/// stored y offset.
pub fn scrollable(
    id: WidgetId,
    rect: Rect,
    content_size: Size,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> ScrollResponse {
    let hovered = !disabled && routed_hit_test(id, rect, input, memory);
    if hovered {
        memory.set_hovered(id);
    }

    let previous = clamp_scroll_offset(memory.scroll_offset(id), rect.size(), content_size);
    let requested_delta = if hovered {
        Vec2::new(-input.pointer.wheel_delta.x, -input.pointer.wheel_delta.y)
    } else {
        Vec2::ZERO
    };
    let offset = clamp_scroll_offset(
        Vec2::new(
            previous.x + requested_delta.x,
            previous.y + requested_delta.y,
        ),
        rect.size(),
        content_size,
    );
    memory.set_scroll_offset(id, offset);

    let delta = Vec2::new(offset.x - previous.x, offset.y - previous.y);
    let max_offset = max_scroll_offset(rect.size(), content_size);
    let mut response = Response::new(
        id,
        rect,
        InteractionState {
            hovered,
            focused: memory.is_focused(id),
            active: false,
            pressed: false,
            disabled,
            selected: false,
        },
    );
    response.drag_delta = delta;

    ScrollResponse {
        response,
        offset,
        delta,
        max_offset,
    }
}

/// Resolves neutral context-menu trigger behavior.
pub fn context_menu_trigger(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable(id, rect, input, memory, disabled);
    response.context_requested =
        !disabled && (response.secondary_clicked || keyboard_context_requested(id, input, memory));
    response
}

/// Resolves neutral tooltip trigger behavior.
pub fn tooltip_trigger(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let hovered = !disabled && routed_hit_test(id, rect, input, memory);
    if hovered {
        memory.set_hovered(id);
    }

    let mut response = Response::new(
        id,
        rect,
        InteractionState {
            hovered,
            focused: memory.is_focused(id),
            active: false,
            pressed: false,
            disabled,
            selected: false,
        },
    );
    response.tooltip_requested = hovered
        && !input.pointer.primary.down
        && !input.pointer.secondary.down
        && !input.pointer.middle.down;
    response
}

/// Resolves neutral drop-target behavior for active drags.
pub fn drop_target(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> DropTargetResponse {
    let source = memory
        .released_drag_source()
        .or_else(|| memory.drag_source())
        .filter(|source| *source != id);
    let release_drop_hit = input.pointer.primary.released && source.is_some();
    let hovered = !disabled
        && if release_drop_hit {
            hit_test(rect, input)
        } else {
            routed_hit_test(id, rect, input, memory)
        };
    if hovered {
        memory.set_hovered(id);
    }
    let dropped = !disabled && hovered && input.pointer.primary.released && source.is_some();
    let response = Response::new(
        id,
        rect,
        InteractionState {
            hovered,
            focused: memory.is_focused(id),
            active: false,
            pressed: false,
            disabled,
            selected: false,
        },
    );

    DropTargetResponse {
        response,
        source,
        dropped,
    }
}

/// Clamps a scroll offset to the range implied by a viewport and content size.
#[must_use]
pub fn clamp_scroll_offset(offset: Vec2, viewport_size: Size, content_size: Size) -> Vec2 {
    let max_offset = max_scroll_offset(viewport_size, content_size);
    Vec2::new(
        sanitize_scroll_component(offset.x).clamp(0.0, max_offset.x),
        sanitize_scroll_component(offset.y).clamp(0.0, max_offset.y),
    )
}

/// Returns the maximum legal scroll offset for a viewport and content size.
#[must_use]
pub fn max_scroll_offset(viewport_size: Size, content_size: Size) -> Vec2 {
    Vec2::new(
        (sanitize_scroll_component(content_size.width)
            - sanitize_scroll_component(viewport_size.width))
        .max(0.0),
        (sanitize_scroll_component(content_size.height)
            - sanitize_scroll_component(viewport_size.height))
        .max(0.0),
    )
}

fn keyboard_activation_pressed(id: WidgetId, input: &UiInput, memory: &UiMemory) -> bool {
    memory.is_focused(id)
        && input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && event.modifiers.is_empty()
                && matches!(event.key, Key::Enter | Key::Space)
        })
}

fn routed_hit_test(id: WidgetId, rect: Rect, input: &UiInput, memory: &UiMemory) -> bool {
    memory
        .pointer_capture()
        .is_none_or(|captured| captured == id)
        && hit_test(rect, input)
}

fn keyboard_context_requested(id: WidgetId, input: &UiInput, memory: &UiMemory) -> bool {
    memory.is_focused(id)
        && input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && event.modifiers.shift
                && matches!(event.key, Key::Function(10))
        })
}

fn sanitize_scroll_component(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
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

#[cfg(test)]
mod tests {
    use super::{
        clamp_scroll_offset, context_menu_trigger, draggable, drop_target, focusable, hit_test,
        max_scroll_offset, pressable, scrollable, selectable, tooltip_trigger,
    };
    use crate::Size;
    use crate::{
        Key, KeyEvent, KeyState, Modifiers, Point, PointerButtonState, PointerInput, Rect, UiInput,
        UiMemory, Vec2, WidgetId,
    };

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
        assert!(memory.has_pointer_capture(id));

        input.pointer.primary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.clicked);
        assert!(!response.state.active);
        assert!(!response.state.pressed);
        assert_eq!(memory.active(), None);
        assert_eq!(memory.pointer_capture(), None);
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

        input.pointer.primary = PointerButtonState::default();
        input.pointer.secondary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(!response.secondary_clicked);

        input.pointer.secondary = PointerButtonState::new(true, true, false);
        pressable(id, rect, &input, &mut memory, false);
        input.pointer.secondary = PointerButtonState::new(false, false, true);
        let response = pressable(id, rect, &input, &mut memory, false);
        assert!(response.secondary_clicked);
    }

    #[test]
    fn pointer_capture_suppresses_hover_on_other_widgets() {
        let owner = WidgetId::from_key("owner");
        let other = WidgetId::from_key("other");
        let owner_rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let other_rect = Rect::new(20.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        let owner_press = pressable(owner, owner_rect, &input, &mut memory, false);
        assert!(owner_press.state.hovered);
        assert!(memory.has_pointer_capture(owner));

        memory.begin_frame();
        input.pointer.position = Some(Point::new(25.0, 5.0));
        input.pointer.primary = PointerButtonState::new(true, false, false);
        let other_response = pressable(other, other_rect, &input, &mut memory, false);

        assert!(!other_response.state.hovered);
        assert_eq!(memory.hovered(), None);
        assert_eq!(memory.pointer_capture(), Some(owner));
    }

    #[test]
    fn secondary_click_requires_matching_press_owner() {
        let id = WidgetId::from_key("button");
        let other = WidgetId::from_key("other");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);
        memory.press_secondary(other);
        input.pointer.secondary = PointerButtonState::new(false, false, true);

        let response = pressable(id, rect, &input, &mut memory, false);

        assert!(!response.secondary_clicked);
        assert_eq!(memory.secondary_pressed(), Some(other));
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
        assert_eq!(memory.focused(), Some(id));
    }

    #[test]
    fn focused_pressable_activates_from_keyboard() {
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        memory.focus(id);
        let input = UiInput {
            keyboard: crate::KeyboardInput {
                modifiers: Modifiers::default(),
                events: vec![KeyEvent::new(
                    Key::Space,
                    KeyState::Pressed,
                    Modifiers::default(),
                    false,
                )],
            },
            ..UiInput::default()
        };

        let response = pressable(id, rect, &input, &mut memory, false);

        assert!(response.clicked);
        assert!(response.keyboard_activated);
        assert!(!response.state.pressed);
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
        assert_eq!(memory.drag_source(), Some(id));
    }

    #[test]
    fn draggable_finishes_drag_on_release_for_drop_targets() {
        let id = WidgetId::from_key("handle");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        draggable(id, rect, &input, &mut memory, false);
        input.pointer.primary = PointerButtonState::new(true, false, false);
        input.pointer.delta = Vec2::new(1.0, 0.0);
        draggable(id, rect, &input, &mut memory, false);

        input.pointer.primary = PointerButtonState::new(false, false, true);
        draggable(id, rect, &input, &mut memory, false);

        assert_eq!(memory.drag_source(), None);
        assert_eq!(memory.released_drag_source(), Some(id));
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

    #[test]
    fn scroll_helpers_clamp_offsets_to_content_bounds() {
        assert_eq!(
            max_scroll_offset(Size::new(100.0, 80.0), Size::new(140.0, 200.0)),
            Vec2::new(40.0, 120.0)
        );
        assert_eq!(
            clamp_scroll_offset(
                Vec2::new(f32::INFINITY, -20.0),
                Size::new(100.0, 80.0),
                Size::new(140.0, 200.0),
            ),
            Vec2::new(0.0, 0.0)
        );
        assert_eq!(
            clamp_scroll_offset(
                Vec2::new(90.0, 180.0),
                Size::new(100.0, 80.0),
                Size::new(140.0, 200.0),
            ),
            Vec2::new(40.0, 120.0)
        );
    }

    #[test]
    fn scrollable_applies_hovered_wheel_delta_and_stores_offset() {
        let id = WidgetId::from_key("scroll");
        let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(20.0, 20.0);
        input.pointer.wheel_delta = Vec2::new(0.0, -30.0);

        let output = scrollable(
            id,
            rect,
            Size::new(100.0, 200.0),
            &input,
            &mut memory,
            false,
        );

        assert!(output.response.state.hovered);
        assert_eq!(output.offset, Vec2::new(0.0, 30.0));
        assert_eq!(output.delta, Vec2::new(0.0, 30.0));
        assert_eq!(memory.scroll_offset(id), Vec2::new(0.0, 30.0));
    }

    #[test]
    fn scrollable_ignores_wheel_when_not_hovered_or_disabled() {
        let id = WidgetId::from_key("scroll");
        let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
        let mut memory = UiMemory::new();
        memory.set_scroll_offset(id, Vec2::new(0.0, 40.0));
        let mut input = input_at(120.0, 20.0);
        input.pointer.wheel_delta = Vec2::new(0.0, -30.0);

        let output = scrollable(
            id,
            rect,
            Size::new(100.0, 200.0),
            &input,
            &mut memory,
            false,
        );

        assert!(!output.response.state.hovered);
        assert_eq!(output.offset, Vec2::new(0.0, 40.0));

        let disabled = scrollable(
            id,
            rect,
            Size::new(100.0, 200.0),
            &input_at(20.0, 20.0),
            &mut memory,
            true,
        );
        assert!(disabled.response.state.disabled);
        assert_eq!(disabled.offset, Vec2::new(0.0, 40.0));
    }

    #[test]
    fn context_menu_trigger_uses_secondary_click_and_shift_f10() {
        let id = WidgetId::from_key("menu");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(5.0, 5.0);
        input.pointer.secondary = PointerButtonState::new(true, true, false);
        context_menu_trigger(id, rect, &input, &mut memory, false);
        input.pointer.secondary = PointerButtonState::new(false, false, true);

        let pointer = context_menu_trigger(id, rect, &input, &mut memory, false);
        assert!(pointer.context_requested);

        memory.focus(id);
        let input = UiInput {
            keyboard: crate::KeyboardInput {
                modifiers: Modifiers::new(true, false, false, false),
                events: vec![KeyEvent::new(
                    Key::Function(10),
                    KeyState::Pressed,
                    Modifiers::new(true, false, false, false),
                    false,
                )],
            },
            ..UiInput::default()
        };
        let keyboard = context_menu_trigger(id, rect, &input, &mut memory, false);
        assert!(keyboard.context_requested);
    }

    #[test]
    fn tooltip_trigger_reports_idle_hover_only() {
        let id = WidgetId::from_key("tip");
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let mut memory = UiMemory::new();

        let response = tooltip_trigger(id, rect, &input_at(5.0, 5.0), &mut memory, false);
        assert!(response.tooltip_requested);

        let mut pressed = input_at(5.0, 5.0);
        pressed.pointer.primary = PointerButtonState::new(true, true, false);
        let response = tooltip_trigger(id, rect, &pressed, &mut memory, false);
        assert!(!response.tooltip_requested);
    }

    #[test]
    fn drop_target_reports_drag_source_released_over_target() {
        let source = WidgetId::from_key("source");
        let target = WidgetId::from_key("target");
        let source_rect = Rect::new(30.0, 0.0, 20.0, 20.0);
        let target_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
        let mut memory = UiMemory::new();
        let mut input = input_at(35.0, 5.0);
        input.pointer.primary = PointerButtonState::new(true, true, false);
        draggable(source, source_rect, &input, &mut memory, false);

        input.pointer.primary = PointerButtonState::new(true, false, false);
        input.pointer.delta = Vec2::new(5.0, 0.0);
        draggable(source, source_rect, &input, &mut memory, false);

        input.pointer.position = Some(Point::new(5.0, 5.0));
        input.pointer.primary = PointerButtonState::new(false, false, true);

        let output = drop_target(target, target_rect, &input, &mut memory, false);

        assert_eq!(output.source, Some(source));
        assert!(output.dropped);
        assert!(output.response.state.hovered);
    }

    #[test]
    fn drop_target_does_not_accept_plain_pointer_capture() {
        let source = WidgetId::from_key("source");
        let target = WidgetId::from_key("target");
        let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
        let mut memory = UiMemory::new();
        memory.capture_pointer(source);
        let mut input = input_at(5.0, 5.0);
        input.pointer.primary = PointerButtonState::new(false, false, true);

        let output = drop_target(target, rect, &input, &mut memory, false);

        assert_eq!(output.source, None);
        assert!(!output.dropped);
    }

    #[test]
    fn drop_target_ignores_self_capture_disabled_and_misses() {
        let target = WidgetId::from_key("target");
        let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
        let mut memory = UiMemory::new();
        memory.start_drag(target);
        let mut input = input_at(5.0, 5.0);
        input.pointer.primary = PointerButtonState::new(false, false, true);

        let own_capture = drop_target(target, rect, &input, &mut memory, false);
        assert_eq!(own_capture.source, None);
        assert!(!own_capture.dropped);

        memory.start_drag(WidgetId::from_key("source"));
        let disabled = drop_target(target, rect, &input, &mut memory, true);
        assert!(!disabled.dropped);

        let missed = drop_target(target, rect, &released_at(40.0, 40.0), &mut memory, false);
        assert!(!missed.dropped);
    }

    fn released_at(x: f32, y: f32) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                primary: PointerButtonState::new(false, false, true),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }
}
