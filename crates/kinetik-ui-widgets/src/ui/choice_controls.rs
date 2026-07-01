use std::hash::Hash;

#[allow(unused_imports)]
use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, ClipId, Color,
    ComponentState, DropTargetResponse, FrameContext, FrameOutput, ImageId, Insets, PhysicalSize,
    PlatformRequest, Primitive, Rect, RepaintRequest, Response, ScaleFactor, ScrollResponse,
    SemanticNode, Size, TextPrimitive, Theme, TimeInfo, Transform, Ui as CoreUi, UiInput, UiMemory,
    Vec2, ViewportInfo, WidgetId, context_menu_trigger, draggable, drop_target, focusable,
    pressable, scrollable, selectable, tooltip_trigger,
};
#[allow(unused_imports)]
use kinetik_ui_text::{
    TextComposition, TextEditState, TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
};

#[allow(unused_imports)]
use super::{
    RadioGroupChoice, RadioGroupOutput, ScrollAreaOutput, TextVisualState, Ui,
    normalize_radio_group_selection, rect_key, response_activated,
    response_requests_followup_repaint, selected_radio_group_index, slider_value_changed,
    text_caret_next_blink_delay, text_caret_visible, update_radio_group_output_selection,
};
#[allow(unused_imports)]
use crate::{
    AssetSlotAsset, AssetSlotConfig, AssetSlotOutput, ColorFieldConfig, ColorFieldOutput,
    CommandPaletteOverlay, DropdownCloseResult, DropdownItemId, DropdownModel, DropdownOverlay,
    IconId, IconLibrary, MenuOverlay, MultiLineTextFieldOutput, NumericInputOutput,
    NumericScrubInputConfig, NumericScrubInputOutput, OverlayStack, PanelFrame, PathFieldConfig,
    PathFieldOutput, PropertyGridAffordanceOutput, PropertyGridAffordanceRects, PropertyGridRow,
    SearchFieldOutput, SelectFieldConfig, SelectFieldOutput, SliderStep, TextFieldOutput,
    VectorScrubInputConfig, VectorScrubInputOutput, WidgetOutput,
    asset_slot_field as asset_slot_field_widget, button as button_widget,
    checkbox as checkbox_widget, checkbox_with_label as checkbox_with_label_widget,
    checkbox_with_label_target as checkbox_with_label_target_widget,
    color_field as color_field_widget, icon_button as fallback_icon_button_widget,
    icon_button_with_label as fallback_icon_button_with_label_widget,
    icon_button_with_library as icon_button_with_library_widget, image as image_widget,
    image_icon_button as image_icon_button_widget,
    image_icon_button_sized as image_icon_button_sized_widget,
    image_icon_selectable_button as image_icon_selectable_button_widget,
    image_icon_selectable_button_sized as image_icon_selectable_button_sized_widget,
    image_semantics, label as label_widget, label_semantics, list_row as list_row_widget,
    multi_line_text_field_with_text_layouts_and_caret_visibility as multi_line_text_field_widget,
    numeric_input_with_text_layouts_and_caret_visibility as numeric_input_widget,
    numeric_scrub_input_with_text_layouts_and_caret_visibility as numeric_scrub_input_widget,
    panel as panel_widget, panel_semantics,
    path_field_with_text_layouts_and_caret_visibility as path_field_widget,
    property_grid_row_affordance_controls as property_grid_row_affordance_controls_widget,
    radio_button as radio_button_widget, radio_button_with_label as radio_button_with_label_widget,
    radio_button_with_label_target as radio_button_with_label_target_widget,
    search_field_with_text_layouts_and_caret_visibility as search_field_widget,
    select_field as select_field_widget, separator as separator_widget, slider as slider_widget,
    slider_with_label as slider_with_label_widget,
    slider_with_label_and_step as slider_with_label_and_step_widget,
    slider_with_step as slider_with_step_widget, tab_button as tab_button_widget,
    text_field_with_text_layouts_and_caret_visibility as text_field_widget,
    toggle as toggle_widget, toggle_with_label as toggle_with_label_widget,
    toggle_with_label_target as toggle_with_label_target_widget,
    vector_scrub_input_with_text_layouts_and_caret_visibility as vector_scrub_input_widget,
    vector2_component_rects, vector3_component_rects, vector4_component_rects,
};

