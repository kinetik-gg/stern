use super::hit::HitTarget;
use super::{
    DropTargetResponse, InteractionState, Response, canonical_pointer_fenced, pressable,
    pressable_transformed,
};
use crate::{
    Key, KeyState, MouseButton, PointerRoute, Rect, Transform, UiInput, UiInputEvent, UiMemory,
    WidgetId,
};

/// Resolves neutral context-menu trigger behavior.
pub fn context_menu_trigger(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable(id, rect, input, memory, disabled);
    response.context_requested =
        !disabled && (response.secondary_clicked || keyboard_context_requested(id, input, memory));
    response
}

/// Resolves neutral context-menu trigger behavior with transformed local-space hit testing.
pub fn context_menu_trigger_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let mut response = pressable_transformed(id, rect, local_to_screen, input, memory, disabled);
    response.context_requested =
        !disabled && (response.secondary_clicked || keyboard_context_requested(id, input, memory));
    response
}

/// Resolves neutral tooltip trigger behavior.
pub fn tooltip_trigger(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    tooltip_trigger_with_hit_target(id, rect, HitTarget::Rect, input, memory, disabled)
}

/// Resolves neutral tooltip trigger behavior with transformed local-space hit testing.
pub fn tooltip_trigger_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    tooltip_trigger_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
    )
}

fn tooltip_trigger_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> Response {
    let hovered = !disabled
        && !memory.pointer_input_conflicted(input)
        && !canonical_pointer_fenced(input)
        && hit_target.routed_hit_test(id, rect, input, memory);
    if hovered {
        memory.set_hovered(id);
    }

    let mut response = Response::new(
        id,
        rect,
        InteractionState {
            hovered,
            focused: memory.is_focused(id),
            active: false,
            pressed: false,
            disabled,
            selected: false,
        },
    );
    response.tooltip_requested = hovered
        && !input.pointer.primary.down
        && !input.pointer.secondary.down
        && !input.pointer.middle.down;
    response
}

/// Resolves neutral drop-target behavior for active drags.
pub fn drop_target(
    id: WidgetId,
    rect: Rect,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> DropTargetResponse {
    drop_target_with_hit_target(id, rect, HitTarget::Rect, input, memory, disabled)
}

/// Resolves neutral drop-target behavior for active drags with transformed local-space hit testing.
pub fn drop_target_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> DropTargetResponse {
    drop_target_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        input,
        memory,
        disabled,
    )
}

fn drop_target_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> DropTargetResponse {
    let drag_release = drag_release(input, memory);
    let planned_drag_source = (!input.events.is_empty()
        && matches!(memory.pointer_drop_route(), PointerRoute::Target(_)))
    .then(|| memory.planned_drag_source())
    .flatten();
    let termination = if drag_release.is_none() && planned_drag_source.is_some() {
        DragTermination::Pending
    } else {
        drag_termination(input, drag_release)
    };
    let input_conflicted = memory.pointer_input_conflicted(input);
    let source_candidate = drag_release
        .map(|release| release.source)
        .or_else(|| memory.drag_source())
        .or(planned_drag_source)
        .filter(|source| *source != id);
    let target_hit = hit_target.hit_test(rect, input);
    let (release_seen, source_hit) = match termination {
        DragTermination::Pending => (false, target_hit),
        DragTermination::Release { position } => {
            (true, hit_target.hit_test_position(rect, position))
        }
        DragTermination::Cancelled => (false, false),
    };
    let route_allows = match termination {
        DragTermination::Release { .. } if input.events.is_empty() => {
            memory.pointer_drop_route_allows(id)
        }
        DragTermination::Release { .. } => memory.pointer_drop_route_is_planned_for(id),
        DragTermination::Pending => memory.pointer_drop_route_allows(id),
        DragTermination::Cancelled => false,
    };
    let pointer_cancelled = input_conflicted
        || matches!(termination, DragTermination::Cancelled)
        || (matches!(termination, DragTermination::Pending)
            && memory.pointer_interaction_cancelled());
    let hovered = !pointer_cancelled
        && !disabled
        && if source_candidate.is_some() {
            source_hit && route_allows
        } else {
            hit_target.routed_hit_test(id, rect, input, memory)
        };
    let source = if !pointer_cancelled
        && !disabled
        && source_candidate.is_some()
        && source_hit
        && route_allows
    {
        source_candidate
    } else {
        None
    };
    if hovered {
        memory.set_hovered(id);
    }
    let dropped = !pointer_cancelled && !disabled && hovered && release_seen && source.is_some();
    let response = Response::new(
        id,
        rect,
        InteractionState {
            hovered,
            focused: memory.is_focused(id),
            active: false,
            pressed: false,
            disabled,
            selected: false,
        },
    );

    DropTargetResponse {
        response,
        source,
        dropped,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragTermination {
    Pending,
    Release { position: Option<crate::Point> },
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DragRelease {
    source: WidgetId,
    position: Option<crate::Point>,
}

fn drag_release(input: &UiInput, memory: &UiMemory) -> Option<DragRelease> {
    if input.events.is_empty() {
        let source = memory
            .released_drag_source()
            .or_else(|| memory.drag_source())?;
        return input.pointer.primary.released.then_some(DragRelease {
            source,
            position: input.pointer.position,
        });
    }

    match memory.pointer_drop_route() {
        PointerRoute::Target(_) => memory.planned_drag_release().and_then(|release| {
            canonical_drag_release(input, memory, release.source, release.ordinal)
        }),
        PointerRoute::Blocked => None,
        PointerRoute::Unplanned => {
            let (source, release_ordinal) = (
                memory.released_drag_source()?,
                memory.released_drag_ordinal()?,
            );
            canonical_drag_release(input, memory, source, release_ordinal)
        }
    }
}

fn canonical_drag_release(
    input: &UiInput,
    memory: &UiMemory,
    source: WidgetId,
    release_ordinal: usize,
) -> Option<DragRelease> {
    input
        .events
        .iter()
        .enumerate()
        .find_map(|(event_index, event)| {
            (memory.scoped_pointer_event_ordinal(event_index) == release_ordinal)
                .then_some((event_index, event))
        })
        .and_then(|(event_index, event)| match event {
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                position,
                ..
            } if !memory.scoped_pointer_event_is_cleanup(event_index) => Some(DragRelease {
                source,
                position: *position,
            }),
            _ => None,
        })
}

fn drag_termination(input: &UiInput, release: Option<DragRelease>) -> DragTermination {
    if let Some(release) = release {
        return DragTermination::Release {
            position: release.position,
        };
    }
    if input.events.is_empty() {
        return DragTermination::Pending;
    }
    if input.events.iter().any(|event| {
        matches!(
            event,
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                ..
            } | UiInputEvent::PointerReleaseAll { .. }
                | UiInputEvent::WindowFocusChanged(false)
        )
    }) {
        DragTermination::Cancelled
    } else {
        DragTermination::Pending
    }
}

fn keyboard_context_requested(id: WidgetId, input: &UiInput, memory: &UiMemory) -> bool {
    memory.is_focused(id)
        && input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && (matches!(event.key, Key::ContextMenu)
                    || (event.modifiers.shift && matches!(event.key, Key::Function(10))))
        })
}
