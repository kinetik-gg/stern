use core::cmp::Ordering;

use kinetik_ui_core::{
    InputWheelDelta, Key, KeyState, OrderedTextInputEvent, PointerRoute, Rect,
    SelectionGesturePhase, UiInput, UiInputEvent, UiMemory, Vec2, WidgetId,
};
use kinetik_ui_text::{
    OrderedTextInputResult, ShapedTextNavigation, TextCaret, TextEditMode, TextEditState,
    TextSelection,
};

use super::text_fields::TextFieldAccess;
use super::text_geometry::TextFieldKind;

const WHEEL_LINE_STEP: Vec2 = Vec2::new(40.0, 40.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextPointerPhase {
    Press,
    Move,
    Release,
    Cancel,
    OwnershipPress,
    OwnershipMove,
    OwnershipRelease,
    OwnershipCancel,
    PlaceCaret,
}

impl From<SelectionGesturePhase> for TextPointerPhase {
    fn from(value: SelectionGesturePhase) -> Self {
        match value {
            SelectionGesturePhase::Press => Self::Press,
            SelectionGesturePhase::Move => Self::Move,
            SelectionGesturePhase::Release => Self::Release,
            SelectionGesturePhase::Cancel => Self::Cancel,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedTextPointerAction {
    pub(crate) ordinal: Option<usize>,
    pub(crate) phase: TextPointerPhase,
    pub(crate) model_caret: Option<TextCaret>,
    pub(crate) click_count: u8,
    pub(crate) modifiers: kinetik_ui_core::Modifiers,
    pub(crate) release_clicked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TextNavigationResolution {
    Unavailable,
    Ready(ShapedTextNavigation),
    Invalid,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TextReplayResult {
    pub(crate) ordered: OrderedTextInputResult,
    pub(crate) accepted_press: bool,
    pub(crate) accepted_gesture_anchor: Option<usize>,
    pub(crate) focus_lost: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum ReplayItem {
    Pointer(ResolvedTextPointerAction),
    Text(OrderedTextInputEvent),
}

impl ReplayItem {
    const fn ordinal(&self) -> Option<usize> {
        match self {
            Self::Pointer(action) => action.ordinal,
            Self::Text(event) => event.ordinal,
        }
    }

    const fn pointer_precedence(&self) -> u8 {
        match self {
            Self::Pointer(_) => 0,
            Self::Text(_) => 1,
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn replay_text_field_events(
    state: &mut TextEditState,
    access: TextFieldAccess,
    mode: TextEditMode,
    target: WidgetId,
    entry_focused: bool,
    entry_selection_anchor: usize,
    retained_gesture_anchor: Option<usize>,
    pointer_actions: Vec<ResolvedTextPointerAction>,
    text_events: Vec<OrderedTextInputEvent>,
) -> TextReplayResult {
    replay_text_field_events_with_navigation(
        state,
        access,
        mode,
        target,
        entry_focused,
        entry_selection_anchor,
        retained_gesture_anchor,
        pointer_actions,
        text_events,
        false,
        |_| TextNavigationResolution::Unavailable,
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub(crate) fn replay_text_field_events_with_navigation(
    state: &mut TextEditState,
    access: TextFieldAccess,
    mode: TextEditMode,
    target: WidgetId,
    entry_focused: bool,
    entry_selection_anchor: usize,
    retained_gesture_anchor: Option<usize>,
    pointer_actions: Vec<ResolvedTextPointerAction>,
    text_events: Vec<OrderedTextInputEvent>,
    shaped_navigation_configured: bool,
    mut resolve_navigation: impl FnMut(&TextEditState) -> TextNavigationResolution,
) -> TextReplayResult {
    let mut items = pointer_actions
        .into_iter()
        .map(ReplayItem::Pointer)
        .chain(text_events.into_iter().map(ReplayItem::Text))
        .collect::<Vec<_>>();
    items.sort_by(compare_replay_items);

    let mut result = TextReplayResult::default();
    let mut active = entry_focused;
    let mut focus_fenced = false;
    let mut gesture_anchor = retained_gesture_anchor.unwrap_or(entry_selection_anchor);
    if active && access == TextFieldAccess::ReadOnly {
        let _ = state.apply_read_only_ordered_input(&[], mode);
    }

    for item in items {
        if focus_fenced {
            continue;
        }
        match item {
            ReplayItem::Pointer(action) => match action.phase {
                TextPointerPhase::Press | TextPointerPhase::PlaceCaret => {
                    let Some(caret) = action.model_caret else {
                        continue;
                    };
                    if action.click_count >= 2 {
                        state.select_word_at(caret.offset);
                        gesture_anchor = state.selection.anchor;
                    } else if action.modifiers.shift {
                        state.set_selection_with_affinity(
                            TextSelection::new(entry_selection_anchor, caret.offset),
                            caret.affinity,
                        );
                        gesture_anchor = entry_selection_anchor;
                    } else {
                        state.set_caret_position(caret);
                        gesture_anchor = caret.offset;
                    }
                    active = true;
                    result.accepted_press |= action.phase == TextPointerPhase::Press;
                    result.accepted_gesture_anchor = Some(gesture_anchor);
                    if access == TextFieldAccess::ReadOnly {
                        let _ = state.apply_read_only_ordered_input(&[], mode);
                    }
                }
                TextPointerPhase::Move if active => {
                    if let Some(caret) = action.model_caret {
                        state.set_selection_with_affinity(
                            TextSelection::new(gesture_anchor, caret.offset),
                            caret.affinity,
                        );
                    }
                }
                TextPointerPhase::Move
                | TextPointerPhase::Release
                | TextPointerPhase::Cancel
                | TextPointerPhase::OwnershipPress
                | TextPointerPhase::OwnershipMove
                | TextPointerPhase::OwnershipRelease
                | TextPointerPhase::OwnershipCancel => {}
            },
            ReplayItem::Text(event) => {
                let loses_focus = matches!(event.event, UiInputEvent::WindowFocusChanged(false));
                if active {
                    if let UiInputEvent::Key(key) = &event.event
                        && access != TextFieldAccess::Disabled
                        && is_pressed_horizontal_key(key)
                    {
                        if shaped_navigation_configured && state.composition.is_some() {
                            continue;
                        }
                        match resolve_navigation(state) {
                            TextNavigationResolution::Unavailable
                                if shaped_navigation_configured =>
                            {
                                continue;
                            }
                            TextNavigationResolution::Unavailable => {}
                            TextNavigationResolution::Ready(navigation) => {
                                let outcome = state.apply_visual_navigation_key(key, &navigation);
                                debug_assert!(outcome.is_some());
                                continue;
                            }
                            TextNavigationResolution::Invalid => continue,
                        }
                    }
                    match access {
                        TextFieldAccess::Editable => {
                            merge_ordered_result(
                                &mut result.ordered,
                                state.apply_ordered_input_with_result(&[event.event], target, mode),
                            );
                        }
                        TextFieldAccess::ReadOnly => {
                            result
                                .ordered
                                .platform_requests
                                .extend(state.apply_read_only_ordered_input(&[event.event], mode));
                        }
                        TextFieldAccess::Disabled => {}
                    }
                }
                if loses_focus {
                    result.focus_lost = true;
                    active = false;
                    focus_fenced = true;
                }
            }
        }
    }

    result
}

fn is_pressed_horizontal_key(event: &kinetik_ui_core::KeyEvent) -> bool {
    event.state == KeyState::Pressed && matches!(event.key, Key::ArrowLeft | Key::ArrowRight)
}

fn compare_replay_items(left: &ReplayItem, right: &ReplayItem) -> Ordering {
    match (left.ordinal(), right.ordinal()) {
        (Some(left_ordinal), Some(right_ordinal)) => left_ordinal
            .cmp(&right_ordinal)
            .then_with(|| left.pointer_precedence().cmp(&right.pointer_precedence())),
        (None, None) => left.pointer_precedence().cmp(&right.pointer_precedence()),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
    }
}

fn merge_ordered_result(target: &mut OrderedTextInputResult, next: OrderedTextInputResult) {
    target.platform_requests.extend(next.platform_requests);
    target.commit_requested |= next.commit_requested;
    target.revert_requested |= next.revert_requested;
}

pub(crate) fn text_wheel_delta(
    input: &UiInput,
    memory: &UiMemory,
    id: WidgetId,
    rect: Rect,
    kind: TextFieldKind,
    disabled: bool,
) -> Vec2 {
    if disabled
        || !input
            .pointer
            .position
            .is_some_and(|position| rect.contains_point(position))
    {
        return Vec2::ZERO;
    }
    let route_allows = match memory.pointer_wheel_route() {
        PointerRoute::Unplanned => true,
        PointerRoute::Blocked => false,
        PointerRoute::Target(owner) => owner == id,
    };
    if !route_allows || (input.events.is_empty() && memory.pointer_interaction_cancelled()) {
        return Vec2::ZERO;
    }

    let wheel = normalized_wheel_delta(input);
    match kind {
        TextFieldKind::SingleLine => Vec2::new(-wheel.x, 0.0),
        TextFieldKind::WrappedMultiLine => Vec2::new(0.0, -wheel.y),
    }
}

fn normalized_wheel_delta(input: &UiInput) -> Vec2 {
    if input.events.is_empty() {
        return sanitize_vector(input.pointer.wheel_delta);
    }

    let mut accumulated = Vec2::ZERO;
    for event in &input.events {
        match event {
            UiInputEvent::Wheel { delta, .. } => {
                let delta = match *delta {
                    InputWheelDelta::Lines(delta) => Vec2::new(
                        sanitize_component(delta.x * WHEEL_LINE_STEP.x),
                        sanitize_component(delta.y * WHEEL_LINE_STEP.y),
                    ),
                    InputWheelDelta::Pixels(delta) => sanitize_vector(delta),
                };
                accumulated = Vec2::new(
                    sanitize_component(accumulated.x + delta.x),
                    sanitize_component(accumulated.y + delta.y),
                );
            }
            UiInputEvent::PointerReleaseAll { .. } | UiInputEvent::WindowFocusChanged(false) => {
                break;
            }
            _ => {}
        }
    }
    accumulated
}

fn sanitize_vector(vector: Vec2) -> Vec2 {
    Vec2::new(sanitize_component(vector.x), sanitize_component(vector.y))
}

fn sanitize_component(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
