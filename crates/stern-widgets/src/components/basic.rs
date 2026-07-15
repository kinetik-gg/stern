use super::{
    Brush, ButtonFocusPlacement, ComponentState, CursorShape, Point, Primitive, Rect,
    RowFocusPlacement, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    TabFocusPlacement, TextPrimitive, TextRole, Theme, UiInput, UiMemory, WidgetId, WidgetOutput,
    button_semantics, button_surface_primitives, clicked_select_state, control_text_origin,
    focusable, label_baseline, response_reported_focus, response_reported_pressed,
    row_surface_primitives, selectable, suppress_disabled_interaction_reporting,
    tab_surface_primitives, with_hover_cursor, with_response_state,
};

/// Emits a text label.
#[must_use]
pub fn label(rect: Rect, text: impl Into<String>, theme: &Theme) -> WidgetOutput {
    let recipe = theme.label(TextRole::Body, false);
    WidgetOutput::new(
        None,
        vec![Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(rect.x, label_baseline(rect, theme, TextRole::Body)),
            text: text.into(),
            family: recipe.font.family.to_owned(),
            size: recipe.font.size,
            line_height: recipe.font.line_height,
            brush: Brush::Solid(recipe.foreground),
        })],
    )
}

/// Emits a push button.
pub fn button(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = focusable(id, rect, input, memory, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response_reported_pressed(&response),
        focused: response_reported_focus(&response),
        disabled,
        selected: false,
    };
    let recipe = theme.button(state);
    let text = text.into();
    let mut primitives = button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        ButtonFocusPlacement::Inward,
    );
    primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: control_text_origin(rect, theme),
        text: text.clone(),
        family: theme.font(TextRole::Label).family.to_owned(),
        size: theme.font(TextRole::Label).size,
        line_height: theme.font(TextRole::Label).line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            button_semantics(id, rect, text, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a tab header.
#[allow(clippy::too_many_arguments)]
pub fn tab_button(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = selectable(id, rect, input, memory, selected, disabled);
    let selected = clicked_select_state(selected, response.clicked);
    response.state.selected = selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    };
    let recipe = theme.tab(state);
    let text = text.into();

    let mut semantics = SemanticNode::new(id, SemanticRole::Tab, rect)
        .with_label(text.clone())
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    semantics.state.disabled = disabled;
    semantics.state.selected = selected;

    let mut primitives = tab_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        TabFocusPlacement::Inward,
    );
    primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: control_text_origin(rect, theme),
        text,
        family: theme.font(TextRole::Label).family.to_owned(),
        size: theme.font(TextRole::Label).size,
        line_height: theme.font(TextRole::Label).line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a selectable list or table row surface.
#[allow(clippy::too_many_arguments)]
pub fn list_row(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = selectable(id, rect, input, memory, selected, disabled);
    let selected = clicked_select_state(selected, response.clicked);
    response.state.selected = selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    };
    let recipe = theme.row(state);
    let text = text.into();

    let mut semantics = SemanticNode::new(id, SemanticRole::ListItem, rect)
        .with_label(text.clone())
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    semantics.state.disabled = disabled;
    semantics.state.selected = selected;

    let mut primitives = row_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        RowFocusPlacement::Inward,
    );
    primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: control_text_origin(rect, theme),
        text,
        family: theme.font(TextRole::Label).family.to_owned(),
        size: theme.font(TextRole::Label).size,
        line_height: theme.font(TextRole::Label).line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}
