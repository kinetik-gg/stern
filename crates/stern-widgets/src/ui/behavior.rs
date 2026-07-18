use std::hash::Hash;

#[allow(unused_imports)]
use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource,
    CapturedDomainDragGesture, ClipId, Color, ComponentState, DropTargetResponse, FrameContext,
    FrameOutput, ImageId, Insets, PhysicalSize, PlatformRequest, Primitive, Rect, RepaintRequest,
    Response, ScaleFactor, ScrollResponse, SemanticNode, Size, TextPrimitive, Theme, TimeInfo,
    Transform, Ui as CoreUi, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, context_menu_trigger,
    draggable, drop_target, focusable, pressable, scrollable, selectable, tooltip_trigger,
};
#[allow(unused_imports)]
use stern_text::{
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
    MenuOverlay, MultiLineTextFieldOutput, NumericInputOutput, NumericScrubInputConfig,
    NumericScrubInputOutput, OverlayStack, PanelFrame, PathFieldConfig, PathFieldOutput,
    PropertyGridAffordanceOutput, PropertyGridAffordanceRects, PropertyGridRow, SearchFieldOutput,
    SelectFieldConfig, SelectFieldOutput, SliderStep, TextFieldOutput, VectorScrubInputConfig,
    VectorScrubInputOutput, WidgetOutput, asset_slot_field as asset_slot_field_widget,
    button as button_widget, checkbox as checkbox_widget,
    checkbox_with_label as checkbox_with_label_widget,
    checkbox_with_label_target as checkbox_with_label_target_widget,
    color_field as color_field_widget, icon_button as fallback_icon_button_widget,
    image as image_widget, image_icon_button as image_icon_button_widget,
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
    /// Resolves neutral press/click behavior without painting.
    pub fn pressable(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        self.pressable_with_id(id, rect, disabled)
    }

    /// Resolves neutral press/click behavior for a precomputed widget ID.
    pub fn pressable_with_id(&mut self, id: WidgetId, rect: Rect, disabled: bool) -> Response {
        let (input, memory) = self.runtime.input_and_memory_mut();
        pressable(id, rect, input, memory, disabled)
    }

    /// Resolves neutral focus behavior without painting.
    pub fn focusable(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        focusable(id, rect, input, memory, disabled)
    }

    /// Resolves neutral selectable behavior without painting.
    pub fn selectable(
        &mut self,
        key: impl Hash,
        rect: Rect,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        selectable(id, rect, input, memory, selected, disabled)
    }

    /// Resolves neutral selectable behavior and assigns the value when clicked.
    pub fn selectable_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.selectable(key, rect, *selected == value, disabled);
        if response.clicked {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Resolves neutral draggable behavior without painting.
    pub fn draggable(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        draggable(id, rect, input, memory, disabled)
    }

    pub(crate) fn captured_domain_drag_gesture_with_id(
        &mut self,
        id: WidgetId,
        rect: Rect,
        disabled: bool,
    ) -> CapturedDomainDragGesture {
        self.runtime
            .captured_domain_drag_gesture(id, rect, disabled)
    }

    pub(crate) fn cancel_pointer_interaction(&mut self) {
        self.runtime.memory_mut().cancel_pointer_interaction();
    }

    /// Resolves neutral context-menu trigger behavior without painting.
    pub fn context_menu_trigger(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        context_menu_trigger(id, rect, input, memory, disabled)
    }

    /// Resolves neutral tooltip trigger behavior without painting.
    pub fn tooltip_trigger(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        tooltip_trigger(id, rect, input, memory, disabled)
    }

    /// Resolves neutral drop-target behavior without painting.
    pub fn drop_target(
        &mut self,
        key: impl Hash,
        rect: Rect,
        disabled: bool,
    ) -> DropTargetResponse {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        drop_target(id, rect, input, memory, disabled)
    }
}