impl Ui<'_> {
    /// Emits a checkbox and returns its interaction response.
    pub fn checkbox(
        &mut self,
        key: impl Hash,
        rect: Rect,
        checked: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = checkbox_widget(id, rect, checked, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a checkbox, toggling the provided value when clicked.
    pub fn checkbox_value(
        &mut self,
        key: impl Hash,
        rect: Rect,
        checked: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response = self.checkbox(key, rect, *checked, disabled);
        if response_activated(&response) {
            *checked = !*checked;
            response.state.selected = *checked;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a checkbox with an accessible label and returns its interaction response.
    pub fn checkbox_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        checked: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output =
            checkbox_with_label_widget(id, rect, label, checked, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a labeled checkbox, toggling the provided value when clicked.
    pub fn checkbox_value_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        checked: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response = self.checkbox_with_label(key, rect, label, *checked, disabled);
        if response_activated(&response) {
            *checked = !*checked;
            response.state.selected = *checked;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a labeled checkbox with a separate label activation rectangle.
    pub fn checkbox_with_label_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        checked: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = checkbox_with_label_target_widget(
            id, rect, label_rect, label, checked, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a labeled checkbox with a label activation rectangle and toggles the value.
    pub fn checkbox_value_with_label_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        checked: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response =
            self.checkbox_with_label_target(key, rect, label_rect, label, *checked, disabled);
        if response_activated(&response) {
            *checked = !*checked;
            response.state.selected = *checked;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a radio button and returns its interaction response.
    pub fn radio_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = radio_button_widget(id, rect, selected, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a radio button and assigns the value when clicked.
    pub fn radio_button_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.radio_button(key, rect, *selected == value, disabled);
        if response_activated(&response) {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a radio button with an accessible label and returns its interaction response.
    pub fn radio_button_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = radio_button_with_label_widget(
            id, rect, label, selected, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a labeled radio button and assigns the value when clicked.
    pub fn radio_button_value_with_label<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response =
            self.radio_button_with_label(key, rect, label, *selected == value, disabled);
        if response_activated(&response) {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a labeled radio button with a separate label activation rectangle.
    pub fn radio_button_with_label_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = radio_button_with_label_target_widget(
            id, rect, label_rect, label, selected, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a labeled radio button with a label activation rectangle and assigns the value.
    #[allow(clippy::too_many_arguments)]
    pub fn radio_button_value_with_label_target<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.radio_button_with_label_target(
            key,
            rect,
            label_rect,
            label,
            *selected == value,
            disabled,
        );
        if response_activated(&response) {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a radio group and keeps one enabled choice selected when available.
    pub fn radio_group_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        selected: &mut T,
        choices: &[RadioGroupChoice<T>],
    ) -> RadioGroupOutput<T> {
        let normalized_index = normalize_radio_group_selection(selected, choices);
        let mut changed = normalized_index.changed;
        let mut activated_index = None;
        let mut item_outputs = Vec::with_capacity(choices.len());

        self.runtime.push_id_scope(key);
        for (index, choice) in choices.iter().enumerate() {
            let id = self.id(&choice.key);
            let is_selected = normalized_index.index == Some(index);
            let theme = self.theme;
            let (input, memory) = self.runtime.input_and_memory_mut();
            let output = radio_button_with_label_target_widget(
                id,
                choice.rect,
                choice.label_rect,
                choice.label.clone(),
                is_selected,
                input,
                memory,
                theme,
                choice.disabled,
            );
            if !choice.disabled && output.response.as_ref().is_some_and(response_activated) {
                activated_index = Some(index);
            }
            item_outputs.push(output);
        }
        self.runtime.pop_id_scope();

        if let Some(index) = activated_index {
            let value = choices[index].value;
            if *selected != value {
                *selected = value;
                changed = true;
                self.request_repaint(RepaintRequest::NextFrame);
            }
        } else if changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        let selected_index =
            activated_index.or_else(|| selected_radio_group_index(*selected, choices));
        let activated = activated_index.map(|index| choices[index].value);
        let mut responses = Vec::with_capacity(item_outputs.len());
        for (index, mut output) in item_outputs.into_iter().enumerate() {
            update_radio_group_output_selection(
                &mut output,
                self.theme,
                selected_index == Some(index),
            );
            if let Some(response) = output.response {
                responses.push(response);
            }
            self.push_widget_output(&output);
        }

        RadioGroupOutput {
            selected: *selected,
            selected_index,
            activated,
            activated_index,
            changed,
            responses,
        }
    }

    /// Emits a toggle and returns its interaction response.
    pub fn toggle(&mut self, key: impl Hash, rect: Rect, on: bool, disabled: bool) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = toggle_widget(id, rect, on, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a toggle, mutating the provided value when clicked.
    pub fn toggle_value(
        &mut self,
        key: impl Hash,
        rect: Rect,
        on: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response = self.toggle(key, rect, *on, disabled);
        if response_activated(&response) {
            *on = !*on;
            response.state.selected = *on;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a toggle with an accessible label and returns its interaction response.
    pub fn toggle_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        on: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = toggle_with_label_widget(id, rect, label, on, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a labeled toggle, mutating the provided value when clicked.
    pub fn toggle_value_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        on: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response = self.toggle_with_label(key, rect, label, *on, disabled);
        if response_activated(&response) {
            *on = !*on;
            response.state.selected = *on;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a labeled toggle with a separate label activation rectangle.
    pub fn toggle_with_label_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        on: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = toggle_with_label_target_widget(
            id, rect, label_rect, label, on, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a labeled toggle with a label activation rectangle and toggles the value.
    pub fn toggle_value_with_label_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label_rect: Rect,
        label: impl Into<String>,
        on: &mut bool,
        disabled: bool,
    ) -> Response {
        let mut response =
            self.toggle_with_label_target(key, rect, label_rect, label, *on, disabled);
        if response_activated(&response) {
            *on = !*on;
            response.state.selected = *on;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }
}
