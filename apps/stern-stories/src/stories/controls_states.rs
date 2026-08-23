//! Interactive-state sheet for the controls families: buttons, choice
//! controls, fields, and sliders rendered in their hover and focus recipe
//! states. Pressed shares the pressed-hover styling path and is covered by
//! interaction conformance tests; a static render cannot hold a press
//! transaction open, so it is deliberately absent here.

use stern::core::{Rect, UiMemory, WidgetId};
use stern::text::TextEditState;
use stern::widgets::{DropdownModel, SelectFieldConfig, Ui};

use crate::story::{Story, StoryKind};

/// Controls state sheet: hover/focus states seeded through retained memory.
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "controls/states",
        title: "Controls hover and focus states",
        kind: StoryKind::Component,
        compose,
        seed_memory: Some(seed_states),
    }
}

const ROW_HEIGHT: f32 = 28.0;
const ROW_GAP: f32 = 14.0;

const HOVER_BUTTON: &str = "states-button-hover";
const FOCUS_BUTTON: &str = "states-button-focus";
const HOVER_CHECKBOX: &str = "states-checkbox-hover";
const FOCUS_RADIO: &str = "states-radio-focus";
const HOVER_TOGGLE: &str = "states-toggle-hover";
const FOCUS_FIELD: &str = "states-field-focus";
const HOVER_SLIDER: &str = "states-slider-hover";
const FOCUS_SLIDER: &str = "states-slider-focus";
const HOVER_SELECT: &str = "states-select-hover";

fn seed_states(memory: &mut UiMemory) {
    let hover_button = WidgetId::from_key("root").child(HOVER_BUTTON);
    let focus_button = WidgetId::from_key("root").child(FOCUS_BUTTON);
    let hover_checkbox = WidgetId::from_key("root").child(HOVER_CHECKBOX);
    let focus_radio = WidgetId::from_key("root").child(FOCUS_RADIO);
    let hover_toggle = WidgetId::from_key("root").child(HOVER_TOGGLE);
    let focus_field = WidgetId::from_key("root").child(FOCUS_FIELD);
    let hover_slider = WidgetId::from_key("root").child(HOVER_SLIDER);
    let focus_slider = WidgetId::from_key("root").child(FOCUS_SLIDER);
    let hover_select = WidgetId::from_key("root").child(HOVER_SELECT);
    memory.set_hovered(hover_button);
    memory.set_focused(Some(focus_button));
    memory.set_hovered(hover_checkbox);
    memory.set_focused(Some(focus_radio));
    memory.set_hovered(hover_toggle);
    memory.set_focused(Some(focus_field));
    memory.set_hovered(hover_slider);
    memory.set_focused(Some(focus_slider));
    memory.set_hovered(hover_select);
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let inset = 16.0;
    let width = (rect.width - inset * 2.0).max(0.0);
    let x = rect.x + inset;
    let mut y = rect.y + inset;
    let mut row = |height: f32| {
        let rect = Rect::new(x, y, width, height);
        y += height + ROW_GAP;
        rect
    };
    let cell = |row_rect: Rect, index: usize, cells: usize| {
        #[allow(clippy::cast_precision_loss)] // story cell counts are tiny constants
        let index_f32 = index as f32;
        #[allow(clippy::cast_precision_loss)] // story cell counts are tiny constants
        let cells_f32 = cells as f32;
        let w = (row_rect.width / cells_f32 - 12.0).max(0.0);
        Rect::new(
            row_rect.x + (w + 12.0) * index_f32,
            row_rect.y,
            w,
            row_rect.height,
        )
    };

    ui.label(row(16.0), "Buttons — default, hovered, focused, disabled");
    let buttons = row(ROW_HEIGHT);
    let _ = ui.button(
        "states-button-default",
        cell(buttons, 0, 4),
        "Default",
        false,
    );
    let _ = ui.button(HOVER_BUTTON, cell(buttons, 1, 4), "Hovered", false);
    let _ = ui.button(FOCUS_BUTTON, cell(buttons, 2, 4), "Focused", false);
    let _ = ui.button(
        "states-button-disabled",
        cell(buttons, 3, 4),
        "Disabled",
        true,
    );

    ui.label(
        row(16.0),
        "Choice — checkbox hovered, radio focused, toggle hovered",
    );
    let choices = row(24.0);
    let _ = ui.checkbox_with_label(
        HOVER_CHECKBOX,
        cell(choices, 0, 3),
        "Checkbox hover",
        true,
        false,
    );
    let _ =
        ui.radio_button_with_label(FOCUS_RADIO, cell(choices, 1, 3), "Radio focus", true, false);
    let _ = ui.toggle_with_label(
        HOVER_TOGGLE,
        cell(choices, 2, 3),
        "Toggle hover",
        true,
        false,
    );

    ui.label(row(16.0), "Fields — field focused, select hovered");
    let fields = row(ROW_HEIGHT);
    let half = (fields.width / 2.0 - 8.0).max(0.0);
    let mut focused_field = TextEditState::new("Focused field");
    let _ = ui.text_field(
        FOCUS_FIELD,
        Rect::new(fields.x, fields.y, half, fields.height),
        &mut focused_field,
        false,
    );
    let select_model = DropdownModel::new();
    let _ = ui.select_field(
        HOVER_SELECT,
        Rect::new(fields.x + half + 16.0, fields.y, half, fields.height),
        "Hovered select",
        &select_model,
        SelectFieldConfig::new("Pick one"),
    );

    ui.label(row(16.0), "Sliders — hovered, focused");
    let sliders = row(ROW_HEIGHT);
    let mut hovered_value = 0.35_f32;
    let mut focused_value = 0.65_f32;
    let _ = ui.slider_with_label(
        HOVER_SLIDER,
        cell(sliders, 0, 2),
        "Hovered",
        &mut hovered_value,
        0.0..=1.0,
        false,
    );
    let _ = ui.slider_with_label(
        FOCUS_SLIDER,
        cell(sliders, 1, 2),
        "Focused",
        &mut focused_value,
        0.0..=1.0,
        false,
    );
}
