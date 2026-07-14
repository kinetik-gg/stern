use stern_core::{
    Brush, ClipId, CursorShape, DomainDragGesturePhase, InputWheelDelta, Point, PointerRoute,
    Primitive, Rect, RectPrimitive, RepaintRequest, SemanticNode, SemanticRole, SemanticValue,
    Stroke, UiInput, UiInputEvent, UiMemory, Vec2, WidgetId,
};

use super::Ui;
use crate::viewport::{
    PanZoom, ViewportActionKind, ViewportActionRequest, ViewportFit, ViewportSurface,
    ViewportWidget, ViewportWidgetConfig, ViewportWidgetOutput, viewport_action_widget_id,
};

const PIXELS_PER_ZOOM_UNIT: f32 = 100.0;

impl Ui<'_> {
    /// Prepares one immutable viewport snapshot using this frame's scale factor.
    #[must_use]
    pub fn prepare_viewport_widget(&self, config: ViewportWidgetConfig) -> ViewportWidget {
        ViewportWidget::new(config, self.viewport().scale_factor)
    }

    /// Evaluates and paints one supported viewport component.
    ///
    /// Paint, hit testing, and coordinate conversions use the prepared frozen
    /// surface. Accepted pan/zoom changes update `pan_zoom` for the next frame.
    #[allow(clippy::too_many_lines)]
    pub fn viewport_widget(
        &mut self,
        widget: &ViewportWidget,
        pan_zoom: &mut PanZoom,
        action_requests: &[ViewportActionRequest],
    ) -> ViewportWidgetOutput {
        let config = widget.config();
        let surface = widget.surface();
        let bounds = surface.effective_bounds();
        let valid_bounds = bounds.width > 0.0 && bounds.height > 0.0;
        let disabled = config.disabled || !valid_bounds;
        let old_focus = self.memory().focused();
        let original = surface.pan_zoom;
        let mut next = original;
        let mut response =
            self.runtime
                .captured_domain_drag_gesture(widget.widget_id(), bounds, disabled);
        let pointer_pressed = response
            .actions
            .iter()
            .any(|action| matches!(action.phase, DomainDragGesturePhase::Press));
        let gesture_ended = response.actions.iter().any(|action| {
            matches!(
                action.phase,
                DomainDragGesturePhase::Release | DomainDragGesturePhase::Cancel
            )
        });

        if pointer_pressed || response.response.clicked {
            self.runtime.memory_mut().focus(widget.widget_id());
        }
        response.response.state.focused = self.memory().is_focused(widget.widget_id());

        let drag_has_movement = response.response.dragged
            || self.memory().is_drag_source(widget.widget_id())
            || self.memory().released_drag_source() == Some(widget.widget_id());
        if drag_has_movement {
            for action in &response.actions {
                if matches!(action.phase, DomainDragGesturePhase::Move) {
                    next.pan_by(action.delta);
                }
            }
        }

        let pointer = self
            .input()
            .pointer
            .position
            .filter(|point| surface.contains_screen_point(*point));
        let content_pointer = pointer.and_then(|point| widget.screen_to_content(point));
        let wheel_units = viewport_wheel_units(
            self.input(),
            self.memory(),
            widget.widget_id(),
            bounds,
            disabled,
        );
        if wheel_units != 0.0 {
            next = zoom_around(
                surface,
                next,
                pointer.unwrap_or_else(|| rect_center(bounds)),
                (wheel_units * config.zoom_step).exp(),
                config.min_zoom,
                config.max_zoom,
                widget.scale_factor(),
            );
        }

        let mut unhandled_actions = Vec::new();
        for request in action_requests
            .iter()
            .filter(|request| request.target.viewport == widget.widget_id())
        {
            let handled = match request.kind {
                ViewportActionKind::FitContent if !disabled => {
                    next.fit();
                    true
                }
                ViewportActionKind::ActualSize if !disabled => {
                    next.actual_size();
                    true
                }
                ViewportActionKind::ZoomIn if !disabled => {
                    next = zoom_around(
                        surface,
                        next,
                        pointer.unwrap_or_else(|| rect_center(bounds)),
                        (config.zoom_step).exp(),
                        config.min_zoom,
                        config.max_zoom,
                        widget.scale_factor(),
                    );
                    true
                }
                ViewportActionKind::ZoomOut if !disabled => {
                    next = zoom_around(
                        surface,
                        next,
                        pointer.unwrap_or_else(|| rect_center(bounds)),
                        (-config.zoom_step).exp(),
                        config.min_zoom,
                        config.max_zoom,
                        widget.scale_factor(),
                    );
                    true
                }
                _ => false,
            };
            if !handled {
                unhandled_actions.push(request.clone());
            }
        }

        let pan_changed = vec_changed(original.pan, next.pan);
        let zoom_changed = original.zoom.to_bits() != next.zoom.to_bits();
        let fit_changed = original.fit != next.fit;
        *pan_zoom = next;

        if !disabled {
            let cursor = if self.memory().is_drag_source(widget.widget_id()) {
                CursorShape::Grabbing
            } else {
                CursorShape::Grab
            };
            self.runtime.request_cursor_for(widget.widget_id(), cursor);
        }

        self.register_id(widget.widget_id());
        if valid_bounds {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: bounds,
                fill: Some(Brush::Solid(self.theme.colors.surface.workspace)),
                stroke: Some(Stroke::new(
                    self.theme.controls.border_width,
                    Brush::Solid(self.theme.colors.border.subtle),
                )),
                radius: self.theme.radii.none,
            }));
            let clip = ClipId::from_raw(widget.widget_id().child("viewport-clip").raw());
            self.primitive(Primitive::ClipBegin {
                id: clip,
                rect: bounds,
            });
            self.primitive(surface.texture_primitive_at(widget.scale_factor()));
            self.primitive(Primitive::ClipEnd { id: clip });
        }

        self.push_viewport_semantics(widget, response.response.state.focused, disabled);

        let focus_changed = old_focus != self.memory().focused();
        if pan_changed
            || zoom_changed
            || fit_changed
            || focus_changed
            || response.response.clicked
            || response.response.dragged
            || response.response.state.pressed
            || gesture_ended
        {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        ViewportWidgetOutput {
            response: response.response,
            surface,
            next_pan_zoom: next,
            content_pointer,
            pan_changed,
            zoom_changed,
            fit_changed,
            action_requests: unhandled_actions,
        }
    }

    fn push_viewport_semantics(&mut self, widget: &ViewportWidget, focused: bool, disabled: bool) {
        let config = widget.config();
        let bounds = widget.surface().effective_bounds();
        let actions = config
            .actions
            .iter()
            .filter(|action| action.visible() && action.target.viewport == widget.widget_id());
        let children = actions
            .clone()
            .map(|action| viewport_action_widget_id(widget.widget_id(), action.action_id()))
            .collect::<Vec<_>>();
        let mut root = SemanticNode::new(widget.widget_id(), SemanticRole::Viewport, bounds)
            .with_label(&config.label)
            .with_children(children)
            .focusable(!disabled);
        root.state.focused = focused;
        root.state.disabled = disabled;
        root.state.value = Some(SemanticValue::Text(format!(
            "{:?}, zoom {:.2}",
            widget.surface().pan_zoom.fit,
            display_zoom(widget.surface(), widget.scale_factor())
        )));
        self.push_semantic_node(root);

        for action in actions {
            if let Some(mut node) = action.semantics(widget.widget_id(), bounds) {
                if disabled {
                    node.state.disabled = true;
                    node.focusable = false;
                    node.actions.clear();
                }
                self.push_semantic_node(node);
            }
        }
    }
}

