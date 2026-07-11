use super::drag_select::{
    DomainDragGestureAction, DomainDragGesturePhase, SelectionGestureAction, SelectionGesturePhase,
};
use super::{
    HitTarget, InteractionState, Response, canonical_pointer_fenced, crosses_drag_threshold,
};
use crate::memory::PointerGestureKind;
use crate::{
    Key, KeyState, Modifiers, MouseButton, Point, Rect, Transform, UiInput, UiInputEvent, UiMemory,
    Vec2, WidgetId,
};

pub(super) struct PressResolution {
    pub response: Response,
    pub selection_actions: Vec<SelectionGestureAction>,
    pub selection_clicked_release_ordinals: Vec<Option<usize>>,
    pub domain_drag_actions: Vec<DomainDragGestureAction>,
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
    selection_clicked_release_ordinals: Vec<Option<usize>>,
    capture_selection_clicked_releases: bool,
    domain_drag_actions: Vec<DomainDragGestureAction>,
    capture_domain_drag_actions: bool,
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
        false,
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
        false,
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
    capture_selection_clicked_releases: bool,
) -> PressResolution {
    let conflicted = memory.pointer_input_conflicted(input);
    let owns_primary = owns_primary_gesture(memory, id);
    let owns_secondary = memory.is_secondary_pressed(id);
    let retained_kind = memory.pointer_gesture_kind(id);
    let mode_mismatch = retained_kind.is_some_and(|retained| {
        retained != kind
            && (retained == PointerGestureKind::Selection || kind == PointerGestureKind::Selection)
    });
    let mut outcome = PointerOutcome {
        capture_domain_drag_actions: kind == PointerGestureKind::DomainDrag
            && event_ordinals.is_some(),
        capture_selection_clicked_releases,
        ..PointerOutcome::default()
    };

    if (disabled && (owns_primary || owns_secondary)) || mode_mismatch {
        memory.cancel_pointer_interaction();
    }

    if process_events || kind != PointerGestureKind::Selection {
        recover_cancelled_gesture_action(
            id,
            input,
            memory,
            kind,
            event_ordinals,
            mode_mismatch && kind == PointerGestureKind::Selection,
            &mut outcome,
        );
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

    let pointer_cancelled =
        memory.pointer_interaction_cancelled() || canonical_pointer_fenced(input);
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
        selection_clicked_release_ordinals: outcome.selection_clicked_release_ordinals,
        domain_drag_actions: outcome.domain_drag_actions,
    }
}

