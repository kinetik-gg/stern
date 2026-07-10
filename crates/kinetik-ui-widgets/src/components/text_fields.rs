use super::{
    ComponentState, CursorShape, OrderedTextInputResult, Primitive, Rect, RectPrimitive,
    TextEditMode, TextEditState, TextLayoutStore, TextSelection, Theme, UiInput, UiMemory,
    WidgetId, WidgetOutput, display_text_with_composition, focusable, multi_line_hit_offset,
    multi_line_text_primitives, single_line_hit_offset, single_line_text_primitives,
    text_field_layout, text_field_semantics, text_input_platform_requests, text_line_fragments,
    with_hover_cursor, with_response_state,
};

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
    text_field_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a single-line text field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn text_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
) -> TextFieldOutput {
    text_field_with_text_layouts_and_caret_visibility(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
        true,
    )
}

/// Emits a single-line text field with explicit caret visibility.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_text_layouts_and_caret_visibility(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> TextFieldOutput {
    text_field_with_text_layouts_and_caret_visibility_and_ordered_result(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
        caret_visible,
    )
    .0
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_text_layouts_and_caret_visibility_and_ordered_result(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    mut text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> (TextFieldOutput, OrderedTextInputResult) {
    let before = state.text.clone();
    let mut response = focusable(id, rect, input, memory, disabled);
    let hit_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (hit_text, _, _) = display_text_with_composition(state);
    if !disabled
        && response.state.hovered
        && input.pointer.primary.pressed
        && let Some(position) = input.pointer.position
    {
        let hit_layout = text_field_layout(
            text_layouts.as_deref_mut(),
            &hit_text,
            rect,
            &hit_recipe,
            false,
        );
        state.set_caret(single_line_hit_offset(
            position,
            rect,
            &hit_text,
            &hit_recipe,
            hit_layout,
        ));
        memory.focus(id);
        response.state.focused = true;
    }
    let mut platform_requests = text_input_platform_requests(id, rect, &response, memory);
    let mut ordered_result = OrderedTextInputResult::default();
    if response.state.focused
        && !disabled
        && memory.claim_text_input_events(id)
        && let Ok(events) = memory.effective_text_input_events(input)
    {
        ordered_result =
            state.apply_ordered_input_with_result(&events, id, TextEditMode::SingleLine);
        platform_requests.extend(ordered_result.platform_requests.iter().cloned());
    }
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (display_text, _, _) = display_text_with_composition(state);
    let layout = text_field_layout(text_layouts, &display_text, rect, &recipe, false);
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    primitives.extend(single_line_text_primitives(
        id,
        rect,
        state,
        response.state.focused && !disabled,
        caret_visible,
        &recipe,
        layout,
    ));

    (
        TextFieldOutput {
            widget: with_hover_cursor(
                WidgetOutput::new(Some(response), primitives)
                    .with_semantic(with_response_state(
                        text_field_semantics(id, rect, "Text field", state.text.clone(), disabled),
                        &response,
                    ))
                    .with_platform_requests(platform_requests),
                &response,
                CursorShape::Text,
            ),
            changed: before != state.text,
        },
        ordered_result,
    )
}

/// Output emitted by multi-line text fields.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiLineTextFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Whether the text changed this frame.
    pub changed: bool,
    /// Visible line count emitted by the widget.
    pub visible_lines: usize,
}

/// Parsed state of a numeric input draft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericInputDraft {
    /// The draft contains only whitespace.
    Empty,
    /// The draft parses as a numeric value.
    Valid(f32),
    /// The draft is non-empty and does not parse as a numeric value.
    Invalid,
}

impl NumericInputDraft {
    /// Returns the parsed value when the draft is valid and non-empty.
    #[must_use]
    pub const fn value(self) -> Option<f32> {
        match self {
            Self::Valid(value) => Some(value),
            Self::Empty | Self::Invalid => None,
        }
    }

    /// Returns true when the draft is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true when the draft is empty or valid.
    #[must_use]
    pub const fn is_acceptable(self) -> bool {
        matches!(self, Self::Empty | Self::Valid(_))
    }
}

