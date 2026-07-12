use super::{
    ComponentState, CursorShape, OrderedTextInputResult, Primitive, Rect, RectPrimitive, Response,
    TextEditMode, TextEditState, TextLayoutKey, TextLayoutStore, TextSelection, TextStyle, Theme,
    UiInput, UiMemory, WidgetId, WidgetOutput, display_text_with_composition, focusable,
    multi_line_hit_offset, multi_line_text_primitives, single_line_hit_offset,
    single_line_text_primitives, text_field_layout, text_field_semantics,
    text_input_platform_requests, text_line_fragments, with_hover_cursor, with_response_state,
};
use kinetik_ui_core::{
    CapturedDomainDragGesture, DomainDragGesturePhase, RepaintRequest, TextInputOwnerMode,
    Ui as CoreUi,
};
use kinetik_ui_text::TextViewport;

use super::semantics::text_field_semantics_with_access;
use super::text_geometry::{TextFieldGeometry, TextFieldKind};
use super::text_interaction::{
    ResolvedTextPointerAction, TextNavigationResolution, TextPointerPhase, TextReplayResult,
    replay_text_field_events_with_navigation, text_wheel_delta,
};

/// Access policy for a canonical text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextFieldAccess {
    /// Focusable text that accepts selection, editing, clipboard, and IME input.
    Editable,
    /// Focusable text that permits navigation, selection, and copy without mutation or IME.
    ReadOnly,
    /// Non-interactive text that cannot focus, select, scroll, copy, edit, or own IME.
    Disabled,
}

impl TextFieldAccess {
    const fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    const fn owner_mode(self) -> Option<TextInputOwnerMode> {
        match self {
            Self::Editable => Some(TextInputOwnerMode::Editable),
            Self::ReadOnly => Some(TextInputOwnerMode::ReadOnly),
            Self::Disabled => None,
        }
    }
}

