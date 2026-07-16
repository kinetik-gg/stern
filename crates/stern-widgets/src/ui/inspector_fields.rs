use std::hash::Hash;

#[allow(unused_imports)]
use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, ClipId, Color,
    ComponentState, DropTargetResponse, FrameContext, FrameOutput, ImageId, Insets, PhysicalSize,
    PlatformRequest, Primitive, Rect, RepaintRequest, Response, ScaleFactor, ScrollResponse,
    SemanticNode, Size, TextPrimitive, Theme, TimeInfo, Transform, Ui as CoreUi, UiInput, UiMemory,
    Vec2, ViewportInfo, WidgetId, context_menu_trigger, draggable, drop_target, focusable,
    pressable, scrollable, selectable, tooltip_trigger,
};
#[allow(unused_imports)]
use stern_text::{
    TextComposition, TextEditState, TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
};

use crate::components::select_field_with_text_layouts as select_field_widget;

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
    numeric_scrub_input_with_runtime as numeric_scrub_input_widget, panel as panel_widget,
    panel_semantics, path_field_with_text_layouts_and_caret_visibility as path_field_widget,
    property_grid_row_affordance_controls as property_grid_row_affordance_controls_widget,
    radio_button as radio_button_widget, radio_button_with_label as radio_button_with_label_widget,
    radio_button_with_label_target as radio_button_with_label_target_widget,
    search_field_with_text_layouts_and_caret_visibility as search_field_widget,
    separator as separator_widget, slider as slider_widget,
    slider_with_label as slider_with_label_widget,
    slider_with_label_and_step as slider_with_label_and_step_widget,
    slider_with_step as slider_with_step_widget, tab_button as tab_button_widget,
    text_field_with_text_layouts_and_caret_visibility as text_field_widget,
    toggle as toggle_widget, toggle_with_label as toggle_with_label_widget,
    toggle_with_label_target as toggle_with_label_target_widget,
    vector_scrub_input_with_runtime as vector_scrub_input_widget, vector2_component_rects,
    vector3_component_rects, vector4_component_rects,
};

impl Ui<'_> {
    /// Emits a numeric input field with horizontal scrub adjustment.
    pub fn numeric_scrub_input(
        &mut self,
        key: impl Hash,
        rect: Rect,
        value: &mut f32,
        state: &mut TextEditState,
        config: NumericScrubInputConfig,
    ) -> NumericScrubInputOutput {
        let id = self.id(key);
        let theme = self.theme;
        let before_text = TextVisualState::from_state(state);
        let before_value = *value;
        let caret_visible = text_caret_visible(self.time());
        let text_layouts = self.text_layouts.as_deref_mut();
        let output = numeric_scrub_input_widget(
            &mut self.runtime,
            id,
            rect,
            value,
            state,
            config,
            theme,
            text_layouts,
            caret_visible,
        );
        self.push_widget_output(&output.input.field.widget);
        self.request_text_caret_blink_repaint(&output.input.field.widget);
        self.request_repaint_if_text_visual_changed(&before_text, state);
        if slider_value_changed(before_value, *value) || output.scrub_response.dragged {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    /// Emits a Vec2 numeric scrub field.
    pub fn vector2_scrub_input(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl AsRef<str>,
        values: &mut [f32; 2],
        states: &mut [TextEditState; 2],
        config: VectorScrubInputConfig,
    ) -> VectorScrubInputOutput<2> {
        let component_rects = vector2_component_rects(rect, config.layout);
        self.vector_scrub_input(
            key,
            rect,
            label.as_ref(),
            values,
            states,
            config,
            component_rects,
        )
    }

    /// Emits a Vec3 numeric scrub field.
    pub fn vector3_scrub_input(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl AsRef<str>,
        values: &mut [f32; 3],
        states: &mut [TextEditState; 3],
        config: VectorScrubInputConfig,
    ) -> VectorScrubInputOutput<3> {
        let component_rects = vector3_component_rects(rect, config.layout);
        self.vector_scrub_input(
            key,
            rect,
            label.as_ref(),
            values,
            states,
            config,
            component_rects,
        )
    }

    /// Emits a Vec4 numeric scrub field.
    pub fn vector4_scrub_input(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl AsRef<str>,
        values: &mut [f32; 4],
        states: &mut [TextEditState; 4],
        config: VectorScrubInputConfig,
    ) -> VectorScrubInputOutput<4> {
        let component_rects = vector4_component_rects(rect, config.layout);
        self.vector_scrub_input(
            key,
            rect,
            label.as_ref(),
            values,
            states,
            config,
            component_rects,
        )
    }

    /// Emits a backend-independent color swatch/picker entry field.
    pub fn color_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        color: Color,
        config: ColorFieldConfig,
    ) -> ColorFieldOutput {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = color_field_widget(id, rect, label, color, config, input, memory, theme);
        self.push_widget_output(&output.widget);
        output
    }

    /// Emits compact reset/keyframe controls for a property-grid row.
    pub fn property_grid_row_affordance_controls(
        &mut self,
        key: impl Hash,
        row: &PropertyGridRow,
        rects: PropertyGridAffordanceRects,
    ) -> PropertyGridAffordanceOutput {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output =
            property_grid_row_affordance_controls_widget(id, row, rects, input, memory, theme);
        self.push_widget_output(&output.widget);
        output
    }

    /// Emits an inspector select/enum field backed by a dropdown model.
    pub fn select_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        model: &DropdownModel,
        config: SelectFieldConfig,
    ) -> SelectFieldOutput {
        let id = self.id(key);
        let theme = self.theme;
        let text_layouts = self.text_layouts.as_deref_mut();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = select_field_widget(
            id,
            rect,
            label,
            model,
            config,
            input,
            memory,
            theme,
            text_layouts,
        );
        self.push_widget_output(&output.widget);
        output
    }

    /// Emits an inspector asset slot field.
    pub fn asset_slot_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        label: impl Into<String>,
        asset: Option<&AssetSlotAsset>,
        config: AssetSlotConfig,
    ) -> AssetSlotOutput {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = asset_slot_field_widget(id, rect, label, asset, config, input, memory, theme);
        self.push_widget_output(&output.widget);
        output
    }
    #[allow(clippy::too_many_arguments)]
    fn vector_scrub_input<const N: usize>(
        &mut self,
        key: impl Hash,
        _rect: Rect,
        label: &str,
        values: &mut [f32; N],
        states: &mut [TextEditState; N],
        config: VectorScrubInputConfig,
        component_rects: [crate::VectorComponentRect; N],
    ) -> VectorScrubInputOutput<N> {
        let id = self.id(key);
        let theme = self.theme;
        let before_text = states
            .iter()
            .map(TextVisualState::from_state)
            .collect::<Vec<_>>();
        let before_values = *values;
        let caret_visible = text_caret_visible(self.time());
        let text_layouts = self.text_layouts.as_deref_mut();
        let output = vector_scrub_input_widget(
            &mut self.runtime,
            id,
            label,
            values,
            states,
            config,
            theme,
            text_layouts,
            caret_visible,
            component_rects,
        );
        self.push_widget_output(&output.widget);
        for component in &output.components {
            self.request_text_caret_blink_repaint(&component.input.field.widget);
        }
        if before_text
            .iter()
            .zip(states.iter())
            .any(|(before, state)| *before != TextVisualState::from_state(state))
            || before_values
                .iter()
                .zip(values.iter())
                .any(|(before, after)| slider_value_changed(*before, *after))
            || output
                .components
                .iter()
                .any(|component| component.scrub_response.dragged)
        {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }
}
