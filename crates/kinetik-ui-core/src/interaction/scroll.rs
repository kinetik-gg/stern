use super::{HitTarget, InteractionState, Response, ScrollResponse, canonical_pointer_fenced};
use crate::{
    InputWheelDelta, PointerRoute, Rect, Size, Transform, UiInput, UiInputEvent, UiMemory, Vec2,
    WidgetId,
};

const DEFAULT_WHEEL_LINE_STEP: Vec2 = Vec2::new(40.0, 40.0);

/// Resolves neutral scroll behavior and stores a clamped offset in memory.
///
/// Wheel deltas follow the platform input convention. The retained scroll
/// offset increases in the opposite direction so a negative vertical wheel
/// delta, the usual "scroll down" event, moves content down by increasing the
/// stored y offset.
pub fn scrollable(
    id: WidgetId,
    rect: Rect,
    content_size: Size,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> ScrollResponse {
    scrollable_with_hit_target(
        id,
        rect,
        HitTarget::Rect,
        content_size,
        input,
        memory,
        disabled,
    )
}

/// Resolves neutral scroll behavior with transformed local-space hit testing.
///
/// Wheel deltas follow the platform input convention. The retained scroll
/// offset increases in the opposite direction so a negative vertical wheel
/// delta, the usual "scroll down" event, moves content down by increasing the
/// stored y offset.
pub fn scrollable_transformed(
    id: WidgetId,
    rect: Rect,
    local_to_screen: Transform,
    content_size: Size,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> ScrollResponse {
    scrollable_with_hit_target(
        id,
        rect,
        HitTarget::Transformed(local_to_screen),
        content_size,
        input,
        memory,
        disabled,
    )
}

fn scrollable_with_hit_target(
    id: WidgetId,
    rect: Rect,
    hit_target: HitTarget,
    content_size: Size,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> ScrollResponse {
    let conflicted = memory.pointer_input_conflicted(input);
    let target_hit = hit_target.hit_test(rect, input);
    let hovered = !conflicted
        && !disabled
        && !canonical_pointer_fenced(input)
        && target_hit
        && memory.pointer_route_allows(id);
    if hovered {
        memory.set_hovered(id);
    }

    let previous = clamp_scroll_offset(memory.scroll_offset(id), rect.size(), content_size);
    let wheel_routed = !disabled
        && target_hit
        && if input.events.is_empty() {
            memory.pointer_wheel_route_allows(id)
        } else {
            memory.pointer_wheel_route_matches(id)
        };
    let requested_delta = if wheel_routed {
        let delta = normalized_wheel_delta(input);
        Vec2::new(-delta.x, -delta.y)
    } else {
        Vec2::ZERO
    };
    let offset = clamp_scroll_offset(
        Vec2::new(
            previous.x + requested_delta.x,
            previous.y + requested_delta.y,
        ),
        rect.size(),
        content_size,
    );
    if memory.pointer_wheel_route() == PointerRoute::Unplanned {
        memory.set_scroll_offset(id, offset);
    } else {
        memory.stage_scroll_offset(id, offset);
    }

    let delta = Vec2::new(offset.x - previous.x, offset.y - previous.y);
    let max_offset = max_scroll_offset(rect.size(), content_size);
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
    response.drag_delta = delta;

    ScrollResponse {
        response,
        offset,
        delta,
        max_offset,
    }
}

fn normalized_wheel_delta(input: &UiInput) -> Vec2 {
    if input.events.is_empty() {
        return sanitize_wheel_vector(input.pointer.wheel_delta);
    }

    let mut accumulated = Vec2::ZERO;
    for event in &input.events {
        match event {
            UiInputEvent::Wheel { delta, .. } => {
                let delta = match *delta {
                    InputWheelDelta::Lines(delta) => multiply_wheel_vectors(
                        sanitize_wheel_vector(delta),
                        DEFAULT_WHEEL_LINE_STEP,
                    ),
                    InputWheelDelta::Pixels(delta) => sanitize_wheel_vector(delta),
                };
                accumulated = add_wheel_vectors(accumulated, delta);
            }
            UiInputEvent::PointerReleaseAll { .. } | UiInputEvent::WindowFocusChanged(false) => {
                break;
            }
            _ => {}
        }
    }
    accumulated
}

fn sanitize_wheel_vector(value: Vec2) -> Vec2 {
    Vec2::new(
        sanitize_wheel_component(value.x),
        sanitize_wheel_component(value.y),
    )
}

fn multiply_wheel_vectors(left: Vec2, right: Vec2) -> Vec2 {
    sanitize_wheel_vector(Vec2::new(left.x * right.x, left.y * right.y))
}

fn add_wheel_vectors(left: Vec2, right: Vec2) -> Vec2 {
    sanitize_wheel_vector(Vec2::new(left.x + right.x, left.y + right.y))
}

fn sanitize_wheel_component(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Clamps a scroll offset to the range implied by a viewport and content size.
#[must_use]
pub fn clamp_scroll_offset(offset: Vec2, viewport_size: Size, content_size: Size) -> Vec2 {
    let max_offset = max_scroll_offset(viewport_size, content_size);
    Vec2::new(
        sanitize_scroll_component(offset.x).clamp(0.0, max_offset.x),
        sanitize_scroll_component(offset.y).clamp(0.0, max_offset.y),
    )
}

/// Returns the maximum legal scroll offset for a viewport and content size.
#[must_use]
pub fn max_scroll_offset(viewport_size: Size, content_size: Size) -> Vec2 {
    Vec2::new(
        (sanitize_scroll_component(content_size.width)
            - sanitize_scroll_component(viewport_size.width))
        .max(0.0),
        (sanitize_scroll_component(content_size.height)
            - sanitize_scroll_component(viewport_size.height))
        .max(0.0),
    )
}

fn sanitize_scroll_component(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