#[allow(clippy::too_many_arguments)]
fn recover_cancelled_gesture_action(
    id: WidgetId,
    input: &UiInput,
    memory: &mut UiMemory,
    kind: PointerGestureKind,
    event_ordinals: Option<&[usize]>,
    allow_kind_mismatch: bool,
    outcome: &mut PointerOutcome,
) {
    if !matches!(
        kind,
        PointerGestureKind::Selection | PointerGestureKind::DomainDrag
    ) {
        return;
    }
    let Some(cancelled_click_count) =
        memory.take_cancelled_pointer_gesture(id, kind, allow_kind_mismatch)
    else {
        return;
    };
    let Some((ordinal, position, event_click_count)) = cancellation_evidence(input, event_ordinals)
    else {
        return;
    };
    push_gesture_action(
        kind,
        outcome,
        ordinal,
        SelectionGesturePhase::Cancel,
        position,
        Vec2::ZERO,
        event_click_count.unwrap_or(cancelled_click_count),
        false,
    );
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
    if let Some(ordinals) = event_ordinals {
        assert_eq!(
            ordinals.len(),
            input.events.len(),
            "captured gesture ordinal sidecar must match localized input"
        );
    }

    let mut primary_transaction_open = memory.has_primary_pointer_transaction();
    for (event_index, event) in input.events.iter().enumerate() {
        let release_authority_ordinal = memory.scoped_pointer_event_ordinal(event_index);
        let action_ordinal = event_ordinals.map(|ordinals| ordinals[event_index]);
        let cleanup_only = memory.scoped_pointer_event_is_cleanup(event_index);
        match event {
            UiInputEvent::PointerMoved { position, delta } => {
                if !conflicted && owns_primary_gesture(memory, id) {
                    let click_count = memory.pointer_gesture_click_count(id).unwrap_or(0);
                    push_gesture_action(
                        kind,
                        outcome,
                        action_ordinal,
                        SelectionGesturePhase::Move,
                        Some(*position),
                        *delta,
                        click_count,
                        false,
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
                let transaction_was_open = memory
                    .scoped_primary_transaction_was_open(event_index)
                    .unwrap_or(primary_transaction_open);
                if !transaction_was_open
                    && !conflicted
                    && !owns_primary_gesture(memory, id)
                    && memory.canonical_primary_route_allows(id, release_authority_ordinal)
                    && hit_target.hit_test_position(rect, *position)
                    && let Some(press_position) = *position
                {
                    memory.activate(id);
                    memory.press(id);
                    memory.capture_pointer(id);
                    memory.begin_pointer_gesture(id, press_position, kind, *click_count);
                    push_gesture_action(
                        kind,
                        outcome,
                        action_ordinal,
                        SelectionGesturePhase::Press,
                        Some(press_position),
                        Vec2::ZERO,
                        *click_count,
                        false,
                    );
                }
                primary_transaction_open = true;
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
                        *position,
                        *click_count,
                        memory,
                        kind,
                        Some(release_authority_ordinal),
                        action_ordinal,
                        cleanup_only,
                        conflicted,
                        outcome,
                    );
                }
                primary_transaction_open = false;
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
                *position,
                *down,
                memory,
                conflicted,
                cleanup_only,
                outcome,
            ),
            UiInputEvent::PointerReleaseAll { position } => {
                let click_count = memory.pointer_gesture_click_count(id).unwrap_or(0);
                let owns_primary = owns_primary_gesture(memory, id);
                let owns_secondary = memory.is_secondary_pressed(id);
                if owns_primary {
                    push_gesture_action(
                        kind,
                        outcome,
                        action_ordinal,
                        SelectionGesturePhase::Cancel,
                        *position,
                        Vec2::ZERO,
                        click_count,
                        false,
                    );
                }
                resolve_pointer_fence(memory, id, owns_primary, owns_secondary);
                if owns_primary
                    && matches!(
                        kind,
                        PointerGestureKind::Selection | PointerGestureKind::DomainDrag
                    )
                {
                    let _ = memory.take_cancelled_pointer_gesture(id, kind, false);
                }
                break;
            }
            UiInputEvent::WindowFocusChanged(false) => {
                let click_count = memory.pointer_gesture_click_count(id).unwrap_or(0);
                let owns_primary = owns_primary_gesture(memory, id);
                let owns_secondary = memory.is_secondary_pressed(id);
                if owns_primary {
                    push_gesture_action(
                        kind,
                        outcome,
                        action_ordinal,
                        SelectionGesturePhase::Cancel,
                        None,
                        Vec2::ZERO,
                        click_count,
                        false,
                    );
                }
                resolve_pointer_fence(memory, id, owns_primary, owns_secondary);
                if owns_primary
                    && matches!(
                        kind,
                        PointerGestureKind::Selection | PointerGestureKind::DomainDrag
                    )
                {
                    let _ = memory.take_cancelled_pointer_gesture(id, kind, false);
                }
                break;
            }
            _ => {}
        }
    }
}

