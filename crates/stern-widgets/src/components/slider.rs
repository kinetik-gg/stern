use super::common::push_focus_ring;
use super::{
    ComponentState, CursorShape, DEFAULT_SLIDER_PAGE_DIVISIONS, DEFAULT_SLIDER_STEP_DIVISIONS, Key,
    KeyState, Primitive, Rect, RectPrimitive, Theme, UiInput, UiMemory, WidgetId, WidgetOutput,
    draggable, response_reported_focus, response_reported_pressed, slider_semantics,
    suppress_disabled_interaction_reporting, with_hover_cursor, with_response_state,
};

/// Keyboard adjustment contract for sliders.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SliderStep {
    /// Arrow-key adjustment. Non-finite and non-positive values use the range-derived default.
    pub step: f32,
    /// PageUp/PageDown adjustment. Non-finite and non-positive values use the range-derived default.
    pub page_step: f32,
}

impl SliderStep {
    /// Creates a slider step contract with `page_step` set to ten times `step`.
    #[must_use]
    pub const fn new(step: f32) -> Self {
        Self {
            step,
            page_step: step * 10.0,
        }
    }

    /// Sets the PageUp/PageDown adjustment.
    #[must_use]
    pub const fn with_page_step(mut self, page_step: f32) -> Self {
        self.page_step = page_step;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResolvedSliderStep {
    step: f32,
    page_step: f32,
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
    slider_with_label_and_step(
        id,
        rect,
        "Slider",
        value,
        range,
        SliderStep::default(),
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a slider with an accessible label and updates its value while active.
#[allow(clippy::too_many_arguments)]
pub fn slider_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    slider_with_label_and_step(
        id,
        rect,
        label,
        value,
        range,
        SliderStep::default(),
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a slider and updates its value while active or focused for keyboard input.
#[allow(clippy::too_many_arguments)]
pub fn slider_with_step(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    step: SliderStep,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    slider_with_label_and_step(
        id, rect, "Slider", value, range, step, input, memory, theme, disabled,
    )
}

/// Emits a labeled slider and updates its value while active or focused for keyboard input.
#[allow(clippy::too_many_arguments)]
pub fn slider_with_label_and_step(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    step: SliderStep,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = draggable(id, rect, input, memory, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let (start, end) = slider_range_bounds(&range);
    if !disabled
        && (response.state.active || (response.clicked && !response.keyboard_activated))
        && let Some(position) = input.pointer.position
    {
        let t = slider_position_fraction(position.x, rect);
        *value = slider_value_from_fraction(start, end, t);
    }
    if !disabled
        && response.state.focused
        && let Some(adjusted_value) =
            slider_keyboard_adjusted_value(*value, start, end, step, input)
    {
        *value = adjusted_value;
        response.keyboard_activated = true;
    }
    let display_value = slider_clamped_value(*value, start, end);
    let t = slider_value_fraction(display_value, start, end);
    let semantic_range = start.min(end)..=start.max(end);
    let fill_rect = Rect::new(rect.x, rect.y, rect.width * t, rect.height);
    let recipe = theme.slider(ComponentState {
        hovered: response.state.hovered,
        pressed: response_reported_pressed(&response),
        focused: response_reported_focus(&response),
        disabled,
        selected: false,
    });
    let mut primitives = Vec::with_capacity(4);
    push_focus_ring(
        &mut primitives,
        theme,
        response_reported_focus(&response),
        rect,
        recipe.radius,
    );
    primitives.extend([
        Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.track),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }),
        Primitive::Rect(RectPrimitive {
            rect: fill_rect,
            fill: Some(recipe.fill),
            stroke: None,
            radius: recipe.radius,
        }),
    ]);

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            slider_semantics(id, rect, label, display_value, semantic_range, disabled),
            &response,
        )),
        &response,
        CursorShape::ResizeHorizontal,
    )
}

fn slider_keyboard_adjusted_value(
    value: f32,
    start: f32,
    end: f32,
    step: SliderStep,
    input: &UiInput,
) -> Option<f32> {
    let resolved = resolve_slider_step(step, start, end);
    let min = start.min(end);
    let max = start.max(end);
    let mut adjusted = slider_clamped_value(value, start, end);
    let mut changed = false;

    for event in &input.keyboard.events {
        if event.state != KeyState::Pressed || !event.modifiers.is_empty() {
            continue;
        }

        adjusted = match event.key {
            Key::ArrowRight | Key::ArrowUp => adjusted + resolved.step,
            Key::ArrowLeft | Key::ArrowDown => adjusted - resolved.step,
            Key::PageUp => adjusted + resolved.page_step,
            Key::PageDown => adjusted - resolved.page_step,
            Key::Home => min,
            Key::End => max,
            _ => continue,
        };
        adjusted = slider_clamped_value(adjusted, start, end);
        changed = true;
    }

    changed.then_some(adjusted)
}

fn resolve_slider_step(step: SliderStep, start: f32, end: f32) -> ResolvedSliderStep {
    let span = (end - start).abs();
    let default_step = if span.is_finite() && span > f32::EPSILON {
        span / DEFAULT_SLIDER_STEP_DIVISIONS
    } else {
        0.0
    };
    let default_page_step = if span.is_finite() && span > f32::EPSILON {
        span / DEFAULT_SLIDER_PAGE_DIVISIONS
    } else {
        0.0
    };

    ResolvedSliderStep {
        step: sanitize_slider_step(step.step, default_step),
        page_step: sanitize_slider_step(step.page_step, default_page_step),
    }
}

fn sanitize_slider_step(value: f32, default: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

fn slider_range_bounds(range: &core::ops::RangeInclusive<f32>) -> (f32, f32) {
    let start = *range.start();
    let end = *range.end();
    match (start.is_finite(), end.is_finite()) {
        (true, true) => (start, end),
        (true, false) => (start, start),
        (false, true) => (end, end),
        (false, false) => (0.0, 0.0),
    }
}

fn slider_clamped_value(value: f32, start: f32, end: f32) -> f32 {
    if !value.is_finite() {
        return start;
    }
    value.clamp(start.min(end), start.max(end))
}

fn slider_position_fraction(position_x: f32, rect: Rect) -> f32 {
    if !position_x.is_finite() || !rect.x.is_finite() || !rect.width.is_finite() {
        return 0.0;
    }
    if rect.width <= f32::EPSILON {
        return 0.0;
    }
    ((position_x - rect.x) / rect.width).clamp(0.0, 1.0)
}

fn slider_value_fraction(value: f32, start: f32, end: f32) -> f32 {
    let span = end - start;
    if !value.is_finite() || !start.is_finite() || !span.is_finite() {
        return 0.0;
    }
    if span.abs() <= f32::EPSILON {
        return 0.0;
    }
    ((value - start) / span).clamp(0.0, 1.0)
}

fn slider_value_from_fraction(start: f32, end: f32, fraction: f32) -> f32 {
    if !start.is_finite() || !end.is_finite() || !fraction.is_finite() {
        return start;
    }
    start + (end - start) * fraction.clamp(0.0, 1.0)
}