/// Output emitted by editable text widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Whether the text changed this frame.
    pub changed: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_access_runtime(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> (TextFieldOutput, OrderedTextInputResult) {
    let (field, ordered, _) = text_field_with_access_runtime_metadata(
        runtime,
        id,
        rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
    );
    (field, ordered)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_access_runtime_metadata(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> (
    TextFieldOutput,
    OrderedTextInputResult,
    TextFieldPointerMetadata,
) {
    text_field_with_access_runtime_metadata_and_fence(
        runtime,
        id,
        rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_access_runtime_metadata_and_fence(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
    interaction_fenced: bool,
) -> (
    TextFieldOutput,
    OrderedTextInputResult,
    TextFieldPointerMetadata,
) {
    let result = canonical_text_field_runtime(
        runtime,
        id,
        rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
        TextFieldKind::SingleLine,
        TextEditMode::SingleLine,
        interaction_fenced,
        TextFieldPointerSource::Selection,
        |_, _| true,
    );
    let field = TextFieldOutput {
        widget: result.widget,
        changed: result.changed,
    };
    (field, result.ordered, result.pointer)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TextFieldPointerMetadata {
    pub(crate) accepted_double_click: bool,
    pub(crate) domain_drag_modifiers: Option<kinetik_ui_core::Modifiers>,
    pub(crate) transaction_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RawTextPointerAction {
    ordinal: Option<usize>,
    phase: TextPointerPhase,
    position: Option<kinetik_ui_core::Point>,
    click_count: u8,
    modifiers: kinetik_ui_core::Modifiers,
    release_clicked: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum TextFieldPointerSource {
    Selection,
    DomainDrag(CapturedDomainDragGesture),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_pointer_runtime(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
    pointer_source: TextFieldPointerSource,
    replay_guard: impl FnOnce(&TextFieldPointerMetadata, &TextEditState) -> bool,
) -> (
    TextFieldOutput,
    OrderedTextInputResult,
    TextFieldPointerMetadata,
) {
    let result = canonical_text_field_runtime(
        runtime,
        id,
        rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
        TextFieldKind::SingleLine,
        TextEditMode::SingleLine,
        false,
        pointer_source,
        replay_guard,
    );
    (
        TextFieldOutput {
            widget: result.widget,
            changed: result.changed,
        },
        result.ordered,
        result.pointer,
    )
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
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> (TextFieldOutput, OrderedTextInputResult) {
    let response = focusable(id, rect, input, memory, disabled);
    text_field_with_resolved_response_and_ordered_result(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
        caret_visible,
        response,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn text_field_with_resolved_response_and_ordered_result(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    mut text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
    mut response: Response,
) -> (TextFieldOutput, OrderedTextInputResult) {
    let before = state.text.clone();
    if response.clicked {
        memory.focus(id);
        response.state.focused = true;
    }
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn multi_line_text_field_with_access_runtime(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
) -> MultiLineTextFieldOutput {
    let result = canonical_text_field_runtime(
        runtime,
        id,
        rect,
        state,
        theme,
        access,
        text_layouts,
        caret_visible,
        TextFieldKind::WrappedMultiLine,
        TextEditMode::MultiLine,
        false,
        TextFieldPointerSource::Selection,
        |_, _| true,
    );
    MultiLineTextFieldOutput {
        widget: result.widget,
        changed: result.changed,
        visible_lines: text_line_fragments(&state.text).len(),
    }
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

struct CanonicalTextFieldResult {
    widget: WidgetOutput,
    changed: bool,
    ordered: OrderedTextInputResult,
    pointer: TextFieldPointerMetadata,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn canonical_text_field_runtime(
    runtime: &mut CoreUi<'_>,
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    theme: &Theme,
    access: TextFieldAccess,
    mut text_layouts: Option<&mut TextLayoutStore>,
    caret_visible: bool,
    kind: TextFieldKind,
    edit_mode: TextEditMode,
    interaction_fenced: bool,
    pointer_source: TextFieldPointerSource,
    replay_guard: impl FnOnce(&TextFieldPointerMetadata, &TextEditState) -> bool,
) -> CanonicalTextFieldResult {
    if access == TextFieldAccess::ReadOnly && state.composition.is_some() {
        let _ = state.apply_read_only_ordered_input(&[], edit_mode);
    }
    let before = state.text.clone();
    let entry_focused = runtime.memory().is_focused(id);
    let entry_selection_anchor = state.selection.anchor;
    let retained_gesture_anchor = runtime.memory().selection_gesture_anchor(id);
    let retained_offset = runtime.memory().scroll_offset(id);
    let (mut response, raw_pointer_actions, domain_drag_source) = match pointer_source {
        TextFieldPointerSource::Selection => {
            let (gesture, clicked_release_ordinals) = runtime
                .captured_selection_gesture_with_clicked_releases(
                    id,
                    rect,
                    access.is_disabled() || interaction_fenced,
                );
            let mut actions = gesture
                .actions
                .into_iter()
                .map(|action| RawTextPointerAction {
                    ordinal: action.ordinal,
                    phase: TextPointerPhase::from(action.phase),
                    position: action.position,
                    click_count: action.click_count,
                    modifiers: action.modifiers,
                    release_clicked: false,
                })
                .collect::<Vec<_>>();
            attach_clicked_release_provenance(&mut actions, &clicked_release_ordinals);
            (gesture.response, actions, false)
        }
        TextFieldPointerSource::DomainDrag(gesture) => {
            let mut actions = Vec::with_capacity(gesture.actions.len() + 1);
            for action in gesture.actions {
                let phase = match action.phase {
                    DomainDragGesturePhase::Press => TextPointerPhase::OwnershipPress,
                    DomainDragGesturePhase::Move => TextPointerPhase::OwnershipMove,
                    DomainDragGesturePhase::Release => TextPointerPhase::OwnershipRelease,
                    DomainDragGesturePhase::Cancel => TextPointerPhase::OwnershipCancel,
                };
                actions.push(RawTextPointerAction {
                    ordinal: action.ordinal,
                    phase,
                    position: action.position,
                    click_count: action.click_count,
                    modifiers: action.modifiers,
                    release_clicked: action.release_clicked,
                });
                if action.phase == DomainDragGesturePhase::Release && action.release_clicked {
                    actions.push(RawTextPointerAction {
                        ordinal: action.ordinal,
                        phase: TextPointerPhase::PlaceCaret,
                        position: action.position,
                        click_count: action.click_count,
                        modifiers: action.modifiers,
                        release_clicked: true,
                    });
                }
            }
            (gesture.response, actions, true)
        }
    };
    let entry_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: entry_focused,
        disabled: access.is_disabled(),
        selected: false,
    });
    let entry_geometry = TextFieldGeometry::build(
        rect,
        state,
        &entry_recipe,
        kind,
        retained_offset,
        text_layouts.as_deref_mut(),
    );
    let mut pointer_actions = raw_pointer_actions
        .into_iter()
        .map(|action| ResolvedTextPointerAction {
            ordinal: action.ordinal,
            phase: action.phase,
            model_caret: action
                .position
                .map(|position| entry_geometry.model_caret_at(position)),
            click_count: action.click_count,
            modifiers: action.modifiers,
            release_clicked: action.release_clicked,
        })
        .collect::<Vec<_>>();

    let final_root_press = runtime.last_root_primary_press_ordinal();
    let legacy_snapshot_press =
        runtime.input().events.is_empty() && runtime.input().pointer.primary.pressed;
    let root_press_present = final_root_press.is_some() || legacy_snapshot_press;
    let owns_press = if let Some(final_ordinal) = final_root_press {
        final_primary_press_is_unambiguous(&pointer_actions, final_ordinal)
            && pointer_actions
                .iter()
                .filter(|action| {
                    is_pointer_ownership_press(action.phase)
                        && action.ordinal == Some(final_ordinal)
                        && action.model_caret.is_some()
                })
                .count()
                == 1
    } else if legacy_snapshot_press {
        pointer_actions
            .iter()
            .filter(|action| {
                is_pointer_ownership_press(action.phase)
                    && action.ordinal.is_none()
                    && action.model_caret.is_some()
            })
            .count()
            == 1
    } else {
        false
    };

    let domain_drag_aggregate_authoritative = domain_drag_source
        && response.dragged
        && if let Some(final_ordinal) = final_root_press {
            owns_press && discarded_domain_trace_cannot_drag(&pointer_actions, final_ordinal)
        } else if legacy_snapshot_press {
            owns_press
        } else {
            true
        };

    if let Some(final_ordinal) = final_root_press {
        if owns_press {
            pointer_actions.retain(|action| {
                action
                    .ordinal
                    .is_some_and(|ordinal| ordinal >= final_ordinal)
            });
        } else {
            pointer_actions.clear();
        }
    } else if legacy_snapshot_press && !owns_press {
        pointer_actions.clear();
    }

    let accepted_place_caret = pointer_actions
        .iter()
        .any(|action| action.phase == TextPointerPhase::PlaceCaret && action.model_caret.is_some());
    let accepted_double_click = pointer_actions.iter().any(|action| {
        action.release_clicked
            && matches!(
                action.phase,
                TextPointerPhase::Release | TextPointerPhase::OwnershipRelease
            )
            && action.model_caret.is_some()
            && action.click_count >= 2
    });
    let domain_drag_modifiers = domain_drag_aggregate_authoritative.then(|| {
        pointer_actions
            .iter()
            .rev()
            .find(|action| action.phase == TextPointerPhase::OwnershipMove)
            .or_else(|| {
                pointer_actions
                    .iter()
                    .rev()
                    .find(|action| action.phase == TextPointerPhase::OwnershipRelease)
            })
            .map(|action| action.modifiers)
    });
    let mut pointer_metadata = TextFieldPointerMetadata {
        accepted_double_click,
        domain_drag_modifiers: domain_drag_modifiers.flatten(),
        transaction_allowed: true,
    };
    let pointer_activates = if domain_drag_source {
        accepted_place_caret
    } else {
        owns_press
    };

    if access.is_disabled() || interaction_fenced || (root_press_present && !owns_press) {
        if runtime.memory().is_focused(id) {
            runtime.memory_mut().clear_focus();
        }
    } else if pointer_activates {
        runtime.memory_mut().focus(id);
    }

    let entry_accepts_input = entry_focused && (!root_press_present || owns_press);
    let prepared = access.owner_mode().is_some_and(|mode| {
        runtime.memory().is_focused(id) && runtime.prepare_text_input_owner(id, mode)
    });
    let requires_transaction_preview =
        domain_drag_source && response.dragged && pointer_metadata.domain_drag_modifiers.is_some();
    let navigation_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: runtime.memory().is_focused(id) && !access.is_disabled(),
        disabled: access.is_disabled(),
        selected: false,
    });
    let navigation_style = TextStyle::new(
        navigation_recipe.font.family,
        navigation_recipe.font.size,
        navigation_recipe.font.line_height,
    );
    let navigation_width = (rect.width - navigation_recipe.padding_x * 2.0).max(0.0);
    let navigation_wrap = kind.wraps();
    let shaped_navigation_configured = text_layouts.is_some();
    let (replay_enabled, replay) = if access.is_disabled() || interaction_fenced {
        (false, TextReplayResult::default())
    } else if requires_transaction_preview {
        let preview_events = if prepared {
            runtime
                .preview_ordered_text_input_events(id)
                .ok()
                .flatten()
                .map(<[_]>::to_vec)
        } else {
            Some(Vec::new())
        };
        if let Some(preview_events) = preview_events {
            let mut preview_state = state.clone();
            let preview_replay = replay_text_field_events_with_navigation(
                &mut preview_state,
                access,
                edit_mode,
                id,
                entry_accepts_input,
                entry_selection_anchor,
                retained_gesture_anchor,
                pointer_actions.clone(),
                preview_events.clone(),
                shaped_navigation_configured,
                |state| {
                    resolve_text_navigation(
                        text_layouts.as_deref_mut(),
                        state,
                        &navigation_style,
                        navigation_width,
                        navigation_wrap,
                    )
                },
            );
            if replay_guard(&pointer_metadata, &preview_state) {
                let claim_matches = !prepared
                    || runtime
                        .claim_ordered_text_input_events(id)
                        .ok()
                        .flatten()
                        .is_some_and(|claimed| claimed == preview_events);
                if claim_matches {
                    *state = preview_state;
                    (true, preview_replay)
                } else {
                    (false, TextReplayResult::default())
                }
            } else {
                (false, TextReplayResult::default())
            }
        } else {
            (false, TextReplayResult::default())
        }
    } else {
        let replay_enabled = replay_guard(&pointer_metadata, state);
        let ordered_events = if replay_enabled && prepared {
            runtime
                .claim_ordered_text_input_events(id)
                .ok()
                .flatten()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let replay = if replay_enabled {
            replay_text_field_events_with_navigation(
                state,
                access,
                edit_mode,
                id,
                entry_accepts_input,
                entry_selection_anchor,
                retained_gesture_anchor,
                pointer_actions,
                ordered_events,
                shaped_navigation_configured,
                |state| {
                    resolve_text_navigation(
                        text_layouts.as_deref_mut(),
                        state,
                        &navigation_style,
                        navigation_width,
                        navigation_wrap,
                    )
                },
            )
        } else {
            TextReplayResult::default()
        };
        (replay_enabled, replay)
    };
    pointer_metadata.transaction_allowed = replay_enabled;
    if let Some(anchor) = replay.accepted_gesture_anchor {
        let _ = runtime
            .memory_mut()
            .set_selection_gesture_anchor(id, anchor);
    }
    if replay.focus_lost && runtime.memory().is_focused(id) {
        runtime.memory_mut().clear_focus();
    }

    response.state.focused = runtime.memory().is_focused(id) && !access.is_disabled();
    response.state.disabled = access.is_disabled();
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: access.is_disabled(),
        selected: false,
    });
    let geometry =
        TextFieldGeometry::build(rect, state, &recipe, kind, retained_offset, text_layouts);

    if !access.is_disabled() && replay_enabled {
        let wheel = text_wheel_delta(runtime.input(), runtime.memory(), id, rect, kind, false);
        let viewport = geometry.viewport();
        let mut candidate = viewport.scroll_by(wheel);
        let candidate_viewport = TextViewport::new(
            kind.viewport_mode(),
            viewport.viewport_size(),
            viewport.content_size(),
            candidate,
        );
        if response.state.focused {
            candidate = candidate_viewport.reveal(geometry.caret_content_rect());
        }
        if retained_offset != candidate {
            runtime.stage_scroll_offset(id, candidate);
            runtime.request_repaint(RepaintRequest::NextFrame);
        }
    }

    if access == TextFieldAccess::Editable
        && replay_enabled
        && response.state.focused
        && !replay.focus_lost
        && let Some(caret) = geometry.visible_caret_rect()
    {
        let _ = runtime.publish_text_input_rect(id, caret);
    }

    let content_has_area = rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > recipe.padding_x * 2.0
        && rect.height > recipe.padding_y * 2.0;
    let primitives = if content_has_area {
        geometry.primitives(
            id,
            response.state.focused,
            !access.is_disabled(),
            caret_visible,
        )
    } else {
        vec![Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        })]
    };
    let widget = with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(
                text_field_semantics_with_access(
                    id,
                    rect,
                    "Text field",
                    state.text.clone(),
                    access,
                ),
                &response,
            ))
            .with_platform_requests(replay.ordered.platform_requests.iter().cloned()),
        &response,
        CursorShape::Text,
    );

    CanonicalTextFieldResult {
        widget,
        changed: before != state.text,
        ordered: replay.ordered,
        pointer: pointer_metadata,
    }
}

fn resolve_text_navigation(
    text_layouts: Option<&mut TextLayoutStore>,
    state: &TextEditState,
    style: &TextStyle,
    width: f32,
    wrap: bool,
) -> TextNavigationResolution {
    let Some(store) = text_layouts else {
        return TextNavigationResolution::Unavailable;
    };
    let id = store.layout_id(TextLayoutKey::new(
        state.text.clone(),
        style.clone(),
        width,
        wrap,
    ));
    let Some(layout) = store.layout(id) else {
        return TextNavigationResolution::Invalid;
    };
    layout.navigation(&state.text).map_or(
        TextNavigationResolution::Invalid,
        TextNavigationResolution::Ready,
    )
}

fn discarded_domain_trace_cannot_drag(
    actions: &[ResolvedTextPointerAction],
    final_ordinal: usize,
) -> bool {
    let discarded = actions
        .iter()
        .filter(|action| {
            action
                .ordinal
                .is_some_and(|ordinal| ordinal < final_ordinal)
        })
        .collect::<Vec<_>>();
    if discarded.is_empty() {
        return true;
    }

    let mut open = !is_pointer_ownership_press(discarded[0].phase);
    for action in &discarded {
        match action.phase {
            phase if is_pointer_ownership_press(phase) => {
                if open {
                    return false;
                }
                open = true;
            }
            TextPointerPhase::OwnershipMove => open = true,
            TextPointerPhase::OwnershipRelease => {
                if !action.release_clicked {
                    return false;
                }
                open = false;
            }
            TextPointerPhase::OwnershipCancel => return false,
            TextPointerPhase::PlaceCaret
            | TextPointerPhase::Press
            | TextPointerPhase::Move
            | TextPointerPhase::Release
            | TextPointerPhase::Cancel
            | TextPointerPhase::OwnershipPress => {}
        }
    }
    !open
}

fn attach_clicked_release_provenance(
    actions: &mut [RawTextPointerAction],
    clicked_release_ordinals: &[Option<usize>],
) {
    for clicked_ordinal in clicked_release_ordinals {
        let mut matching = actions
            .iter()
            .enumerate()
            .filter(|(_, action)| {
                action.phase == TextPointerPhase::Release && action.ordinal == *clicked_ordinal
            })
            .map(|(index, _)| index);
        let Some(index) = matching.next() else {
            continue;
        };
        if matching.next().is_none() {
            actions[index].release_clicked = true;
        }
    }
}

fn final_primary_press_is_unambiguous(
    actions: &[ResolvedTextPointerAction],
    final_ordinal: usize,
) -> bool {
    let mut primary_open = false;
    for action in actions.iter().filter(|action| {
        action
            .ordinal
            .is_some_and(|ordinal| ordinal < final_ordinal)
    }) {
        match action.phase {
            phase if is_pointer_ownership_press(phase) => primary_open = true,
            phase if is_pointer_ownership_end(phase) => primary_open = false,
            TextPointerPhase::Move
            | TextPointerPhase::OwnershipMove
            | TextPointerPhase::PlaceCaret => {}
            TextPointerPhase::Press
            | TextPointerPhase::Release
            | TextPointerPhase::Cancel
            | TextPointerPhase::OwnershipPress
            | TextPointerPhase::OwnershipRelease
            | TextPointerPhase::OwnershipCancel => unreachable!(),
        }
    }
    !primary_open
}

const fn is_pointer_ownership_press(phase: TextPointerPhase) -> bool {
    matches!(
        phase,
        TextPointerPhase::Press | TextPointerPhase::OwnershipPress
    )
}

const fn is_pointer_ownership_end(phase: TextPointerPhase) -> bool {
    matches!(
        phase,
        TextPointerPhase::Release
            | TextPointerPhase::Cancel
            | TextPointerPhase::OwnershipRelease
            | TextPointerPhase::OwnershipCancel
    )
}