/// Generic commit/revert policy emitted by numeric inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInputPolicy {
    /// Current draft classification.
    pub draft: NumericInputDraft,
    /// Whether the current frame requested committing a valid non-empty draft.
    pub commit_requested: bool,
    /// Whether the current frame requested reverting the draft to a caller-owned baseline.
    pub revert_requested: bool,
}

impl NumericInputPolicy {
    /// Creates a policy with no keyboard requests.
    #[must_use]
    pub const fn idle(draft: NumericInputDraft) -> Self {
        Self {
            draft,
            commit_requested: false,
            revert_requested: false,
        }
    }
}

/// Classifies numeric input draft text without mutating widget or application state.
#[must_use]
pub fn classify_numeric_input_draft(text: &str) -> NumericInputDraft {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        NumericInputDraft::Empty
    } else if let Ok(value) = trimmed.parse::<f32>() {
        NumericInputDraft::Valid(value)
    } else {
        NumericInputDraft::Invalid
    }
}

/// Restores a text-edit draft to a caller-owned baseline.
///
/// This helper is generic text state plumbing for commit/revert flows. It does
/// not parse, validate, or apply application-owned numeric values.
pub fn restore_text_draft(state: &mut TextEditState, draft: impl Into<String>) -> bool {
    let draft = draft.into();
    let caret = draft.len();
    let changed = state.text != draft
        || state.composition.is_some()
        || state.selection != TextSelection::new(caret, caret);

    state.text = draft;
    state.composition = None;
    state.set_selection(TextSelection::new(caret, caret));

    changed
}

/// Emits a multi-line text field and applies text input while focused.
pub fn multi_line_text_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> MultiLineTextFieldOutput {
    multi_line_text_field_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a multi-line text field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn multi_line_text_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
) -> MultiLineTextFieldOutput {
    multi_line_text_field_with_text_layouts_and_caret_visibility(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
        true,
    )
}

/// Emits a multi-line text field with explicit caret visibility.
#[allow(clippy::too_many_arguments)]
pub(crate) fn multi_line_text_field_with_text_layouts_and_caret_visibility(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    mut text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> MultiLineTextFieldOutput {
    let before = state.text.clone();
    let mut response = focusable(id, rect, input, memory, disabled);
    let hit_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (hit_text, _, _) = display_text_with_composition(state);
    if !disabled
        && response.state.hovered
        && input.pointer.primary.pressed
        && let Some(position) = input.pointer.position
    {
        let hit_layout = text_field_layout(
            text_layouts.as_deref_mut(),
            &hit_text,
            rect,
            &hit_recipe,
            true,
        );
        state.set_caret(multi_line_hit_offset(
            position,
            rect,
            &hit_text,
            &hit_recipe,
            hit_layout,
        ));
        memory.focus(id);
        response.state.focused = true;
    }
    let mut platform_requests = text_input_platform_requests(id, rect, &response, memory);
    if response.state.focused
        && !disabled
        && memory.claim_text_input_events(id)
        && let Ok(events) = memory.effective_text_input_events(input)
    {
        platform_requests.extend(state.apply_ordered_input(&events, id, TextEditMode::MultiLine));
    }
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (display_text, _, _) = display_text_with_composition(state);
    let layout = text_field_layout(text_layouts, &display_text, rect, &recipe, true);
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    primitives.extend(multi_line_text_primitives(
        id,
        rect,
        state,
        response.state.focused && !disabled,
        caret_visible,
        &recipe,
        layout,
    ));

    MultiLineTextFieldOutput {
        widget: with_hover_cursor(
            WidgetOutput::new(Some(response), primitives)
                .with_semantic(with_response_state(
                    text_field_semantics(id, rect, "Text field", state.text.clone(), disabled),
                    &response,
                ))
                .with_platform_requests(platform_requests),
            &response,
            CursorShape::Text,
        ),
        changed: before != state.text,
        visible_lines: text_line_fragments(&state.text).len(),
    }
}
