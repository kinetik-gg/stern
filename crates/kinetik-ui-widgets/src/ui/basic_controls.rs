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
    /// Emits a push button and returns its interaction response.
    pub fn button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: impl Into<String>,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = button_widget(id, rect, text, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a button for an application action and queues the invocation on activation.
    ///
    /// Hidden actions return `None` and emit no widget output. Disabled actions
    /// are shown but cannot invoke.
    pub fn action_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        action: &ActionDescriptor,
        context: ActionContext,
    ) -> Option<Response> {
        if !action.state.visible {
            return None;
        }

        let response = self.button(key, rect, action.label.clone(), !action.state.enabled);
        if response.clicked || response.keyboard_activated {
            self.invoke_action_descriptor(action, ActionSource::Button, context);
        }
        Some(response)
    }

    /// Emits an icon button and returns its interaction response.
    pub fn icon_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        icon: IconId,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let icons = self.icons;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = if let Some(icons) = icons {
            icon_button_with_library_widget(
                id,
                rect,
                icon,
                format!("Icon {}", icon.raw()),
                icons,
                input,
                memory,
                theme,
                disabled,
            )
        } else {
            fallback_icon_button_widget(id, rect, icon, input, memory, theme, disabled)
        };
        self.push_interactive(output)
    }

    /// Emits an icon button with an accessible label and returns its interaction response.
    pub fn icon_button_with_label(
        &mut self,
        key: impl Hash,
        rect: Rect,
        icon: IconId,
        label: impl Into<String>,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let icons = self.icons;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = if let Some(icons) = icons {
            icon_button_with_library_widget(
                id, rect, icon, label, icons, input, memory, theme, disabled,
            )
        } else {
            fallback_icon_button_with_label_widget(
                id, rect, icon, label, input, memory, theme, disabled,
            )
        };
        self.push_interactive(output)
    }

    /// Emits a bitmap-backed icon button with an accessible label.
    pub fn image_icon_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output =
            image_icon_button_widget(id, rect, image, label, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a bitmap-backed icon button with an explicit icon side length.
    pub fn image_icon_button_sized(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        icon_size: f32,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = image_icon_button_sized_widget(
            id, rect, image, label, icon_size, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a selectable bitmap-backed icon button with an accessible label.
    pub fn image_icon_selectable_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = image_icon_selectable_button_widget(
            id, rect, image, label, selected, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a selectable bitmap-backed icon button with an explicit icon side length.
    #[allow(clippy::too_many_arguments)]
    pub fn image_icon_selectable_button_sized(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        selected: bool,
        icon_size: f32,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = image_icon_selectable_button_sized_widget(
            id, rect, image, label, selected, icon_size, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a selectable bitmap-backed icon button and assigns the value when clicked.
    #[allow(clippy::too_many_arguments)]
    pub fn image_icon_button_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.image_icon_selectable_button(
            key,
            rect,
            image,
            label,
            *selected == value,
            disabled,
        );
        if response.clicked {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a selectable bitmap-backed icon button with explicit icon size and assigns the value.
    #[allow(clippy::too_many_arguments)]
    pub fn image_icon_button_value_sized<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        image: ImageId,
        label: impl Into<String>,
        icon_size: f32,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.image_icon_selectable_button_sized(
            key,
            rect,
            image,
            label,
            *selected == value,
            icon_size,
            disabled,
        );
        if response.clicked {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a tab header and returns its interaction response.
    pub fn tab_button(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: impl Into<String>,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = tab_button_widget(id, rect, text, selected, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a tab header and assigns the value when clicked.
    pub fn tab_button_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: impl Into<String>,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.tab_button(key, rect, text, *selected == value, disabled);
        if response.clicked {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }

    /// Emits a selectable list row and returns its interaction response.
    pub fn list_row(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: impl Into<String>,
        selected: bool,
        disabled: bool,
    ) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = list_row_widget(id, rect, text, selected, input, memory, theme, disabled);
        self.push_interactive(output)
    }

    /// Emits a selectable list row and assigns the value when clicked.
    pub fn list_row_value<T: Copy + Eq>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        text: impl Into<String>,
        selected: &mut T,
        value: T,
        disabled: bool,
    ) -> Response {
        let mut response = self.list_row(key, rect, text, *selected == value, disabled);
        if response.clicked {
            *selected = value;
            response.state.selected = true;
            self.request_repaint(RepaintRequest::NextFrame);
        }
        response
    }
}
