use super::common::push_focus_ring;
use super::{
    ComponentState, CornerRadius, CursorShape, Primitive, Rect, RectPrimitive, Theme, UiInput,
    UiMemory, WidgetId, WidgetOutput, checkbox_semantics, clicked_select_state,
    clicked_toggle_state, radio_button_semantics, response_reported_focus,
    response_reported_pressed, selectable, suppress_disabled_interaction_reporting,
    toggle_semantics, with_hover_cursor, with_response_state,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckControlKind {
    Checkbox,
    Radio,
}

/// Returns the deterministic activation target for a choice control and its label.
#[must_use]
pub fn choice_label_target_rect(control_rect: Rect, label_rect: Rect) -> Rect {
    control_rect.union(label_rect)
}

/// Emits a checkbox.
pub fn checkbox(
    id: WidgetId,
    rect: Rect,
    checked: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    checkbox_with_label(
        id, rect, "Checkbox", checked, input, memory, theme, disabled,
    )
}

/// Emits a checkbox with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn checkbox_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    checked: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    checkbox_with_label_target(
        id,
        rect,
        Rect::ZERO,
        label,
        checked,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a checkbox with a deterministic label activation target.
#[allow(clippy::too_many_arguments)]
pub fn checkbox_with_label_target(
    id: WidgetId,
    rect: Rect,
    label_rect: Rect,
    label: impl Into<String>,
    checked: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    check_control_with_label_target(
        id,
        rect,
        label_rect,
        label,
        checked,
        input,
        memory,
        theme,
        disabled,
        CheckControlKind::Checkbox,
    )
}

#[allow(clippy::too_many_arguments)]
fn check_control_with_label_target(
    id: WidgetId,
    rect: Rect,
    label_rect: Rect,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    kind: CheckControlKind,
) -> WidgetOutput {
    let target_rect = choice_label_target_rect(rect, label_rect);
    let mut response = selectable(id, target_rect, input, memory, selected, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let display_selected = match kind {
        CheckControlKind::Checkbox => clicked_toggle_state(selected, response.clicked),
        CheckControlKind::Radio => clicked_select_state(selected, response.clicked),
    };
    response.state.selected = display_selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response_reported_pressed(&response),
        focused: response_reported_focus(&response),
        disabled,
        selected: display_selected,
    };
    let recipe = match kind {
        CheckControlKind::Checkbox => theme.checkbox(state),
        CheckControlKind::Radio => theme.radio_button(state),
    };
    let box_rect = Rect::new(rect.x, rect.y, recipe.size, recipe.size);
    let mut primitives = Vec::with_capacity(3);
    push_focus_ring(
        &mut primitives,
        theme,
        response_reported_focus(&response),
        box_rect,
        recipe.radius,
    );
    primitives.push(Primitive::Rect(RectPrimitive {
        rect: box_rect,
        fill: Some(recipe.fill),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    }));
    let semantics = match kind {
        CheckControlKind::Checkbox => {
            checkbox_semantics(id, target_rect, label, display_selected, disabled)
        }
        CheckControlKind::Radio => {
            radio_button_semantics(id, target_rect, label, display_selected, disabled)
        }
    };

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a radio button.
pub fn radio_button(
    id: WidgetId,
    rect: Rect,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    radio_button_with_label(
        id,
        rect,
        "Radio button",
        selected,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a radio button with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn radio_button_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    radio_button_with_label_target(
        id,
        rect,
        Rect::ZERO,
        label,
        selected,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a radio button with a deterministic label activation target.
#[allow(clippy::too_many_arguments)]
pub fn radio_button_with_label_target(
    id: WidgetId,
    rect: Rect,
    label_rect: Rect,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    check_control_with_label_target(
        id,
        rect,
        label_rect,
        label,
        selected,
        input,
        memory,
        theme,
        disabled,
        CheckControlKind::Radio,
    )
}

/// Emits a toggle control.
pub fn toggle(
    id: WidgetId,
    rect: Rect,
    on: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    toggle_with_label(id, rect, "Toggle", on, input, memory, theme, disabled)
}

/// Emits a toggle control with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn toggle_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    on: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    toggle_with_label_target(
        id,
        rect,
        Rect::ZERO,
        label,
        on,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a toggle control with a deterministic label activation target.
#[allow(clippy::too_many_arguments)]
pub fn toggle_with_label_target(
    id: WidgetId,
    rect: Rect,
    label_rect: Rect,
    label: impl Into<String>,
    on: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let target_rect = choice_label_target_rect(rect, label_rect);
    let mut response = selectable(id, target_rect, input, memory, on, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let selected = clicked_toggle_state(on, response.clicked);
    response.state.selected = selected;
    let recipe = theme.toggle(ComponentState {
        hovered: response.state.hovered,
        pressed: response_reported_pressed(&response),
        focused: response_reported_focus(&response),
        disabled,
        selected,
    });
    let knob_x = if selected {
        rect.max_x() - rect.height
    } else {
        rect.x
    };
    let radius = CornerRadius::all(rect.height * 0.5);
    let mut primitives = Vec::with_capacity(4);
    push_focus_ring(
        &mut primitives,
        theme,
        response_reported_focus(&response),
        rect,
        radius,
    );
    primitives.extend([
        Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.track),
            stroke: Some(recipe.border),
            radius,
        }),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(
                knob_x + recipe.padding,
                rect.y + recipe.padding,
                rect.height - recipe.padding * 2.0,
                rect.height - recipe.padding * 2.0,
            ),
            fill: Some(recipe.thumb),
            stroke: None,
            radius: CornerRadius::all((rect.height - recipe.padding * 2.0) * 0.5),
        }),
    ]);

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            toggle_semantics(id, target_rect, label, selected, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}
