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

impl<'a> Ui<'a> {
    /// Creates a compatibility UI builder for one frame.
    ///
    /// Prefer [`Self::from_core`] when viewport/time context or full frame
    /// output is available.
    #[must_use]
    pub fn new(input: &'a UiInput, memory: &'a mut UiMemory, theme: &'a Theme) -> Self {
        let context = FrameContext::new(
            ViewportInfo::new(Size::ZERO, PhysicalSize::ZERO, ScaleFactor::ONE),
            input.clone(),
            TimeInfo::default(),
        );
        Self::begin_frame(context, memory, theme)
    }

    /// Starts a widget frame from a full runtime context.
    #[must_use]
    pub fn begin_frame(context: FrameContext, memory: &'a mut UiMemory, theme: &'a Theme) -> Self {
        Self::from_core(CoreUi::begin_frame(context, memory), theme)
    }

    /// Creates a widget façade over a core UI runtime.
    #[must_use]
    pub const fn from_core(runtime: CoreUi<'a>, theme: &'a Theme) -> Self {
        Self {
            runtime,
            theme,
            text_layouts: None,
            viewport_presentations: Vec::new(),
        }
    }

    /// Creates a widget façade with shaped text layout caching enabled.
    pub fn from_core_with_text_layouts(
        runtime: CoreUi<'a>,
        theme: &'a Theme,
        text_layouts: &'a mut TextLayoutStore,
    ) -> Self {
        text_layouts.advance_generation();
        Self {
            runtime,
            theme,
            text_layouts: Some(text_layouts),
            viewport_presentations: Vec::new(),
        }
    }

    /// Starts a widget frame from a full runtime context with shaped text layout caching enabled.
    pub fn begin_frame_with_text_layouts(
        context: FrameContext,
        memory: &'a mut UiMemory,
        theme: &'a Theme,
        text_layouts: &'a mut TextLayoutStore,
    ) -> Self {
        Self::from_core_with_text_layouts(CoreUi::begin_frame(context, memory), theme, text_layouts)
    }

    /// Enables shaped text layout caching for this frame.
    #[must_use]
    pub fn with_text_layouts(mut self, text_layouts: &'a mut TextLayoutStore) -> Self {
        text_layouts.advance_generation();
        self.text_layouts = Some(text_layouts);
        self
    }

    /// Returns the full frame context used by this frame.
    #[must_use]
    pub const fn context(&self) -> &FrameContext {
        self.runtime.context()
    }

    /// Returns viewport and DPI information used by this frame.
    #[must_use]
    pub const fn viewport(&self) -> ViewportInfo {
        self.runtime.context().viewport
    }

    /// Returns time information used by this frame.
    #[must_use]
    pub const fn time(&self) -> TimeInfo {
        self.runtime.context().time
    }