fn resolve_pointer_fence(
    memory: &mut UiMemory,
    id: WidgetId,
    owns_primary: bool,
    owns_secondary: bool,
) {
    if owns_primary {
        memory.cancel_primary_pointer_interaction();
    }
    if owns_secondary {
        memory.cancel_secondary_pointer_interaction(id);
    }
    if !memory.has_pointer_transaction() {
        memory.fence_pointer_stream();
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
    let owned_at_entry = owns_primary_gesture(memory, id);
    if hovered && input.pointer.primary.pressed && !owned_at_entry {
        let Some(position) = input.pointer.position else {
            return;
        };
        memory.activate(id);
        memory.press(id);
        memory.capture_pointer(id);
        memory.begin_pointer_gesture(id, position, kind, input.pointer.click_count);
        push_gesture_action(
            kind,
            outcome,
            None,
            SelectionGesturePhase::Press,
            Some(position),
            Vec2::ZERO,
            input.pointer.click_count,
            false,
        );
    }

    if owned_at_entry
        && input.pointer.delta != Vec2::ZERO
        && let Some(position) = input.pointer.position
    {
        push_gesture_action(
            kind,
            outcome,
            None,
            SelectionGesturePhase::Move,
            Some(position),
            input.pointer.delta,
            input.pointer.click_count,
            false,
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
            None,
            false,
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
    let crosses_now = !was_crossed && crosses_drag_threshold(origin, position);

    if crosses_now {
        memory.mark_pointer_threshold_crossed(id);
        if kind == PointerGestureKind::DomainDrag
            && memory.pointer_gesture_kind(id) == Some(PointerGestureKind::DomainDrag)
        {
            memory.start_drag(id);
            outcome.dragged = true;
            outcome.drag_delta = displacement;
        }
        return;
    }

    if was_crossed
        && kind == PointerGestureKind::DomainDrag
        && memory.pointer_gesture_kind(id) == Some(PointerGestureKind::DomainDrag)
    {
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
    release_authority_ordinal: Option<usize>,
    action_ordinal: Option<usize>,
    cleanup_only: bool,
    conflicted: bool,
    outcome: &mut PointerOutcome,
) {
    if !conflicted
        && !cleanup_only
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
    let released_inside = !cleanup_only && hit_target.hit_test_position(rect, position);
    let release_clicked = !conflicted
        && !threshold_crossed
        && released_inside
        && memory.is_active(id)
        && memory.is_pressed(id);
    if release_clicked {
        outcome.clicked = true;
        outcome.double_clicked |= click_count >= 2;
    }
    push_gesture_action(
        kind,
        outcome,
        action_ordinal,
        if cleanup_only || conflicted {
            SelectionGesturePhase::Cancel
        } else {
            SelectionGesturePhase::Release
        },
        position,
        Vec2::ZERO,
        click_count,
        release_clicked,
    );
    let exact_domain_gesture = kind == PointerGestureKind::DomainDrag
        && memory.pointer_gesture_kind(id) == Some(PointerGestureKind::DomainDrag);
    if cleanup_only || conflicted || !exact_domain_gesture {
        memory.clear_active_drag();
    } else {
        memory.finish_drag_at(id, release_authority_ordinal);
    }
    if cleanup_only || !released_inside {
        outcome.suppress_drag_output = true;
    }
    if conflicted {
        memory.discard_primary_interaction();
    } else if let Some(release_ordinal) = release_authority_ordinal {
        memory.clear_primary_interaction_at(release_ordinal);
    } else {
        memory.clear_primary_interaction();
    }
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
    cleanup_only: bool,
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
        outcome.secondary_clicked =
            !conflicted && !cleanup_only && hit_target.hit_test_position(rect, position);
        memory.release_secondary(id);
    }
}

#[allow(clippy::too_many_arguments)]
fn push_gesture_action(
    kind: PointerGestureKind,
    outcome: &mut PointerOutcome,
    ordinal: Option<usize>,
    phase: SelectionGesturePhase,
    position: Option<Point>,
    delta: Vec2,
    click_count: u8,
    release_clicked: bool,
) {
    match kind {
        PointerGestureKind::Selection => {
            if release_clicked && outcome.capture_selection_clicked_releases {
                outcome.selection_clicked_release_ordinals.push(ordinal);
            }
            outcome.selection_actions.push(SelectionGestureAction {
                ordinal,
                phase,
                position,
                delta,
                click_count,
                modifiers: Modifiers::default(),
            });
        }
        PointerGestureKind::DomainDrag if outcome.capture_domain_drag_actions => {
            outcome.domain_drag_actions.push(DomainDragGestureAction {
                ordinal,
                phase: match phase {
                    SelectionGesturePhase::Press => DomainDragGesturePhase::Press,
                    SelectionGesturePhase::Move => DomainDragGesturePhase::Move,
                    SelectionGesturePhase::Release => DomainDragGesturePhase::Release,
                    SelectionGesturePhase::Cancel => DomainDragGesturePhase::Cancel,
                },
                position,
                delta,
                click_count,
                modifiers: Modifiers::default(),
                release_clicked,
            });
        }
        PointerGestureKind::Press | PointerGestureKind::DomainDrag => {}
    }
}

fn cancellation_evidence(
    input: &UiInput,
    event_ordinals: Option<&[usize]>,
) -> Option<(Option<usize>, Option<Point>, Option<u8>)> {
    if input.events.is_empty() {
        return Some((
            None,
            input.pointer.position,
            Some(input.pointer.click_count),
        ));
    }
    let ordinals = event_ordinals?;
    input
        .events
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                click_count,
                position,
            } => Some((Some(ordinals[index]), *position, Some(*click_count))),
            UiInputEvent::PointerReleaseAll { position } => {
                Some((Some(ordinals[index]), *position, None))
            }
            UiInputEvent::WindowFocusChanged(false) => Some((Some(ordinals[index]), None, None)),
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