fn viewport_wheel_units(
    input: &UiInput,
    memory: &UiMemory,
    id: WidgetId,
    bounds: Rect,
    disabled: bool,
) -> f32 {
    if disabled
        || input
            .pointer
            .position
            .is_none_or(|point| !bounds.contains_point(point))
    {
        return 0.0;
    }
    let routed = match memory.pointer_wheel_route() {
        PointerRoute::Target(owner) => owner == id,
        PointerRoute::Unplanned => input.events.is_empty(),
        PointerRoute::Blocked => false,
    };
    if !routed {
        return 0.0;
    }

    let units = if input.events.is_empty() {
        input.pointer.wheel_delta.y / PIXELS_PER_ZOOM_UNIT
    } else {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                UiInputEvent::Wheel { delta, .. } => Some(match *delta {
                    InputWheelDelta::Lines(delta) => delta.y,
                    InputWheelDelta::Pixels(delta) => delta.y / PIXELS_PER_ZOOM_UNIT,
                }),
                _ => None,
            })
            .sum()
    };
    if units.is_finite() { units } else { 0.0 }
}

fn zoom_around(
    frozen_surface: ViewportSurface,
    current: PanZoom,
    anchor: Point,
    factor: f32,
    min_zoom: f32,
    max_zoom: f32,
    scale_factor: stern_core::ScaleFactor,
) -> PanZoom {
    if !factor.is_finite() || factor <= 0.0 {
        return current;
    }
    let mut current_surface = frozen_surface;
    current_surface.pan_zoom = current;
    let current_zoom = display_zoom(current_surface, scale_factor);
    let desired = (current_zoom * factor).clamp(min_zoom, max_zoom);
    let Some(content_anchor) = frozen_surface.screen_to_content_at(anchor, scale_factor) else {
        return current;
    };

    let mut next = current;
    next.set_zoom(desired);
    let mut next_surface = frozen_surface;
    next_surface.pan_zoom = next;
    if let Some(projected) = next_surface.content_to_screen_at(content_anchor, scale_factor) {
        next.pan_by(Vec2::new(anchor.x - projected.x, anchor.y - projected.y));
    }
    next
}

fn display_zoom(surface: ViewportSurface, scale_factor: stern_core::ScaleFactor) -> f32 {
    if matches!(surface.pan_zoom.fit, ViewportFit::Zoom) {
        return surface.pan_zoom.zoom;
    }
    let mut actual = surface;
    actual.pan_zoom.actual_size();
    let native = actual.content_scale_at(scale_factor);
    if native > 0.0 {
        surface.content_scale_at(scale_factor) / native
    } else {
        1.0
    }
}

fn rect_center(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn vec_changed(before: Vec2, after: Vec2) -> bool {
    before.x.to_bits() != after.x.to_bits() || before.y.to_bits() != after.y.to_bits()
}