    /// Returns the theme used by this frame.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        self.theme
    }

    /// Returns the input snapshot used by this frame.
    #[must_use]
    pub const fn input(&self) -> &UiInput {
        self.runtime.input()
    }

    /// Returns the memory used by this frame.
    #[must_use]
    pub fn memory(&self) -> &UiMemory {
        self.runtime.memory()
    }

    /// Derives and registers a widget ID in the current scope.
    pub fn id(&mut self, key: impl Hash) -> WidgetId {
        self.runtime.id(key)
    }

    /// Registers an externally derived widget ID as present and checks duplicates.
    pub fn register_id(&mut self, id: WidgetId) -> WidgetId {
        self.runtime.register_id(id)
    }

    /// Derives a widget ID without registering it before a pointer-plan prepass.
    #[must_use]
    pub fn make_id(&self, key: impl Hash) -> WidgetId {
        self.runtime.make_id(key)
    }

    /// Resolves one closed-world pointer target plan before widget behaviors run.
    ///
    /// # Errors
    ///
    /// Returns a deterministic validation error when the frame installs a
    /// second plan or declares duplicate paint orders or conflicting IDs.
    pub fn resolve_pointer_targets(
        &mut self,
        declare: impl FnOnce(&mut stern_core::PointerTargetPlan),
    ) -> Result<stern_core::PointerRoutes, stern_core::PointerPlanError> {
        self.runtime.resolve_pointer_targets(declare)
    }

    /// Runs a closure inside a stable ID scope.
    pub fn scope<T>(&mut self, key: impl Hash, f: impl FnOnce(&mut Self) -> T) -> T {
        self.runtime.push_id_scope(key);
        let output = f(self);
        self.runtime.pop_id_scope();
        output
    }

    /// Appends an arbitrary primitive.
    pub fn primitive(&mut self, primitive: Primitive) {
        let mut primitive = primitive;
        self.attach_text_layout(&mut primitive);
        self.runtime.push_primitive(primitive);
    }

    /// Appends arbitrary primitives.
    pub fn extend(&mut self, primitives: impl IntoIterator<Item = Primitive>) {
        for primitive in primitives {
            self.primitive(primitive);
        }
    }

    /// Adds an action invocation to the current frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.runtime.push_action(invocation);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.runtime.invoke_action(action_id, source, context);
    }

    /// Requests a repaint from application code that mutates state during this frame.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.runtime.request_repaint(request);
    }

    /// Appends a semantic node for custom application-drawn UI.
    pub fn push_semantic_node(&mut self, node: SemanticNode) {
        self.runtime.push_semantic_node(node);
    }

    /// Adds a raw platform request for custom application-drawn UI.
    ///
    /// Built-in widget cursor requests are routed through the widget response
    /// owner. This escape hatch stays direct for custom low-level app surfaces.
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        self.runtime.push_platform_request(request);
    }

    /// Invokes a visible, enabled action descriptor from the provided UI source.
    ///
    /// Returns false when the descriptor is hidden or disabled.
    pub fn invoke_action_descriptor(
        &mut self,
        action: &ActionDescriptor,
        source: ActionSource,
        context: ActionContext,
    ) -> bool {
        if !action.can_invoke() {
            return false;
        }

        self.invoke_action(action.id.clone(), source, context);
        true
    }

    /// Emits an invocation for an enabled visible menu-overlay item.
    ///
    /// Returns false when the visible index is not an enabled action item.
    pub fn invoke_menu_overlay_item(
        &mut self,
        overlay: &MenuOverlay,
        visible_index: usize,
    ) -> bool {
        let Some(invocation) = overlay.invocation_for_visible(visible_index) else {
            return false;
        };
        self.push_action(invocation);
        true
    }

    /// Emits an invocation for the selected command-palette overlay item.
    ///
    /// Returns false when no enabled matching command is selected.
    pub fn invoke_command_palette_overlay(&mut self, overlay: &CommandPaletteOverlay) -> bool {
        let Some(invocation) = overlay.invocation_for_selected() else {
            return false;
        };
        self.push_action(invocation);
        true
    }

    /// Selects a dropdown-overlay item, closes its overlay, and requests a follow-up repaint.
    ///
    /// Returns `None` when the item is disabled, unknown, or the dropdown is not open.
    pub fn select_dropdown_overlay_item(
        &mut self,
        overlay: &mut DropdownOverlay,
        item_id: DropdownItemId,
        stack: &mut OverlayStack,
    ) -> Option<DropdownCloseResult> {
        let result = overlay.select_and_close(item_id, stack)?;
        self.request_repaint(RepaintRequest::NextFrame);
        Some(result)
    }

    /// Returns the accumulated frame output so far.
    #[must_use]
    pub const fn output(&self) -> &FrameOutput {
        self.runtime.output()
    }

    /// Finishes the frame and returns full core frame output.
    #[must_use]
    pub fn finish_output(self) -> FrameOutput {
        self.runtime.end_frame()
    }

    /// Finishes the frame and returns the primitive stream.
    #[must_use]
    pub fn finish(self) -> Vec<Primitive> {
        self.finish_output().primitives
    }
}
