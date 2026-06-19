//! Immediate-mode composition wrapper for widget primitives.

use std::hash::Hash;

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, ClipId,
    DropTargetResponse, FrameContext, FrameOutput, ImageId, Insets, PhysicalSize, Primitive, Rect,
    RepaintRequest, Response, ScaleFactor, ScrollResponse, Size, TextPrimitive, Theme, TimeInfo,
    Transform, Ui as CoreUi, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, context_menu_trigger,
    draggable, drop_target, focusable, pressable, scrollable, selectable, tooltip_trigger,
};
use kinetik_ui_text::{TextEditState, TextLayoutKey, TextLayoutStore, TextStyle};

use crate::{
    IconId, IconLibrary, MultiLineTextFieldOutput, NumericInputOutput, PanelFrame,
    SearchFieldOutput, TextFieldOutput, WidgetOutput, button as button_widget,
    checkbox as checkbox_widget, checkbox_with_label as checkbox_with_label_widget,
    icon_button as fallback_icon_button_widget,
    icon_button_with_label as fallback_icon_button_with_label_widget,
    icon_button_with_library as icon_button_with_library_widget, image as image_widget,
    image_icon_button as image_icon_button_widget, image_semantics, label as label_widget,
    label_semantics, list_row as list_row_widget,
    multi_line_text_field_with_text_layouts as multi_line_text_field_widget,
    numeric_input_with_text_layouts as numeric_input_widget, panel as panel_widget,
    panel_semantics, radio_button as radio_button_widget,
    radio_button_with_label as radio_button_with_label_widget,
    search_field_with_text_layouts as search_field_widget, separator as separator_widget,
    slider as slider_widget, slider_with_label as slider_with_label_widget,
    tab_button as tab_button_widget, text_field_with_text_layouts as text_field_widget,
    toggle as toggle_widget, toggle_with_label as toggle_with_label_widget,
};

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
            icons: None,
        }
    }

    /// Creates a widget façade with shaped text layout caching enabled.
    pub fn from_core_with_text_layouts(
        runtime: CoreUi<'a>,
        theme: &'a Theme,
        text_layouts: &'a mut TextLayoutStore,
    ) -> Self {
        Self {
            runtime,
            theme,
            text_layouts: Some(text_layouts),
            icons: None,
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
        self.text_layouts = Some(text_layouts);
        self
    }

    /// Enables vector icon resolution for this frame.
    #[must_use]
    pub const fn with_icons(mut self, icons: &'a IconLibrary) -> Self {
        self.icons = Some(icons);
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

    /// Emits a text label.
    pub fn label(&mut self, rect: Rect, text: impl Into<String>) {
        let text = text.into();
        let id = self.id(format!("label:{}:{}", rect_key("", rect), text));
        let output = label_widget(rect, text.clone(), self.theme);
        self.push_widget_output(&output);
        self.runtime
            .push_semantic_node(label_semantics(id, rect, text));
    }

    /// Emits a passive panel surface.
    pub fn panel(&mut self, rect: Rect) {
        let id = self.id(rect_key("panel", rect));
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
        let output = image_widget(rect, image);
        self.push_widget_output(&output);
        self.runtime.push_semantic_node(image_semantics(
            id,
            rect,
            format!("Image {}", image.raw()),
        ));
    }

    /// Resolves neutral press/click behavior without painting.
    pub fn pressable(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
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

    /// Resolves neutral draggable behavior without painting.
    pub fn draggable(&mut self, key: impl Hash, rect: Rect, disabled: bool) -> Response {
        let id = self.id(key);
        let (input, memory) = self.runtime.input_and_memory_mut();
        draggable(id, rect, input, memory, disabled)
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
            self.invoke_action(action.id.clone(), ActionSource::Button, context);
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

    /// Emits a toggle and returns its interaction response.
    pub fn toggle(&mut self, key: impl Hash, rect: Rect, on: bool, disabled: bool) -> Response {
        let id = self.id(key);
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = toggle_widget(id, rect, on, input, memory, theme, disabled);
        self.push_interactive(output)
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
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = slider_widget(id, rect, value, range, input, memory, theme, disabled);
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
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = slider_with_label_widget(
            id, rect, label, value, range, input, memory, theme, disabled,
        );
        self.push_interactive(output)
    }

    /// Emits a clipped, scrollable content region.
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

    /// Emits a single-line text field.
    pub fn text_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        state: &mut TextEditState,
        disabled: bool,
    ) -> TextFieldOutput {
        let id = self.id(key);
        let theme = self.theme;
        let text_layouts = self.text_layouts.as_deref_mut();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = text_field_widget(
            id,
            rect,
            state,
            input,
            memory,
            theme,
            disabled,
            text_layouts,
        );
        self.push_widget_output(&output.widget);
        output
    }

    /// Emits a multi-line text field.
    pub fn multi_line_text_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        state: &mut TextEditState,
        disabled: bool,
    ) -> MultiLineTextFieldOutput {
        let id = self.id(key);
        let theme = self.theme;
        let text_layouts = self.text_layouts.as_deref_mut();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = multi_line_text_field_widget(
            id,
            rect,
            state,
            input,
            memory,
            theme,
            disabled,
            text_layouts,
        );
        self.push_widget_output(&output.widget);
        output
    }

    /// Emits a numeric input field.
    pub fn numeric_input(
        &mut self,
        key: impl Hash,
        rect: Rect,
        state: &mut TextEditState,
        disabled: bool,
    ) -> NumericInputOutput {
        let id = self.id(key);
        let theme = self.theme;
        let text_layouts = self.text_layouts.as_deref_mut();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = numeric_input_widget(
            id,
            rect,
            state,
            input,
            memory,
            theme,
            disabled,
            text_layouts,
        );
        self.push_widget_output(&output.field.widget);
        output
    }

    /// Emits a search field.
    pub fn search_field(
        &mut self,
        key: impl Hash,
        rect: Rect,
        state: &mut TextEditState,
        disabled: bool,
    ) -> SearchFieldOutput {
        let id = self.id(key);
        let theme = self.theme;
        let text_layouts = self.text_layouts.as_deref_mut();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = search_field_widget(
            id,
            rect,
            state,
            input,
            memory,
            theme,
            disabled,
            text_layouts,
        );
        self.push_widget_output(&output.field.widget);
        output
    }

    fn push_widget_output(&mut self, output: &WidgetOutput) {
        self.extend(output.primitives.clone());
        for node in &output.semantics {
            self.runtime.push_semantic_node(node.clone());
        }
        for request in &output.platform_requests {
            self.runtime.push_platform_request(request.clone());
        }
    }

    fn push_interactive(&mut self, output: WidgetOutput) -> Response {
        let response = output.response.expect("interactive widget response");
        if response_requests_followup_repaint(response) {
            self.runtime.request_repaint(RepaintRequest::NextFrame);
        }
        self.extend(output.primitives);
        for node in output.semantics {
            self.runtime.push_semantic_node(node);
        }
        for request in output.platform_requests {
            self.runtime.push_platform_request(request);
        }
        response
    }

    fn attach_text_layout(&mut self, primitive: &mut Primitive) {
        let Some(text_layouts) = self.text_layouts.as_deref_mut() else {
            return;
        };
        let Primitive::Text(text) = primitive else {
            return;
        };
        if text.layout.is_some() {
            return;
        }

        text.layout = Some(text_layouts.layout_id(text_layout_key(text)));
    }
}

fn response_requests_followup_repaint(response: Response) -> bool {
    response.clicked
        || response.secondary_clicked
        || response.dragged
        || response.keyboard_activated
        || response.context_requested
}

fn text_layout_key(text: &TextPrimitive) -> TextLayoutKey {
    TextLayoutKey::new(
        text.text.clone(),
        TextStyle::new("sans-serif", text.size, text.size + 5.0),
        0.0,
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Ui;
    use crate::{IconGraphic, IconLibrary, IconPath};
    use kinetik_ui_core::{
        ActionContext, ActionDescriptor, ActionSource, CursorShape, FrameContext, FrameWarning,
        IconId, ImageId, Insets, PathElement, PhysicalSize, PlatformRequest, Point,
        PointerButtonState, PointerInput, Primitive, Rect, RepaintRequest, ScaleFactor,
        SemanticRole, Size, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
        default_dark_theme,
    };
    use kinetik_ui_text::{TextEditState, TextLayoutStore};

    fn pressed_at(x: f32, y: f32) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                primary: PointerButtonState::new(true, true, false),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    fn check_icon() -> IconGraphic {
        IconGraphic::new(
            Rect::new(0.0, 0.0, 24.0, 24.0),
            [IconPath::stroked(
                vec![
                    PathElement::MoveTo(Point::new(5.0, 12.0)),
                    PathElement::LineTo(Point::new(10.0, 17.0)),
                    PathElement::LineTo(Point::new(19.0, 7.0)),
                ],
                2.0,
            )],
        )
    }

    fn released_at(x: f32, y: f32) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                primary: PointerButtonState::new(false, false, true),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    fn scrolled_at(x: f32, y: f32, wheel_delta: Vec2) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                wheel_delta,
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    fn frame_context() -> FrameContext {
        FrameContext::new(
            ViewportInfo::new(
                Size::new(1280.0, 720.0),
                PhysicalSize::new(2560, 1440),
                ScaleFactor::new(2.0),
            ),
            UiInput::default(),
            TimeInfo::new(Duration::from_millis(32), Duration::from_millis(16), 2),
        )
    }

    #[test]
    fn ui_begin_frame_preserves_full_runtime_context() {
        let theme = default_dark_theme();
        let context = frame_context();
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(context.clone(), &mut memory, &theme);

        assert_eq!(ui.context(), &context);
        assert_eq!(ui.viewport(), context.viewport);
        assert_eq!(ui.time(), context.time);
        assert!(ui.output().primitives.is_empty());

        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
        assert_eq!(ui.output().primitives.len(), 1);
    }

    #[test]
    fn ui_begin_frame_with_text_layouts_attaches_layouts() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let mut text_layouts = TextLayoutStore::new();
        let mut ui = Ui::begin_frame_with_text_layouts(
            frame_context(),
            &mut memory,
            &theme,
            &mut text_layouts,
        );

        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
        let output = ui.finish_output();

        assert!(matches!(
            output.primitives.first(),
            Some(Primitive::Text(text)) if text.layout.is_some()
        ));
        assert_eq!(text_layouts.len(), 1);
    }

    #[test]
    fn ui_collects_widget_primitives() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
        ui.panel(Rect::new(0.0, 24.0, 120.0, 48.0));
        let primitives = ui.finish();

        assert!(
            primitives
                .iter()
                .any(|item| matches!(item, Primitive::Text(_)))
        );
        assert!(
            primitives
                .iter()
                .any(|item| matches!(item, Primitive::Rect(_)))
        );
    }

    #[test]
    fn ui_attaches_shaped_text_layouts_when_store_is_available() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut text_layouts = TextLayoutStore::new();
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut text_layouts);

        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
        let output = ui.finish_output();

        let text = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) => Some(text),
                _ => None,
            })
            .expect("label emits text");
        let layout = text.layout.expect("text layout is attached");
        assert!(text_layouts.layout(layout).is_some());
        assert_eq!(text_layouts.len(), 1);
    }

    #[test]
    fn ui_text_field_uses_shaped_text_layout_store_for_caret_geometry() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("Wide text");
        let mut text_layouts = TextLayoutStore::new();
        memory.focus(WidgetId::from_key("root").child("field"));
        state.set_caret(4);
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut text_layouts);

        let output = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 28.0), &mut state, false);
        assert!(!output.changed);
        let frame = ui.finish_output();

        assert!(!text_layouts.is_empty());
        assert!(
            frame.primitives.iter().any(
                |primitive| matches!(primitive, Primitive::Text(text) if text.layout.is_some())
            )
        );
        assert!(frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::Rect(rect)
                if (rect.rect.width - 1.0).abs() < f32::EPSILON
                    && rect.rect.height > theme.text_size
        )));
    }

    #[test]
    fn ui_scroll_area_clips_translates_content_and_stores_offset() {
        let theme = default_dark_theme();
        let input = scrolled_at(8.0, 8.0, Vec2::new(0.0, -24.0));
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let output = ui.scroll_area(
            "area",
            Rect::new(0.0, 0.0, 100.0, 80.0),
            Size::new(100.0, 200.0),
            false,
            |ui, offset| {
                ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Inside");
                offset
            },
        );
        assert_eq!(output.inner, Vec2::new(0.0, 24.0));
        assert_eq!(output.scroll.offset, Vec2::new(0.0, 24.0));

        let frame = ui.finish_output();
        assert_eq!(
            memory.scroll_offset(output.scroll.response.id),
            Vec2::new(0.0, 24.0)
        );
        assert!(matches!(frame.primitives[0], Primitive::ClipBegin { .. }));
        assert!(matches!(
            frame.primitives[1],
            Primitive::TransformBegin(transform)
                if transform.dx.abs() < f32::EPSILON && (transform.dy + 24.0).abs() < f32::EPSILON
        ));
        assert!(
            frame
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Text(_)))
        );
        assert!(matches!(
            frame.primitives[frame.primitives.len() - 2],
            Primitive::TransformEnd
        ));
        assert!(matches!(
            frame.primitives[frame.primitives.len() - 1],
            Primitive::ClipEnd { .. }
        ));
    }

    #[test]
    fn ui_panel_body_emits_balanced_body_clip() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let body = ui.panel_body(
            "inspector",
            Rect::new(10.0, 20.0, 120.0, 80.0),
            Insets::new(8.0, 10.0, 12.0, 14.0),
            |ui, body| {
                ui.label(body, "Inside");
                body
            },
        );
        let frame = ui.finish_output();

        assert_eq!(body, Rect::new(18.0, 32.0, 102.0, 54.0));
        assert!(
            frame
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Rect(_)))
        );
        assert!(frame.primitives.iter().any(|primitive| matches!(
            primitive,
            Primitive::ClipBegin { rect, .. } if *rect == body
        )));
        assert!(matches!(
            frame.primitives.last(),
            Some(Primitive::ClipEnd { .. })
        ));
        assert!(frame.warnings.is_empty());
    }

    #[test]
    fn ui_routes_button_interaction_through_memory() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let input = pressed_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let response = ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);

        assert!(response.state.hovered);
        assert!(response.state.pressed);
    }

    #[test]
    fn ui_action_button_queues_action_invocation_on_click() {
        let theme = default_dark_theme();
        let action = ActionDescriptor::new("run", "Run");
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut memory = UiMemory::new();

        let input = pressed_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let pressed = ui
            .action_button("run", rect, &action, ActionContext::Global)
            .expect("visible action");
        assert!(pressed.state.pressed);
        assert!(ui.finish_output().actions.is_empty());

        let input = released_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let clicked = ui
            .action_button("run", rect, &action, ActionContext::Global)
            .expect("visible action");
        let mut output = ui.finish_output();

        assert!(clicked.clicked);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
        assert_eq!(output.actions.len(), 1);
        let invocation = output.actions.pop_front().expect("queued action");
        assert_eq!(invocation.action_id, action.id);
        assert_eq!(invocation.source, ActionSource::Button);
        assert_eq!(invocation.context, ActionContext::Global);
    }

    #[test]
    fn ui_action_button_respects_hidden_and_disabled_action_state() {
        let theme = default_dark_theme();
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut hidden = ActionDescriptor::new("hidden", "Hidden");
        hidden.state.visible = false;
        let mut disabled = ActionDescriptor::new("disabled", "Disabled");
        disabled.state.enabled = false;

        let mut memory = UiMemory::new();
        let input = released_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);

        assert!(
            ui.action_button("hidden", rect, &hidden, ActionContext::Global)
                .is_none()
        );
        let response = ui
            .action_button("disabled", rect, &disabled, ActionContext::Global)
            .expect("disabled action is visible");
        let output = ui.finish_output();

        assert!(response.state.disabled);
        assert!(output.actions.is_empty());
        assert!(!output.primitives.is_empty());
    }

    #[test]
    fn ui_can_invoke_action_descriptors_without_a_button_surface() {
        let theme = default_dark_theme();
        let mut action = ActionDescriptor::new("export", "Export");
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        assert!(ui.invoke_action_descriptor(&action, ActionSource::Menu, ActionContext::Global));
        action.state.enabled = false;
        assert!(!ui.invoke_action_descriptor(&action, ActionSource::Menu, ActionContext::Global));

        let output = ui.finish_output();
        assert_eq!(output.actions.len(), 1);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
    }

    #[test]
    fn ui_interactive_click_requests_followup_repaint() {
        let theme = default_dark_theme();
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut memory = UiMemory::new();

        let input = pressed_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.button("run", rect, "Run", false);
        assert_eq!(ui.finish_output().repaint, RepaintRequest::None);

        let input = released_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let response = ui.button("run", rect, "Run", false);
        let output = ui.finish_output();

        assert!(response.clicked);
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
    }

    #[test]
    fn ui_exposes_neutral_behavior_primitives() {
        let theme = default_dark_theme();
        let input = pressed_at(4.0, 4.0);

        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let pressed = ui.pressable("pressable", Rect::new(0.0, 0.0, 20.0, 20.0), false);
        assert!(pressed.state.pressed);
        assert!(ui.finish_output().primitives.is_empty());

        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let selected = ui.selectable("selectable", Rect::new(0.0, 0.0, 20.0, 20.0), true, false);
        assert!(selected.state.selected);
        assert!(ui.finish_output().primitives.is_empty());

        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        let dragged = ui.draggable("draggable", Rect::new(0.0, 0.0, 20.0, 20.0), false);
        assert!(dragged.state.active);
        assert!(ui.finish_output().primitives.is_empty());
    }

    #[test]
    fn ui_exposes_overlay_and_drop_behavior_primitives() {
        let theme = default_dark_theme();
        let source = WidgetId::from_key("source");
        let mut memory = UiMemory::new();
        memory.start_drag(source);
        memory.press_secondary(WidgetId::from_key("root").child("context"));
        let mut input = released_at(4.0, 4.0);
        input.pointer.secondary = PointerButtonState::new(false, false, true);
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let context = ui.context_menu_trigger("context", Rect::new(0.0, 0.0, 20.0, 20.0), false);
        let tooltip = ui.tooltip_trigger("tooltip", Rect::new(0.0, 0.0, 20.0, 20.0), false);
        let drop = ui.drop_target("drop", Rect::new(0.0, 0.0, 20.0, 20.0), false);

        assert!(context.context_requested);
        assert!(tooltip.tooltip_requested);
        assert_eq!(drop.source, Some(source));
        assert!(drop.dropped);
    }

    #[test]
    fn ui_finish_output_preserves_core_runtime_warnings() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let id = ui.id("duplicate");
        ui.id("duplicate");
        let output = ui.finish_output();

        assert_eq!(
            output.warnings,
            vec![FrameWarning::DuplicateWidgetId { id }]
        );
    }

    #[test]
    fn ui_finish_output_preserves_widget_semantics() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abc");
        let mut ui = Ui::new(&input, &mut memory, &theme);

        ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
        ui.button("run", Rect::new(0.0, 24.0, 80.0, 28.0), "Run", false);
        ui.text_field(
            "field",
            Rect::new(0.0, 60.0, 120.0, 24.0),
            &mut state,
            false,
        );
        let output = ui.finish_output();

        let roles = output
            .semantics
            .nodes()
            .iter()
            .map(|node| node.role.clone())
            .collect::<Vec<_>>();
        assert!(roles.contains(&SemanticRole::Label));
        assert!(roles.contains(&SemanticRole::Button));
        assert!(roles.contains(&SemanticRole::TextField));
        assert_eq!(output.semantics.focus_order().len(), 2);
    }

    #[test]
    fn ui_labeled_controls_preserve_accessible_names() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut value = 0.4;
        let mut ui = Ui::new(&input, &mut memory, &theme);

        ui.checkbox_with_label(
            "snap",
            Rect::new(0.0, 0.0, 20.0, 20.0),
            "Enable snapping",
            true,
            false,
        );
        ui.radio_button_with_label(
            "blend",
            Rect::new(0.0, 24.0, 20.0, 20.0),
            "Blend mode",
            true,
            false,
        );
        ui.toggle_with_label(
            "loop",
            Rect::new(0.0, 48.0, 36.0, 18.0),
            "Loop playback",
            true,
            false,
        );
        ui.slider_with_label(
            "opacity",
            Rect::new(0.0, 72.0, 100.0, 12.0),
            "Brush opacity",
            &mut value,
            0.0..=1.0,
            false,
        );
        ui.icon_button_with_label(
            "save",
            Rect::new(0.0, 96.0, 24.0, 24.0),
            IconId::from_raw(1),
            "Save project",
            false,
        );
        let output = ui.finish_output();

        let labels = output
            .semantics
            .nodes()
            .iter()
            .filter_map(|node| node.label.as_deref())
            .collect::<Vec<_>>();
        assert!(labels.contains(&"Enable snapping"));
        assert!(labels.contains(&"Blend mode"));
        assert!(labels.contains(&"Loop playback"));
        assert!(labels.contains(&"Brush opacity"));
        assert!(labels.contains(&"Save project"));
    }

    #[test]
    fn ui_icon_buttons_use_registered_vector_icons() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut icons = IconLibrary::new();
        let icon = IconId::from_raw(7);
        icons.register(icon, check_icon());
        let mut ui = Ui::new(&input, &mut memory, &theme).with_icons(&icons);

        ui.icon_button_with_label(
            "check",
            Rect::new(0.0, 0.0, 24.0, 24.0),
            icon,
            "Apply",
            false,
        );
        let output = ui.finish_output();

        assert_eq!(output.primitives.len(), 2);
        assert!(matches!(output.primitives[1], Primitive::Path(_)));
        assert!(output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Apply")
        }));
    }

    #[test]
    fn ui_image_icon_button_uses_bitmap_icon_widget() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut memory, &theme);

        ui.image_icon_button(
            "save",
            Rect::new(0.0, 0.0, 24.0, 24.0),
            ImageId::from_raw(7),
            "Save project",
            false,
        );
        let output = ui.finish_output();

        assert!(
            output
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::Image(_)))
        );
        assert!(output.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::IconButton && node.label.as_deref() == Some("Save project")
        }));
    }

    #[test]
    fn ui_forwards_widget_platform_requests() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let input = pressed_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);

        ui.button("run", Rect::new(0.0, 0.0, 80.0, 28.0), "Run", false);
        let output = ui.finish_output();

        assert!(
            output
                .platform_requests
                .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
        );
    }

    #[test]
    fn ui_text_field_requests_platform_text_input_when_focused() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abc");

        let input = pressed_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
        let _ = ui.finish_output();

        let input = released_at(4.0, 4.0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);
        let output = ui.finish_output();

        assert!(output.platform_requests.iter().any(|request| {
            matches!(
                request,
                PlatformRequest::StartTextInput {
                    rect: Some(rect),
                } if *rect == Rect::new(0.0, 0.0, 120.0, 24.0)
            )
        }));
    }

    #[test]
    fn ui_text_fields_use_public_text_widget() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abc");
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let output = ui.text_field("field", Rect::new(0.0, 0.0, 120.0, 24.0), &mut state, false);

        assert!(!output.changed);
        assert!(!ui.finish().is_empty());
    }

    #[test]
    fn ui_multi_line_text_field_uses_public_widget() {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("one\ntwo");
        let mut ui = Ui::new(&input, &mut memory, &theme);

        let output =
            ui.multi_line_text_field("field", Rect::new(0.0, 0.0, 160.0, 80.0), &mut state, false);

        assert_eq!(output.visible_lines, 2);
        assert!(!ui.finish().is_empty());
    }
}
