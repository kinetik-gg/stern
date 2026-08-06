use super::common::push_focus_ring;
use super::vendored_icons::CHECK_ICON;
use super::{
    Brush, ComponentState, CornerRadius, CursorShape, IconPrimitive, Point, Primitive, Rect,
    RectPrimitive, TextPrimitive, TextRole, Theme, UiInput, UiMemory, WidgetId, WidgetOutput,
    checkbox_semantics, clicked_select_state, clicked_toggle_state, radio_button_semantics,
    response_reported_focus, response_reported_pressed, selectable,
    suppress_disabled_interaction_reporting, toggle_semantics, with_hover_cursor,
    with_response_state,
};

/// Inset from the 14x14 checkbox box to the painted check glyph, sized so the
/// Phosphor bold check reads as the spec's white 2px-stroke mark
/// (`docs/visual-spec/03-choice-sliders-tabs.md` §Checkbox checked row).
const CHECKBOX_GLYPH_INSET: f32 = 2.0;
/// Inset from the 14x14 radio circle to the checked dot: the spec's "inner
/// dot inset 3 (8px dot)" (`docs/visual-spec/03-choice-sliders-tabs.md` §Radio).
const RADIO_DOT_INSET: f32 = 3.0;
/// Gap between the control box and its label text, per
/// `docs/visual-spec/03-choice-sliders-tabs.md` ("gap 6 (label gap; labs 7 →
/// normalize 6)"). The token ladder has no 6px gap role, so the normative
/// spec value is pinned here.
const CHOICE_LABEL_GAP: f32 = 6.0;

/// Returns the region a choice-control label paints into.
///
/// An explicit non-empty `label_rect` wins. Otherwise the label paints to the
/// right of the control box inside the caller's control rect (KNOWN-GAPS #48:
/// the *_with_label APIs take a single rect, so the box-adjacent remainder is
/// the only deterministic label region without a layout engine). Returns
/// `None` when no room remains — notably for toggles without an explicit
/// label rect, whose track consumes the whole control rect.
fn choice_label_paint_region(control_rect: Rect, control_max_x: f32, label_rect: Rect) -> Option<Rect> {
    if !label_rect.is_empty() {
        return Some(label_rect);
    }
    let x = control_max_x + CHOICE_LABEL_GAP;
    let width = control_rect.max_x() - x;
    (width > 0.0 && control_rect.height > 0.0)
        .then(|| Rect::new(x, control_rect.y, width, control_rect.height))
}

/// Emits the control-type label per `docs/visual-spec/03-choice-sliders-tabs.md`:
/// label control type (11) `content.secondary` at rest, promoted to
/// `content.primary` on hover, `content.disabled` when disabled.
fn push_choice_label(
    primitives: &mut Vec<Primitive>,
    region: Option<Rect>,
    label: &str,
    theme: &Theme,
    hovered: bool,
    disabled: bool,
) {
    let Some(region) = region else {
        return;
    };
    if label.is_empty() {
        return;
    }
    let font = theme.font(TextRole::Label);
    let color = if disabled {
        theme.colors.content.disabled
    } else if hovered {
        theme.colors.content.primary
    } else {
        theme.colors.content.secondary
    };
    let baseline = region.y + (region.height - font.line_height).max(0.0) * 0.5 + font.size;
    primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(region.x, baseline),
        text: label.to_owned(),
        family: font.family.to_owned(),
        size: font.size,
        line_height: font.line_height,
        brush: Brush::Solid(color),
    }));
}

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

/// Emits a checkbox with an accessible label, painted to the right of the
/// box inside `rect` (gap 6, control type, secondary; visual-spec 03).
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
    let label = label.into();
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
    if display_selected {
        match kind {
            // Checked checkbox paints the check glyph in the recipe mark
            // color (white on accent; content.disabled when disabled) per
            // visual-spec 03 §Checkbox. The spec's mixed-state 6x2 bar is
            // not painted because no public mixed/indeterminate state
            // exists on this API yet.
            CheckControlKind::Checkbox => {
                primitives.push(Primitive::Icon(IconPrimitive::new(
                    CHECK_ICON,
                    box_rect.inset(CHECKBOX_GLYPH_INSET),
                    recipe.mark,
                )));
            }
            // Checked radio paints the neutral 8px inner dot per
            // visual-spec 03 §Radio.
            CheckControlKind::Radio => {
                let dot_rect = box_rect.inset(RADIO_DOT_INSET);
                primitives.push(Primitive::Rect(RectPrimitive {
                    rect: dot_rect,
                    fill: Some(Brush::Solid(recipe.mark)),
                    stroke: None,
                    radius: CornerRadius::all(dot_rect.width * 0.5),
                }));
            }
        }
    }
    push_choice_label(
        &mut primitives,
        choice_label_paint_region(rect, box_rect.max_x(), label_rect),
        &label,
        theme,
        response.state.hovered && !disabled,
        disabled,
    );
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

/// Emits a radio button with an accessible label, painted to the right of
/// the circle inside `rect` (gap 6, control type, secondary; visual-spec 03).
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
///
/// The track fills the whole control rect, so this variant has no room for a
/// visible label; use [`toggle_with_label_target`] with an explicit label
/// rect to paint one (KNOWN-GAPS #48).
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
///
/// The label paints into `label_rect` when it is non-empty. Unlike
/// checkbox/radio, a toggle given only a control rect has NO label region:
/// its track fills the whole control rect, so callers that want a visible
/// label must pass an explicit `label_rect` (KNOWN-GAPS #48).
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
    let label = label.into();
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
    push_choice_label(
        &mut primitives,
        choice_label_paint_region(rect, rect.max_x(), label_rect),
        &label,
        theme,
        response.state.hovered && !disabled,
        disabled,
    );

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            toggle_semantics(id, target_rect, label, selected, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}
