//! Reusable widgets built from Kinetik UI core primitives.

pub mod overlays;

pub use overlays::*;

use kinetik_ui_core::{
    Brush, ComponentState, CornerRadius, ImageId, ImagePrimitive, LinePrimitive, Point, Primitive,
    Rect, RectPrimitive, Response, Stroke, TextPrimitive, Theme, UiInput, UiMemory, WidgetId,
    draggable, fit_box, focusable, selectable,
};
use kinetik_ui_text::TextEditState;

/// Output emitted by a widget.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetOutput {
    /// Interaction response, when the widget is interactive.
    pub response: Option<Response>,
    /// Render primitives emitted by the widget.
    pub primitives: Vec<Primitive>,
}

impl WidgetOutput {
    /// Creates widget output.
    #[must_use]
    pub const fn new(response: Option<Response>, primitives: Vec<Primitive>) -> Self {
        Self {
            response,
            primitives,
        }
    }
}

/// Symbolic icon handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconId(u64);

impl IconId {
    /// Creates an icon ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Emits a text label.
#[must_use]
pub fn label(rect: Rect, text: impl Into<String>, theme: &Theme) -> WidgetOutput {
    WidgetOutput::new(
        None,
        vec![Primitive::Text(TextPrimitive {
            origin: Point::new(rect.x, rect.y + theme.text_size),
            text: text.into(),
            size: theme.text_size,
            brush: Brush::Solid(theme.colors.text),
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
    let response = focusable(id, rect, input, memory, disabled);
    let recipe = theme.button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let text = text.into();

    WidgetOutput::new(
        Some(response),
        vec![
            Primitive::Rect(RectPrimitive {
                rect,
                fill: Some(recipe.background),
                stroke: Some(recipe.border),
                radius: recipe.radius,
            }),
            Primitive::Text(TextPrimitive {
                origin: Point::new(rect.x + theme.spacing.md, rect.y + theme.text_size + 6.0),
                text,
                size: theme.text_size,
                brush: Brush::Solid(recipe.foreground),
            }),
        ],
    )
}

/// Emits an icon button.
pub fn icon_button(
    id: WidgetId,
    rect: Rect,
    _icon: IconId,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = focusable(id, rect, input, memory, disabled);
    let recipe = theme.button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let icon_rect = fit_box(
        rect,
        kinetik_ui_core::Size::new(14.0, 14.0),
        kinetik_ui_core::Alignment::Center,
        kinetik_ui_core::Alignment::Center,
    );

    WidgetOutput::new(
        Some(response),
        vec![
            Primitive::Rect(RectPrimitive {
                rect,
                fill: Some(recipe.background),
                stroke: Some(recipe.border),
                radius: recipe.radius,
            }),
            Primitive::Rect(RectPrimitive {
                rect: icon_rect,
                fill: Some(Brush::Solid(recipe.foreground)),
                stroke: None,
                radius: CornerRadius::all(1.0),
            }),
        ],
    )
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
    let response = selectable(id, rect, input, memory, checked, disabled);
    let box_rect = Rect::new(rect.x, rect.y, 14.0, 14.0);
    let fill = if checked {
        theme.colors.accent
    } else {
        theme.colors.surface_sunken
    };

    WidgetOutput::new(
        Some(response),
        vec![Primitive::Rect(RectPrimitive {
            rect: box_rect,
            fill: Some(Brush::Solid(fill)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(theme.colors.border))),
            radius: CornerRadius::all(2.0),
        })],
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
    let mut output = checkbox(id, rect, selected, input, memory, theme, disabled);
    if let Some(Primitive::Rect(primitive)) = output.primitives.first_mut() {
        primitive.radius = CornerRadius::all(7.0);
    }
    output
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
    let response = selectable(id, rect, input, memory, on, disabled);
    let fill = if on {
        theme.colors.accent
    } else {
        theme.colors.surface_active
    };
    let knob_x = if on {
        rect.max_x() - rect.height
    } else {
        rect.x
    };

    WidgetOutput::new(
        Some(response),
        vec![
            Primitive::Rect(RectPrimitive {
                rect,
                fill: Some(Brush::Solid(fill)),
                stroke: Some(Stroke::new(1.0, Brush::Solid(theme.colors.border))),
                radius: CornerRadius::all(rect.height * 0.5),
            }),
            Primitive::Rect(RectPrimitive {
                rect: Rect::new(
                    knob_x + 2.0,
                    rect.y + 2.0,
                    rect.height - 4.0,
                    rect.height - 4.0,
                ),
                fill: Some(Brush::Solid(theme.colors.text)),
                stroke: None,
                radius: CornerRadius::all((rect.height - 4.0) * 0.5),
            }),
        ],
    )
}

/// Emits a slider and updates its value while active.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = draggable(id, rect, input, memory, disabled);
    if !disabled
        && (response.state.active || response.clicked)
        && let Some(position) = input.pointer.position
    {
        let t = ((position.x - rect.x) / rect.width).clamp(0.0, 1.0);
        let start = *range.start();
        let end = *range.end();
        *value = start + (end - start) * t;
    }
    let start = *range.start();
    let end = *range.end();
    let t = ((*value - start) / (end - start)).clamp(0.0, 1.0);
    let fill_rect = Rect::new(rect.x, rect.y, rect.width * t, rect.height);

    WidgetOutput::new(
        Some(response),
        vec![
            Primitive::Rect(RectPrimitive {
                rect,
                fill: Some(Brush::Solid(theme.colors.surface_sunken)),
                stroke: Some(Stroke::new(1.0, Brush::Solid(theme.colors.border))),
                radius: CornerRadius::all(rect.height * 0.5),
            }),
            Primitive::Rect(RectPrimitive {
                rect: fill_rect,
                fill: Some(Brush::Solid(theme.colors.accent)),
                stroke: None,
                radius: CornerRadius::all(rect.height * 0.5),
            }),
        ],
    )
}

/// Emits a passive panel surface.
#[must_use]
pub fn panel(rect: Rect, theme: &Theme) -> WidgetOutput {
    WidgetOutput::new(
        None,
        vec![Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(theme.colors.surface_raised)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(theme.colors.border))),
            radius: theme.radius,
        })],
    )
}

