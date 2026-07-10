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
    /// Emits a text label.
    pub fn label(&mut self, rect: Rect, text: impl Into<String>) {
        let text = text.into();
        let id = self.id(format!("label:{}:{}", rect_key("", rect), text));
        self.label_with_id(id, rect, text);
    }

    /// Emits a text label with caller-provided stable identity.
    pub fn label_keyed(&mut self, key: impl Hash, rect: Rect, text: impl Into<String>) {
        let id = self.id(key);
        self.label_with_id(id, rect, text);
    }

    fn label_with_id(&mut self, id: WidgetId, rect: Rect, text: impl Into<String>) {
        let text = text.into();
        let output = label_widget(rect, text.clone(), self.theme);
        self.push_widget_output(&output);
        self.runtime
            .push_semantic_node(label_semantics(id, rect, text));
    }

    /// Emits a passive panel surface.
    pub fn panel(&mut self, rect: Rect) {
        let id = self.id(rect_key("panel", rect));
        self.panel_with_id(id, rect);
    }

    /// Emits a passive panel surface with caller-provided stable identity.
    pub fn panel_keyed(&mut self, key: impl Hash, rect: Rect) {
        let id = self.id(key);
        self.panel_with_id(id, rect);
    }

    fn panel_with_id(&mut self, id: WidgetId, rect: Rect) {
        let output = panel_widget(rect, self.theme);
        self.push_widget_output(&output);
        self.runtime
            .push_semantic_node(panel_semantics(id, rect, "Panel"));
    }

    /// Resolves and emits a passive panel surface with an inset content body.
    pub fn panel_frame(&mut self, rect: Rect, body_insets: Insets) -> PanelFrame {
        self.panel(rect);
        PanelFrame::new(rect, body_insets)
    }

    /// Emits a passive panel and runs clipped content inside its inset body.
    pub fn panel_body<T>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        body_insets: Insets,
        f: impl FnOnce(&mut Self, Rect) -> T,
    ) -> T {
        let frame = self.panel_frame(rect, body_insets);
        self.clip_rect(key, frame.body, |ui| f(ui, frame.body))
    }

    /// Runs a closure inside a rectangular clip scope.
    pub fn clip_rect<T>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let id = self.id(key);
        let clip = ClipId::from_raw(id.raw());
        self.primitive(Primitive::ClipBegin { id: clip, rect });
        self.runtime.push_id_scope(("clip_rect_content", id.raw()));
        let inner = f(self);
        self.runtime.pop_id_scope();
        self.primitive(Primitive::ClipEnd { id: clip });
        inner
    }

    /// Emits a separator line.
    pub fn separator(&mut self, rect: Rect) {
        self.primitive(separator_widget(rect, self.theme));
    }

    /// Emits a static image.
    pub fn image(&mut self, rect: Rect, image: ImageId) {
        let id = self.id(format!("image:{}:{}", rect_key("", rect), image.raw()));
        self.image_with_id(id, rect, image);
    }

    /// Emits a static image with caller-provided stable identity.
    pub fn image_keyed(&mut self, key: impl Hash, rect: Rect, image: ImageId) {
        let id = self.id(key);
        self.image_with_id(id, rect, image);
    }

    fn image_with_id(&mut self, id: WidgetId, rect: Rect, image: ImageId) {
        let output = image_widget(rect, image);
        self.push_widget_output(&output);
        self.runtime.push_semantic_node(image_semantics(
            id,
            rect,
            format!("Image {}", image.raw()),
        ));
    }
    /// Emits a clipped, scrollable content region.
    ///
    /// The closure receives the retained offset for virtualization decisions.
    /// Child rectangles remain in content coordinates; this runtime scope owns
    /// the matching paint, input, semantic, debug, and IME translation.
    pub fn scroll_area<T>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        content_size: Size,
        disabled: bool,
        f: impl FnOnce(&mut Self, Vec2) -> T,
    ) -> ScrollAreaOutput<T> {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        let scroll = scrollable(id, rect, content_size, input, memory, disabled);
        if scroll.delta != Vec2::ZERO {
            self.runtime.request_repaint(RepaintRequest::NextFrame);
        }
        let clip = ClipId::from_raw(id.raw());

        self.runtime
            .push_semantic_node(panel_semantics(id, rect, "Scroll area"));
        self.primitive(Primitive::ClipBegin { id: clip, rect });
        self.primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(-scroll.offset.x, -scroll.offset.y),
        )));
        self.runtime
            .push_id_scope(("scroll_area_content", id.raw()));
        let inner = f(self, scroll.offset);
        self.runtime.pop_id_scope();
        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: clip });

        ScrollAreaOutput { scroll, inner }
    }
}
