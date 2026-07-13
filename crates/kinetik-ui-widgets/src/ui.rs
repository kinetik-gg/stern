//! Immediate-mode composition wrapper for widget primitives.

mod basic_controls;
mod behavior;
mod choice_controls;
mod chrome;
mod collections;
mod frame;
mod inspector_fields;
mod layout;
mod output;
mod overlays;
mod passive;
mod sliders;
#[cfg(test)]
mod tests;
mod text_controls;
mod virtual_tree;

use std::time::Duration;

use kinetik_ui_core::{
    ComponentState, Primitive, Rect, Response, ScrollResponse, TextPrimitive, Theme, TimeInfo,
    Ui as CoreUi, UiInput,
};
use kinetik_ui_text::{
    TextComposition, TextEditState, TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
};

use crate::{IconLibrary, WidgetOutput};

const TEXT_CARET_BLINK_INTERVAL: Duration = Duration::from_millis(500);

fn rect_key(prefix: &str, rect: Rect) -> String {
    format!(
        "{prefix}:{:.3}:{:.3}:{:.3}:{:.3}",
        rect.x, rect.y, rect.width, rect.height
    )
}

/// Frame-local UI builder.
///
/// `Ui` is intentionally thin: it delegates runtime state and output to
/// `kinetik-ui-core` while layering ergonomic widget methods on top. This keeps
/// showcase and application code from hand-painting controls.
pub struct Ui<'a> {
    runtime: CoreUi<'a>,
    theme: &'a Theme,
    text_layouts: Option<&'a mut TextLayoutStore>,
    icons: Option<&'a IconLibrary>,
}

/// Output returned by [`Ui::scroll_area`].
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollAreaOutput<T> {
    /// Scroll behavior response and clamped offset data.
    pub scroll: ScrollResponse,
    /// Value returned by the scroll-area content closure.
    pub inner: T,
}

/// One rendered choice in a radio group.
#[derive(Debug, Clone, PartialEq)]
pub struct RadioGroupChoice<T> {
    /// Stable key used to derive the radio button widget ID inside the group scope.
    pub key: String,
    /// Radio control rectangle.
    pub rect: Rect,
    /// Optional label activation rectangle paired with the radio control.
    pub label_rect: Rect,
    /// Accessible label for the radio item.
    pub label: String,
    /// Value assigned when the choice is activated.
    pub value: T,
    /// Whether this choice is unavailable for selection.
    pub disabled: bool,
}

impl<T> RadioGroupChoice<T> {
    /// Creates an enabled radio-group choice.
    pub fn new(key: impl Into<String>, rect: Rect, label: impl Into<String>, value: T) -> Self {
        Self {
            key: key.into(),
            rect,
            label_rect: Rect::ZERO,
            label: label.into(),
            value,
            disabled: false,
        }
    }

    /// Sets the paired label activation rectangle.
    #[must_use]
    pub const fn with_label_rect(mut self, label_rect: Rect) -> Self {
        self.label_rect = label_rect;
        self
    }

    /// Sets whether the choice is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Output returned by [`Ui::radio_group_value`].
#[derive(Debug, Clone, PartialEq)]
pub struct RadioGroupOutput<T> {
    /// Selected value after normalization and activation handling.
    pub selected: T,
    /// Index of the selected enabled choice, when one is available.
    pub selected_index: Option<usize>,
    /// Value activated this frame, if an enabled choice was activated.
    pub activated: Option<T>,
    /// Index activated this frame, if an enabled choice was activated.
    pub activated_index: Option<usize>,
    /// True when the selected value changed during this frame.
    pub changed: bool,
    /// Per-choice interaction responses in input order.
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextVisualState {
    text: String,
    selection: TextSelection,
    composition: Option<TextComposition>,
}

impl TextVisualState {
    fn from_state(state: &TextEditState) -> Self {
        Self {
            text: state.text.clone(),
            selection: state.selection,
            composition: state.composition.clone(),
        }
    }
}

fn response_requests_followup_repaint(response: Response, input: &UiInput) -> bool {
    response.clicked
        || response.secondary_clicked
        || response.dragged
        || response.keyboard_activated
        || response.context_requested
        || (response.state.pressed && input.pointer.primary.pressed)
}

fn response_activated(response: &Response) -> bool {
    response.clicked || response.keyboard_activated
}

fn slider_value_changed(before: f32, after: f32) -> bool {
    !(before.is_nan() && after.is_nan()) && before.to_bits() != after.to_bits()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormalizedRadioGroupSelection {
    index: Option<usize>,
    changed: bool,
}

fn normalize_radio_group_selection<T: Copy + Eq>(
    selected: &mut T,
    choices: &[RadioGroupChoice<T>],
) -> NormalizedRadioGroupSelection {
    if let Some(index) = selected_radio_group_index(*selected, choices) {
        return NormalizedRadioGroupSelection {
            index: Some(index),
            changed: false,
        };
    }

    if let Some(index) = choices.iter().position(|choice| !choice.disabled) {
        *selected = choices[index].value;
        return NormalizedRadioGroupSelection {
            index: Some(index),
            changed: true,
        };
    }

    NormalizedRadioGroupSelection {
        index: None,
        changed: false,
    }
}

fn selected_radio_group_index<T: Copy + Eq>(
    selected: T,
    choices: &[RadioGroupChoice<T>],
) -> Option<usize> {
    choices
        .iter()
        .position(|choice| !choice.disabled && choice.value == selected)
}

fn update_radio_group_output_selection(output: &mut WidgetOutput, theme: &Theme, selected: bool) {
    let Some(response) = output.response.as_mut() else {
        return;
    };

    response.state.selected = selected;
    let recipe = theme.radio_button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed && !response.state.disabled,
        focused: response.state.focused && !response.state.disabled,
        disabled: response.state.disabled,
        selected,
    });

    if let Some(Primitive::Rect(primitive)) = output.primitives.first_mut() {
        primitive.fill = Some(recipe.fill);
        primitive.stroke = Some(recipe.border);
        primitive.radius = recipe.radius;
    }

    for node in &mut output.semantics {
        node.state.selected = selected;
        node.state.checked = Some(selected);
    }
}

fn text_caret_visible(time: TimeInfo) -> bool {
    let interval = TEXT_CARET_BLINK_INTERVAL.as_millis().max(1);
    (time.now.as_millis() / interval).is_multiple_of(2)
}

fn text_caret_next_blink_delay(time: TimeInfo) -> Duration {
    let interval_ms = u64::try_from(TEXT_CARET_BLINK_INTERVAL.as_millis())
        .unwrap_or(u64::MAX)
        .max(1);
    let elapsed_ms = u64::try_from(time.now.as_millis()).unwrap_or(u64::MAX);
    let remainder = elapsed_ms % interval_ms;
    Duration::from_millis((interval_ms - remainder).max(1))
}

fn text_layout_key(text: &TextPrimitive) -> TextLayoutKey {
    TextLayoutKey::new(
        text.text.clone(),
        TextStyle::new(text.family.clone(), text.size, text.line_height),
        0.0,
        false,
    )
}