/// Emits a simple horizontal separator line.
#[must_use]
pub fn separator(rect: Rect, theme: &Theme) -> Primitive {
    Primitive::Line(LinePrimitive {
        from: Point::new(rect.x, rect.center().y),
        to: Point::new(rect.max_x(), rect.center().y),
        stroke: Stroke::new(1.0, Brush::Solid(theme.colors.border_subtle)),
    })
}

/// Emits an image primitive for a static icon-like resource.
#[must_use]
pub fn image(rect: Rect, image: ImageId) -> WidgetOutput {
    WidgetOutput::new(None, vec![Primitive::Image(ImagePrimitive { image, rect })])
}

/// Output emitted by editable text widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Whether the text changed this frame.
    pub changed: bool,
}

/// Emits a single-line text field and applies text input while focused.
pub fn text_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> TextFieldOutput {
    let before = state.text.clone();
    let response = focusable(id, rect, input, memory, disabled);
    if response.state.focused && !disabled {
        state.apply_input(&input.text_events, &input.keyboard.events);
    }
    let border = if response.state.focused {
        theme.colors.accent
    } else {
        theme.colors.border
    };

    TextFieldOutput {
        widget: WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(Brush::Solid(theme.colors.surface_sunken)),
                    stroke: Some(Stroke::new(1.0, Brush::Solid(border))),
                    radius: theme.radius,
                }),
                Primitive::Text(TextPrimitive {
                    origin: Point::new(rect.x + theme.spacing.sm, rect.y + theme.text_size + 5.0),
                    text: state.text.clone(),
                    size: theme.text_size,
                    brush: Brush::Solid(theme.colors.text),
                }),
            ],
        ),
        changed: before != state.text,
    }
}

/// Output emitted by numeric input.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputOutput {
    /// Text field output.
    pub field: TextFieldOutput,
    /// Parsed numeric value, if valid.
    pub value: Option<f32>,
    /// Whether the current text parses as a number.
    pub valid: bool,
}

/// Emits a numeric input field.
pub fn numeric_input(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> NumericInputOutput {
    let field = text_field(id, rect, state, input, memory, theme, disabled);
    let value = state.text.trim().parse::<f32>().ok();

    NumericInputOutput {
        field,
        value,
        valid: value.is_some() || state.text.trim().is_empty(),
    }
}

/// Output emitted by search fields.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchFieldOutput {
    /// Text field output.
    pub field: TextFieldOutput,
    /// Current query.
    pub query: String,
    /// Whether the query is empty.
    pub empty: bool,
}

