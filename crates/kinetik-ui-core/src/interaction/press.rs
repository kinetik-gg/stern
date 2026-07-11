use super::drag_select::{SelectionGestureAction, SelectionGesturePhase};
use super::{HitTarget, InteractionState, Response};
use crate::{
    Key, KeyState, MouseButton, Point, Rect, Transform, UiInput, UiInputEvent, UiMemory, Vec2,
    WidgetId,
};

const DRAG_THRESHOLD_SQUARED: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PointerGestureKind {
    Press,
    DomainDrag,
    Selection,
}

pub(super) struct PressResolution {
    pub response: Response,
    pub selection_actions: Vec<SelectionGestureAction>,
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
struct PointerOutcome {
    clicked: bool,
    double_clicked: bool,
    secondary_clicked: bool,
    dragged: bool,
    drag_delta: Vec2,
    suppress_drag_output: bool,
    selection_actions: Vec<SelectionGestureAction>,
}

/// Resolves neutral press/click behavior.
pub fn pressable(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    resolve_pressable_with_hit_target(
        id,
        rect,
        HitTarget::Rect,
        input,
        memory,
        disabled,
        PointerGestureKind::Press,
        None,
        true,
    )
    .response
}

/// Resolves neutral press/click behavior with transformed local-space hit testing.
pub fn pressable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    resolve_pressable_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
        PointerGestureKind::Press,
        None,
        true,
    )
    .response
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_pressable_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
    kind: PointerGestureKind,
    event_ordinals: Option<&[usize]>,
    process_events: bool,
) -> PressResolution {
    let conflicted = memory.pointer_input_conflicted(input);
    let owns_primary = owns_primary_gesture(memory, id);
    let mut outcome = PointerOutcome::default();

    if disabled && owns_primary {
        memory.cancel_pointer_interaction();
    }

    if kind == PointerGestureKind::Selection
        && memory.take_cancelled_pointer_gesture(id)
        && let Some((ordinal, position)) = cancellation_evidence(input, event_ordinals)
    {
        outcome.selection_actions.push(SelectionGestureAction {
            ordinal,
            phase: SelectionGesturePhase::Cancel,
            position,
            delta: Vec2::ZERO,
            click_count: input.pointer.click_count,
        });
    }

    if process_events && !disabled && !memory.pointer_interaction_cancelled() {
        if input.events.is_empty() {
            resolve_legacy_pointer(
                id,
                rect,
                hit_target,
                input,
                memory,
                kind,
                conflicted,
                &mut outcome,
            );
        } else {
            resolve_canonical_pointer(
                id,
                rect,
                hit_target,
                input,
                memory,
                kind,
                event_ordinals,
                conflicted,
                &mut outcome,
            );
        }
    }

    let pointer_cancelled = memory.pointer_interaction_cancelled();
    let hovered = !pointer_cancelled
        && !conflicted
        && !disabled
        && hit_target.routed_hit_test(id, rect, input, memory);
    if hovered {
        memory.set_hovered(id);
    }

    let keyboard_activated =
        process_events && !disabled && keyboard_activation_pressed(id, input, memory);
    outcome.clicked |= keyboard_activated;
    outcome.double_clicked &= outcome.clicked;
    if outcome.suppress_drag_output {
        outcome.dragged = false;
        outcome.drag_delta = Vec2::ZERO;
    }

    let active = memory.is_active(id);
    let pressed = memory.is_pressed(id) && input.pointer.primary.down;
    PressResolution {
        response: Response {
            clicked: outcome.clicked,
            double_clicked: outcome.double_clicked,
            secondary_clicked: outcome.secondary_clicked,
            dragged: outcome.dragged,
            keyboard_activated,
            drag_delta: outcome.drag_delta,
            ..Response::new(
                id,
                rect,
                InteractionState {
                    hovered,
                    focused: memory.is_focused(id),
                    active,
                    pressed,
                    disabled,
                    selected: false,
                },
            )
        },
        selection_actions: outcome.selection_actions,
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn resolve_canonical_pointer(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    kind: PointerGestureKind,
    event_ordinals: Option<&[usize]>,
    conflicted: bool,
    outcome: &mut PointerOutcome,
) {
    if kind == PointerGestureKind::Selection {
        let ordinals = event_ordinals.expect("selection gestures require root event ordinals");
        assert_eq!(
            ordinals.len(),
            input.events.len(),
            "selection gesture ordinal sidecar must match localized input"
        );
    }

    for (event_index, event) in input.events.iter().enumerate() {
        let ordinal = selection_ordinal(kind, event_ordinals, event_index);
        match event {
            UiInputEvent::PointerMoved { position, delta } => {
                if !conflicted && owns_primary_gesture(memory, id) {
                    push_selection_action(
                        kind,
                        outcome,
                        ordinal,
                        SelectionGesturePhase::Move,
                        Some(*position),
                        *delta,
                        input.pointer.click_count,
                    );
                    resolve_motion(id, *position, *delta, memory, kind, outcome);
                }
            }
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: true,
                click_count,
                position,
            } => {
                if !conflicted
                    && !owns_primary_gesture(memory, id)
                    && memory.pointer_route_allows(id)
                    && hit_target.hit_test_position(rect, position.or(input.pointer.position))
                    && let Some(press_position) = position.or(input.pointer.position)
                {
                    memory.activate(id);
                    memory.press(id);
                    memory.capture_pointer(id);
                    memory.begin_pointer_gesture(id, press_position);
                    push_selection_action(
                        kind,
                        outcome,
                        ordinal,
                        SelectionGesturePhase::Press,
                        Some(press_position),
                        Vec2::ZERO,
                        *click_count,
                    );
                }
            }
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                click_count,
                position,
            } => {
                if owns_primary_gesture(memory, id) {
                    resolve_primary_release(
                        id,
                        rect,
                        hit_target,
                        position.or(input.pointer.position),
                        *click_count,
                        memory,
                        kind,
                        ordinal,
                        conflicted,
                        outcome,
                    );
                }
            }
            UiInputEvent::PointerButton {
                button: MouseButton::Secondary,
                down,
                position,
                ..
            } => resolve_secondary_transition(
                id,
                rect,
                hit_target,
                position.or(input.pointer.position),
                *down,
                memory,
                conflicted,
                outcome,
            ),
            UiInputEvent::PointerReleaseAll { position } => {
                if owns_primary_gesture(memory, id) {
                    push_selection_action(
                        kind,
                        outcome,
                        ordinal,
                        SelectionGesturePhase::Cancel,
                        position.or(input.pointer.position),
                        Vec2::ZERO,
                        input.pointer.click_count,
                    );
                    memory.cancel_pointer_interaction();
                    outcome.suppress_drag_output = true;
                } else if memory.is_secondary_pressed(id) {
                    memory.release_secondary(id);
                }
            }
            UiInputEvent::WindowFocusChanged(false) => {
                if owns_primary_gesture(memory, id) {
                    push_selection_action(
                        kind,
                        outcome,
                        ordinal,
                        SelectionGesturePhase::Cancel,
                        input.pointer.position,
                        Vec2::ZERO,
                        input.pointer.click_count,
                    );
                    memory.cancel_pointer_interaction();
                    outcome.suppress_drag_output = true;
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_legacy_pointer(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    kind: PointerGestureKind,
    conflicted: bool,
    outcome: &mut PointerOutcome,
) {
    if conflicted {
        return;
    }

    let hovered = hit_target.routed_hit_test(id, rect, input, memory);
    if hovered && input.pointer.primary.pressed && !owns_primary_gesture(memory, id) {
        let Some(position) = input.pointer.position else {
            return;
        };
        let origin = Point::new(
            position.x - input.pointer.delta.x,
            position.y - input.pointer.delta.y,
        );
        memory.activate(id);
        memory.press(id);
        memory.capture_pointer(id);
        memory.begin_pointer_gesture(id, origin);
        push_selection_action(
            kind,
            outcome,
            None,
            SelectionGesturePhase::Press,
            Some(origin),
            Vec2::ZERO,
            input.pointer.click_count,
        );
    }

    if owns_primary_gesture(memory, id)
        && input.pointer.delta != Vec2::ZERO
        && let Some(position) = input.pointer.position
    {
        push_selection_action(
            kind,
            outcome,
            None,
            SelectionGesturePhase::Move,
            Some(position),
            input.pointer.delta,
            input.pointer.click_count,
        );
        resolve_motion(id, position, input.pointer.delta, memory, kind, outcome);
    }

    if input.pointer.primary.released && owns_primary_gesture(memory, id) {
        resolve_primary_release(
            id,
            rect,
            hit_target,
            input.pointer.position,
            input.pointer.click_count,
            memory,
            kind,
            None,
            false,
            outcome,
        );
    }

    if input.pointer.secondary.pressed && hovered {
        memory.press_secondary(id);
    }
    if input.pointer.secondary.released && memory.is_secondary_pressed(id) {
        outcome.secondary_clicked = hovered;
        memory.release_secondary(id);
    }
}

fn resolve_motion(
    id: WidgetId,
    position: Point,
    delta: Vec2,
    memory: &mut UiMemory,
    kind: PointerGestureKind,
    outcome: &mut PointerOutcome,
) {
    let Some((origin, was_crossed)) = memory.pointer_gesture(id) else {
        return;
    };
    let displacement = Vec2::new(position.x - origin.x, position.y - origin.y);
    let displacement_squared = displacement
        .x
        .mul_add(displacement.x, displacement.y * displacement.y);
    let crosses_now = !was_crossed
        && displacement_squared.is_finite()
        && displacement_squared >= DRAG_THRESHOLD_SQUARED;

    if crosses_now {
        memory.mark_pointer_threshold_crossed(id);
        if kind == PointerGestureKind::DomainDrag {
            memory.start_drag(id);
            outcome.dragged = true;
            outcome.drag_delta = displacement;
        }
        return;
    }

    if was_crossed && kind == PointerGestureKind::DomainDrag {
        memory.start_drag(id);
        if delta != Vec2::ZERO {
            outcome.dragged = true;
            outcome.drag_delta = add_vectors(outcome.drag_delta, delta);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_primary_release(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    position: Option<Point>,
    click_count: u8,
    memory: &mut UiMemory,
    kind: PointerGestureKind,
    ordinal: Option<usize>,
    conflicted: bool,
    outcome: &mut PointerOutcome,
) {
    if !conflicted
        && memory
            .pointer_gesture(id)
            .is_some_and(|(_, crossed)| !crossed)
        && let Some(position) = position
    {
        resolve_motion(id, position, Vec2::ZERO, memory, kind, outcome);
    }
    let threshold_crossed = memory
        .pointer_gesture(id)
        .is_some_and(|(_, crossed)| crossed)
        || memory.is_drag_source(id);
    let released_inside = hit_target.hit_test_position(rect, position);
    push_selection_action(
        kind,
        outcome,
        ordinal,
        SelectionGesturePhase::Release,
        position,
        Vec2::ZERO,
        click_count,
    );

    if !conflicted
        && !threshold_crossed
        && released_inside
        && memory.is_active(id)
        && memory.is_pressed(id)
    {
        outcome.clicked = true;
        outcome.double_clicked |= click_count >= 2;
    }
    if conflicted {
        memory.clear_drag();
    } else {
        memory.finish_drag(id);
    }
    if !released_inside {
        outcome.suppress_drag_output = true;
    }
    memory.clear_interaction();
}

#[allow(clippy::too_many_arguments)]
fn resolve_secondary_transition(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    position: Option<Point>,
    down: bool,
    memory: &mut UiMemory,
    conflicted: bool,
    outcome: &mut PointerOutcome,
) {
    if down {
        if !conflicted
            && memory.pointer_route_allows(id)
            && hit_target.hit_test_position(rect, position)
        {
            memory.press_secondary(id);
        }
    } else if memory.is_secondary_pressed(id) {
        outcome.secondary_clicked = !conflicted && hit_target.hit_test_position(rect, position);
        memory.release_secondary(id);
    }
}

fn selection_ordinal(
    kind: PointerGestureKind,
    event_ordinals: Option<&[usize]>,
    event_index: usize,
) -> Option<usize> {
    (kind == PointerGestureKind::Selection)
        .then(|| event_ordinals.expect("selection ordinal sidecar")[event_index])
}

#[allow(clippy::too_many_arguments)]
fn push_selection_action(
    kind: PointerGestureKind,
    outcome: &mut PointerOutcome,
    ordinal: Option<usize>,
    phase: SelectionGesturePhase,
    position: Option<Point>,
    delta: Vec2,
    click_count: u8,
) {
    if kind == PointerGestureKind::Selection {
        outcome.selection_actions.push(SelectionGestureAction {
            ordinal,
            phase,
            position,
            delta,
            click_count,
        });
    }
}

fn cancellation_evidence(
    input: &UiInput,
    event_ordinals: Option<&[usize]>,
) -> Option<(Option<usize>, Option<Point>)> {
    if input.events.is_empty() {
        return Some((None, input.pointer.position));
    }
    let ordinals = event_ordinals?;
    input
        .events
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, event)| match event {
            UiInputEvent::PointerReleaseAll { position } => {
                Some((Some(ordinals[index]), position.or(input.pointer.position)))
            }
            UiInputEvent::WindowFocusChanged(false) => {
                Some((Some(ordinals[index]), input.pointer.position))
            }
            _ => None,
        })
}

fn owns_primary_gesture(memory: &UiMemory, id: WidgetId) -> bool {
    memory.pointer_gesture_owner() == Some(id)
        || memory.pointer_capture() == Some(id)
        || memory.is_active(id)
}

fn add_vectors(left: Vec2, right: Vec2) -> Vec2 {
    Vec2::new(left.x + right.x, left.y + right.y)
}

fn keyboard_activation_pressed(id: WidgetId, input: &UiInput, memory: &UiMemory) -> bool {
    memory.is_focused(id)
        && input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && !event.repeat
                && event.modifiers.is_empty()
                && matches!(event.key, Key::Enter | Key::Space)
                && !(memory.owns_text_input(id) && matches!(event.key, Key::Space))
        })
}
