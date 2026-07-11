use super::hit::HitTarget;
use super::press::resolve_pressable_with_hit_target;
use super::{Response, pressable, pressable_transformed};
use crate::memory::PointerGestureKind;
use crate::{Modifiers, Point, Rect, Transform, UiInput, UiInputEvent, UiMemory, Vec2, WidgetId};

/// One claimed editing-domain event paired with its original canonical ordinal.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedTextInputEvent {
    /// Original root event index, or `None` for legacy synthesized input.
    pub ordinal: Option<usize>,
    /// Key, text, clipboard, modifier, IME, or focus event.
    pub event: UiInputEvent,
}

/// Ordered phase emitted by a captured text-selection gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionGesturePhase {
    /// Primary pointer capture started inside the target.
    Press,
    /// Captured pointer movement, including movement below the drag threshold.
    Move,
    /// Primary pointer capture ended normally.
    Release,
    /// Capture ended because input or ownership was cancelled.
    Cancel,
}

/// One ordered action from a captured text-selection gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionGestureAction {
    /// Original index in the root canonical event stream, or `None` for legacy input.
    pub ordinal: Option<usize>,
    /// Gesture transition phase.
    pub phase: SelectionGesturePhase,
    /// Event-time position in the current spatial scope, when available.
    pub position: Option<Point>,
    /// Event-time movement in the current spatial scope.
    pub delta: Vec2,
    /// Click sequence count carried by the transition.
    pub click_count: u8,
    /// Modifier state effective at the original root event ordinal.
    pub modifiers: Modifiers,
}

/// Common response plus ordered actions for neutral captured text selection.
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedSelectionGesture {
    /// Common interaction response.
    pub response: Response,
    /// Ordered press, move, release, and cancellation actions.
    pub actions: Vec<SelectionGestureAction>,
}

/// Resolves neutral draggable behavior.
pub fn draggable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    draggable_with_hit_target(id, rect, HitTarget::Rect, input, memory, disabled)
}

/// Resolves neutral draggable behavior with transformed local-space hit testing.
pub fn draggable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    draggable_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
    )
}

fn draggable_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    resolve_pressable_with_hit_target(
        id,
        rect,
        hit_target,
        input,
        memory,
        disabled,
        PointerGestureKind::DomainDrag,
        None,
        true,
    )
    .response
}

pub(crate) fn captured_selection_gesture_with_ordinals(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    event_ordinals: &[usize],
    memory: &mut UiMemory,
    disabled: bool,
) -> CapturedSelectionGesture {
    let process_events = memory.claim_selection_gesture(id);
    let resolution = resolve_pressable_with_hit_target(
        id,
        rect,
        HitTarget::Rect,
        input,
        memory,
        disabled,
        PointerGestureKind::Selection,
        Some(event_ordinals),
        process_events,
    );
    let mut actions = resolution.selection_actions;
    if input.events.is_empty() {
        for action in &mut actions {
            action.modifiers = input.keyboard.modifiers;
        }
    }
    CapturedSelectionGesture {
        response: resolution.response,
        actions,
    }
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

/// Resolves neutral selectable behavior with transformed local-space hit testing.
pub fn selectable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    selected: bool,
    disabled: bool,
) -> Response {
    let mut response = pressable_transformed(id, rect, local_to_screen, input, memory, disabled);
    response.state.selected = selected;
    response
}