/// Emits a search-oriented text field.
pub fn search_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> SearchFieldOutput {
    let field = text_field(id, rect, state, input, memory, theme, disabled);
    let query = state.text.clone();

    SearchFieldOutput {
        field,
        empty: query.is_empty(),
        query,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IconId, button, checkbox, icon_button, image, label, numeric_input, panel, search_field,
        slider, text_field, toggle,
    };
    use kinetik_ui_core::{
        ImageId, Point, PointerButtonState, PointerInput, Primitive, Rect, UiInput, UiMemory,
        WidgetId, default_dark_theme,
    };
    use kinetik_ui_text::TextEditState;

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
    fn label_emits_text() {
        let output = label(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            "Name",
            &default_dark_theme(),
        );

        assert!(matches!(output.primitives[0], Primitive::Text(_)));
        assert!(output.response.is_none());
    }

    #[test]
    fn button_emits_surface_and_text_and_clicks() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut input = input_at(4.0, 4.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        button(id, rect, "Run", &input, &mut memory, &theme, false);
        input.pointer.primary = PointerButtonState::new(false, false, true);
        let output = button(id, rect, "Run", &input, &mut memory, &theme, false);

        assert_eq!(output.primitives.len(), 2);
        assert!(output.response.expect("button response").clicked);
    }

    #[test]
    fn icon_button_emits_icon_placeholder() {
        let output = icon_button(
            WidgetId::from_key("icon"),
            Rect::new(0.0, 0.0, 24.0, 24.0),
            IconId::from_raw(1),
            &UiInput::default(),
            &mut UiMemory::new(),
            &default_dark_theme(),
            false,
        );

        assert_eq!(output.primitives.len(), 2);
    }

    #[test]
    fn checkbox_and_toggle_reflect_selection() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let checkbox = checkbox(
            WidgetId::from_key("check"),
            Rect::new(0.0, 0.0, 20.0, 20.0),
            true,
            &input_at(1.0, 1.0),
            &mut memory,
            &theme,
            false,
        );
        let toggle = toggle(
            WidgetId::from_key("toggle"),
            Rect::new(0.0, 0.0, 36.0, 18.0),
            true,
            &UiInput::default(),
            &mut memory,
            &theme,
            false,
        );

        assert!(checkbox.response.expect("checkbox response").state.selected);
        assert_eq!(toggle.primitives.len(), 2);
    }

    #[test]
    fn slider_updates_value_from_pointer_position() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("slider");
        let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
        let mut memory = UiMemory::new();
        let mut value = 0.0;
        let mut input = input_at(50.0, 6.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        slider(
            id,
            rect,
            &mut value,
            0.0..=1.0,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!((value - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn panel_and_image_emit_single_primitives() {
        assert_eq!(
            panel(Rect::new(0.0, 0.0, 10.0, 10.0), &default_dark_theme())
                .primitives
                .len(),
            1
        );
        assert!(matches!(
            image(Rect::new(0.0, 0.0, 10.0, 10.0), ImageId::from_raw(1)).primitives[0],
            Primitive::Image(_)
        ));
    }

    #[test]
    fn text_field_applies_input_while_focused() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let mut memory = UiMemory::new();
        memory.focused = Some(id);
        let mut state = TextEditState::new("");
        let input = UiInput {
            text_events: vec![kinetik_ui_core::TextInputEvent::Commit("a".to_owned())],
            ..UiInput::default()
        };

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(output.changed);
        assert_eq!(state.text, "a");
    }

    #[test]
    fn numeric_input_reports_parse_state() {
        let theme = default_dark_theme();
        let mut state = TextEditState::new("42");
        let output = numeric_input(
            WidgetId::from_key("number"),
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
            false,
        );

        assert!(output.valid);
        assert_eq!(output.value, Some(42.0));
    }

    #[test]
    fn search_field_reports_query() {
        let theme = default_dark_theme();
        let mut state = TextEditState::new("media");
        let output = search_field(
            WidgetId::from_key("search"),
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
            false,
        );

        assert_eq!(output.query, "media");
        assert!(!output.empty);
    }
}
