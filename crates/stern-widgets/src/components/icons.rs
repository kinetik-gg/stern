use super::{
    ButtonFocusPlacement, ComponentState, CursorShape, IconPrimitive, ImageId, ImagePrimitive,
    Primitive, Rect, StaticIcon, Theme, UiInput, UiMemory, WidgetId, WidgetOutput,
    button_surface_primitives, clicked_select_state, fit_box, focusable, icon_button_semantics,
    response_reported_focus, response_reported_pressed, suppress_disabled_interaction_reporting,
    with_hover_cursor, with_response_state,
};

/// Emits an icon button with a required accessible label.
#[allow(clippy::too_many_arguments)]
pub fn icon_button(
    id: WidgetId,
    rect: Rect,
    icon: impl Into<StaticIcon>,
    label: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    static_icon_button(id, rect, icon.into(), label, input, memory, theme, disabled)
}

/// Emits an icon button backed by a bitmap image resource.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_button(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_button_sized(
        id,
        rect,
        image,
        label,
        theme.sizes.icon.md,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a bitmap icon button with an explicit icon side length.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_button_sized(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    icon_size: f32,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_selectable_button_sized(
        id, rect, image, label, false, icon_size, input, memory, theme, disabled,
    )
}

/// Emits a selectable icon button backed by a bitmap image resource.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_selectable_button(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_selectable_button_sized(
        id,
        rect,
        image,
        label,
        selected,
        theme.sizes.icon.md,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a selectable bitmap icon button with an explicit icon side length.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_selectable_button_sized(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    selected: bool,
    icon_size: f32,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = focusable(id, rect, input, memory, disabled);
    let selected = clicked_select_state(selected, response.clicked);
    response.state.selected = selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    };
    let recipe = theme.button(state);
    let icon_size = sanitized_icon_size(icon_size, theme.sizes.icon.md);
    let icon_rect = fit_box(
        rect,
        stern_core::Size::new(icon_size, icon_size),
        stern_core::Alignment::Center,
        stern_core::Alignment::Center,
    );
    let mut semantics = icon_button_semantics(id, rect, label, disabled);
    semantics.state.selected = selected;

    let mut primitives = button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        ButtonFocusPlacement::Inward,
    );
    primitives.push(Primitive::Image(ImagePrimitive {
        image,
        rect: icon_rect,
        tint: None,
    }));
    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

fn sanitized_icon_size(size: f32, fallback: f32) -> f32 {
    if size.is_finite() && size > 0.0 {
        size
    } else {
        fallback
    }
}

#[allow(clippy::too_many_arguments)]
fn static_icon_button(
    id: WidgetId,
    rect: Rect,
    icon: StaticIcon,
    label: impl Into<String>,
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
    let icon_rect = fit_box(
        rect,
        stern_core::Size::new(theme.sizes.icon.md, theme.sizes.icon.md),
        stern_core::Alignment::Center,
        stern_core::Alignment::Center,
    );
    let mut primitives = button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        ButtonFocusPlacement::Inward,
    );
    primitives.push(Primitive::Icon(IconPrimitive::new(
        icon,
        icon_rect,
        recipe.foreground,
    )));

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            icon_button_semantics(id, rect, label, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}
