use super::{
    DEFAULT_NUMERIC_SCRUB_COARSE_FACTOR, DEFAULT_NUMERIC_SCRUB_FINE_FACTOR,
    DEFAULT_NUMERIC_SCRUB_STEP, NumericInputDraft, NumericInputPolicy, OrderedTextInputResult,
    Rect, Response, SemanticAction, SemanticActionKind, SemanticValue, TextEditState,
    TextFieldOutput, TextLayoutStore, Theme, UiInput, UiMemory, WidgetId,
    classify_numeric_input_draft, draggable, restore_text_draft,
    text_field_with_text_layouts_and_caret_visibility_and_ordered_result,
};

/// Output emitted by numeric input.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputOutput {
    /// Text field output.
    pub field: TextFieldOutput,
    /// Draft classification and keyboard commit/revert requests.
    pub policy: NumericInputPolicy,
    /// Parsed numeric value, if valid.
    pub value: Option<f32>,
    /// Whether the current draft is empty or parses as a number.
    pub valid: bool,
}

/// Configuration for numeric scrub inputs.
///
/// The base, fine, and coarse steps describe the value change produced by one
/// logical unit of horizontal pointer movement. Non-finite or non-positive
/// steps are replaced with deterministic defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericScrubInputConfig {
    /// Base step used with no modifiers.
    pub base_step: f32,
    /// Fine step used while Shift is held.
    pub fine_step: f32,
    /// Coarse step used while Ctrl or Super is held.
    pub coarse_step: f32,
    /// Optional lower clamp bound.
    pub min: Option<f32>,
    /// Optional upper clamp bound.
    pub max: Option<f32>,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is displayed but not editable.
    pub read_only: bool,
}

impl NumericScrubInputConfig {
    /// Creates a numeric scrub configuration with deterministic default fine
    /// and coarse steps derived from the base step.
    #[must_use]
    pub const fn new(base_step: f32) -> Self {
        Self {
            base_step,
            fine_step: base_step * DEFAULT_NUMERIC_SCRUB_FINE_FACTOR,
            coarse_step: base_step * DEFAULT_NUMERIC_SCRUB_COARSE_FACTOR,
            min: None,
            max: None,
            disabled: false,
            read_only: false,
        }
    }

    /// Sets the fine scrub step.
    #[must_use]
    pub const fn with_fine_step(mut self, fine_step: f32) -> Self {
        self.fine_step = fine_step;
        self
    }

    /// Sets the coarse scrub step.
    #[must_use]
    pub const fn with_coarse_step(mut self, coarse_step: f32) -> Self {
        self.coarse_step = coarse_step;
        self
    }

    /// Sets an inclusive clamp range.
    #[must_use]
    pub const fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Sets a lower clamp bound.
    #[must_use]
    pub const fn with_min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    /// Sets an upper clamp bound.
    #[must_use]
    pub const fn with_max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets whether the field is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether the field is read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
}

