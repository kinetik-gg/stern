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
    /// Emits a slider and mutates its value while active.
    pub fn slider(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let before = *value;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = slider_widget(id, rect, value, range, input, memory, theme, disabled);
        let value_changed = slider_value_changed(before, *value);
        if value_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        self.push_interactive(output)
    }

    /// Emits a slider with an explicit keyboard step contract.
    pub fn slider_with_step(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
        step: SliderStep,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let before = *value;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output =
            slider_with_step_widget(id, rect, value, range, step, input, memory, theme, disabled);
        let value_changed = slider_value_changed(before, *value);
        if value_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        self.push_interactive(output)
    }

    /// Emits a slider with an accessible label and mutates its value while active.
    pub fn slider_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let before = *value;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = slider_with_label_widget(
            id, rect, label, value, range, input, memory, theme, disabled,
        );
        let value_changed = slider_value_changed(before, *value);
        if value_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        self.push_interactive(output)
    }

    /// Emits a labeled slider with an explicit keyboard step contract.
    #[allow(clippy::too_many_arguments)]
    pub fn slider_with_label_and_step(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        value: &mut f32,
        range: core::ops::RangeInclusive<f32>,
        step: SliderStep,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let before = *value;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = slider_with_label_and_step_widget(
            id, rect, label, value, range, step, input, memory, theme, disabled,
        );
        let value_changed = slider_value_changed(before, *value);
        if value_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        self.push_interactive(output)
    }
}
