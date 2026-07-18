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

#[allow(unused_imports)]
use super::{
    RadioGroupChoice, RadioGroupOutput, ScrollAreaOutput, TextVisualState, Ui,
    normalize_radio_group_selection, rect_key, response_activated,
    response_requests_followup_repaint, selected_radio_group_index, slider_value_changed,
    text_caret_next_blink_delay, text_caret_visible, text_layout_key,
    update_radio_group_output_selection,
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
    pub(super) fn push_widget_output(&mut self, output: &WidgetOutput) {
        if let Some(response) = output.response
            && response_requests_followup_repaint(response, self.runtime.input())
        {
            self.runtime.request_repaint(RepaintRequest::NextFrame);
        }
        self.extend(output.primitives.clone());
        for node in &output.semantics {
            self.runtime.push_semantic_node(node.clone());
        }
        self.push_widget_platform_requests(
            output.response,
            output.platform_requests.iter().cloned(),
        );
    }

    pub(super) fn push_interactive(&mut self, output: WidgetOutput) -> Response {
        let response = output.response.expect("interactive widget response");
        if response_requests_followup_repaint(response, self.runtime.input()) {
            self.runtime.request_repaint(RepaintRequest::NextFrame);
        }
        self.extend(output.primitives);
        for node in output.semantics {
            self.runtime.push_semantic_node(node);
        }
        self.push_widget_platform_requests(Some(response), output.platform_requests);
        response
    }

    pub(super) fn push_widget_platform_requests(
        &mut self,
        response: Option<Response>,
        requests: impl IntoIterator<Item = PlatformRequest>,
    ) {
        for request in requests {
            match request {
                PlatformRequest::SetCursor(cursor) => {
                    if let Some(response) = response {
                        self.runtime.request_cursor_for(response.id, cursor);
                    } else {
                        self.runtime
                            .push_platform_request(PlatformRequest::SetCursor(cursor));
                    }
                }
                request => self.runtime.push_platform_request(request),
            }
        }
    }

    pub(super) fn request_repaint_if_text_visual_changed(
        &mut self,
        before: &TextVisualState,
        state: &TextEditState,
    ) {
        if *before != TextVisualState::from_state(state) {
            self.runtime.request_repaint(RepaintRequest::NextFrame);
        }
    }

    pub(super) fn request_text_caret_blink_repaint(&mut self, output: &WidgetOutput) {
        if output
            .response
            .is_some_and(|response| response.state.focused && !response.state.disabled)
        {
            self.runtime
                .request_repaint(RepaintRequest::After(text_caret_next_blink_delay(
                    self.time(),
                )));
        }
    }

    pub(super) fn attach_text_layout(&mut self, primitive: &mut Primitive) {
        let Some(text_layouts) = self.text_layouts.as_deref_mut() else {
            return;
        };
        let Primitive::Text(text) = primitive else {
            return;
        };
        if let Some(layout) = text.layout {
            let _ = text_layouts.touch_layout(layout);
            return;
        }

        text.layout = text_layouts.try_layout_id(text_layout_key(text));
    }
}