impl Default for NumericScrubInputConfig {
    fn default() -> Self {
        Self::new(DEFAULT_NUMERIC_SCRUB_STEP)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResolvedNumericScrubConfig {
    base_step: f32,
    fine_step: f32,
    coarse_step: f32,
    min: Option<f32>,
    max: Option<f32>,
}

/// Output emitted by numeric scrub inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericScrubInputOutput {
    /// Numeric text input output.
    pub input: NumericInputOutput,
    /// Scrub interaction response.
    pub scrub_response: Response,
    /// Value after any accepted scrub mutation.
    pub value: f32,
    /// Step selected for this frame.
    pub step: f32,
    /// Sanitized lower clamp bound.
    pub min: Option<f32>,
    /// Sanitized upper clamp bound.
    pub max: Option<f32>,
    /// Whether horizontal scrubbing changed the value this frame.
    pub scrubbed: bool,
    /// Whether the value changed this frame.
    pub value_changed: bool,
    /// Whether the field is read-only.
    pub read_only: bool,
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
    numeric_input_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a numeric input field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn numeric_input_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
) -> NumericInputOutput {
    numeric_input_with_text_layouts_and_caret_visibility(
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

/// Emits a numeric input field with explicit caret visibility.
#[allow(clippy::too_many_arguments)]
pub(crate) fn numeric_input_with_text_layouts_and_caret_visibility(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> NumericInputOutput {
    let (field, ordered_result) =
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
        );
    let draft = classify_numeric_input_draft(&state.text);
    let policy = numeric_input_keyboard_policy(draft, &field, &ordered_result, disabled);

    NumericInputOutput {
        field,
        policy,
        value: draft.value(),
        valid: draft.is_acceptable(),
    }
}

/// Emits a numeric text field with horizontal scrub adjustment.
#[allow(clippy::too_many_arguments)]
pub fn numeric_scrub_input(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    state: &mut TextEditState,
    config: NumericScrubInputConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> NumericScrubInputOutput {
    numeric_scrub_input_with_text_layouts_and_caret_visibility(
        id, rect, value, state, config, input, memory, theme, None, true,
    )
}

/// Emits a numeric scrub input using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn numeric_scrub_input_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    state: &mut TextEditState,
    config: NumericScrubInputConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
) -> NumericScrubInputOutput {
    numeric_scrub_input_with_text_layouts_and_caret_visibility(
        id,
        rect,
        value,
        state,
        config,
        input,
        memory,
        theme,
        text_layouts,
        true,
    )
}

/// Emits a numeric scrub input with explicit caret visibility.
#[allow(clippy::too_many_arguments)]
pub(crate) fn numeric_scrub_input_with_text_layouts_and_caret_visibility(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    state: &mut TextEditState,
    config: NumericScrubInputConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> NumericScrubInputOutput {
    let interactions_disabled = config.disabled || config.read_only;
    let before = *value;
    let resolved = resolve_numeric_scrub_config(config);
    let mut numeric = numeric_input_with_text_layouts_and_caret_visibility(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        interactions_disabled,
        text_layouts,
        caret_visible,
    );
    let scrub_response = draggable(id, rect, input, memory, interactions_disabled);
    let selected_step = numeric_scrub_step_for_modifiers(&resolved, input.keyboard.modifiers);
    let mut scrubbed = false;

    if !interactions_disabled
        && scrub_response.dragged
        && scrub_response.drag_delta.x.is_finite()
        && let NumericInputDraft::Valid(current) = numeric.policy.draft
        && current.is_finite()
    {
        let next = clamp_numeric_scrub_value(
            current + scrub_response.drag_delta.x * selected_step,
            resolved.min,
            resolved.max,
        );
        if numeric_scrub_value_changed(*value, next) {
            *value = next;
            restore_text_draft(state, format_numeric_scrub_value(next));
            numeric.policy.draft = classify_numeric_input_draft(&state.text);
            numeric.value = numeric.policy.draft.value();
            numeric.valid = numeric.policy.draft.is_acceptable();
            numeric.field.changed = true;
            scrubbed = true;
        }
    }

    apply_numeric_scrub_semantics(&mut numeric, resolved.min, resolved.max);

    NumericScrubInputOutput {
        input: numeric,
        scrub_response,
        value: *value,
        step: selected_step,
        min: resolved.min,
        max: resolved.max,
        scrubbed,
        value_changed: numeric_scrub_value_changed(before, *value),
        read_only: config.read_only,
    }
}

fn numeric_input_keyboard_policy(
    draft: NumericInputDraft,
    field: &TextFieldOutput,
    ordered_result: &OrderedTextInputResult,
    disabled: bool,
) -> NumericInputPolicy {
    let Some(response) = field.widget.response.as_ref() else {
        return NumericInputPolicy::idle(draft);
    };
    if disabled || !response.state.focused || response.state.disabled {
        return NumericInputPolicy::idle(draft);
    }

    let mut policy = NumericInputPolicy::idle(draft);
    policy.commit_requested =
        ordered_result.commit_requested && matches!(draft, NumericInputDraft::Valid(_));
    policy.revert_requested = ordered_result.revert_requested;
    policy
}

fn resolve_numeric_scrub_config(config: NumericScrubInputConfig) -> ResolvedNumericScrubConfig {
    let base_step = sanitize_numeric_scrub_step(config.base_step, DEFAULT_NUMERIC_SCRUB_STEP);
    let fine_step = sanitize_numeric_scrub_step(
        config.fine_step,
        base_step * DEFAULT_NUMERIC_SCRUB_FINE_FACTOR,
    );
    let coarse_step = sanitize_numeric_scrub_step(
        config.coarse_step,
        base_step * DEFAULT_NUMERIC_SCRUB_COARSE_FACTOR,
    );
    let min = config.min.filter(|value| value.is_finite());
    let max = config.max.filter(|value| value.is_finite());
    let (min, max) = match (min, max) {
        (Some(min), Some(max)) if min > max => (Some(max), Some(min)),
        bounds => bounds,
    };

    ResolvedNumericScrubConfig {
        base_step,
        fine_step,
        coarse_step,
        min,
        max,
    }
}

fn sanitize_numeric_scrub_step(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn numeric_scrub_step_for_modifiers(
    config: &ResolvedNumericScrubConfig,
    modifiers: kinetik_ui_core::Modifiers,
) -> f32 {
    if modifiers.shift {
        config.fine_step
    } else if modifiers.ctrl || modifiers.super_key {
        config.coarse_step
    } else {
        config.base_step
    }
}

fn clamp_numeric_scrub_value(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let mut value = if value.is_finite() { value } else { 0.0 };
    if let Some(min) = min {
        value = value.max(min);
    }
    if let Some(max) = max {
        value = value.min(max);
    }
    value
}

fn numeric_scrub_value_changed(before: f32, after: f32) -> bool {
    !(before.is_nan() && after.is_nan()) && before.to_bits() != after.to_bits()
}

fn format_numeric_scrub_value(value: f32) -> String {
    if value.fract().abs() <= f32::EPSILON {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

fn apply_numeric_scrub_semantics(
    numeric: &mut NumericInputOutput,
    min: Option<f32>,
    max: Option<f32>,
) {
    let Some(node) = numeric.field.widget.semantics.first_mut() else {
        return;
    };
    let Some(current) = numeric.value.filter(|value| value.is_finite()) else {
        return;
    };
    let semantic_min = min.unwrap_or(current);
    let semantic_max = max.unwrap_or(current);
    node.state.value = Some(SemanticValue::Number {
        current,
        min: semantic_min.min(semantic_max),
        max: semantic_min.max(semantic_max),
    });
    if !node
        .actions
        .iter()
        .any(|action| action.kind == SemanticActionKind::SetValue)
    {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::SetValue,
            "Set value",
        ));
    }
}
