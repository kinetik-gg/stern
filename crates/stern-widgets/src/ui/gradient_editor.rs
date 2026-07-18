use stern_core::{
    Brush, DomainDragGesturePhase, Key, KeyState, Point, Primitive, Rect, RectPrimitive,
    RepaintRequest, SemanticNode, SemanticRole, SemanticValue, Stroke,
};

use super::Ui;
use crate::gradient_editor::{
    GradientEditorConfig, GradientEditorIntent, GradientEditorOutput, GradientEditorPrepareError,
    GradientEditorStop, GradientEditorStopId, GradientEditorWidget, marker_rect, ramp_rect,
    stop_widget_id,
};

#[allow(missing_docs, clippy::missing_errors_doc)]
impl Ui<'_> {
    pub fn prepare_gradient_editor<'a>(
        &self,
        config: GradientEditorConfig<'a>,
    ) -> Result<GradientEditorWidget<'a>, GradientEditorPrepareError> {
        GradientEditorWidget::prepare(config)
    }

    pub fn gradient_editor(&mut self, widget: &GradientEditorWidget) -> GradientEditorOutput {
        let root = self.register_id(widget.widget_id());
        let inert = widget.disabled() || widget.read_only();
        let mut gesture =
            self.runtime
                .captured_domain_drag_gesture(root, ramp_rect(widget.config.bounds), inert);
        let mut intents = Vec::new();
        let mut active = widget.selected_stop();

        for action in &gesture.actions {
            match action.phase {
                DomainDragGesturePhase::Press => {
                    self.runtime.memory_mut().focus(root);
                    if let Some(position) = action.position
                        && let Some(stop) = hit_stop(widget, position)
                    {
                        active = Some(stop.id);
                        if widget.selected_stop() != active {
                            intents.push(GradientEditorIntent::SelectStop(stop.id));
                        }
                    }
                }
                DomainDragGesturePhase::Move => {
                    push_move(widget, active, action.position, &mut intents);
                }
                DomainDragGesturePhase::Release if !action.release_clicked => {
                    push_move(widget, active, action.position, &mut intents);
                }
                DomainDragGesturePhase::Release | DomainDragGesturePhase::Cancel => {}
            }
        }

        gesture.response.rect = widget.bounds();
        gesture.response.state.focused = self.memory().is_focused(root);
        gesture.response.state.selected = widget.selected_stop().is_some();
        gesture.response.state.disabled = widget.disabled();
        self.handle_gradient_keyboard(widget, &mut intents);

        let bounds = widget.bounds();
        let reverse_id = self.id(("gradient-reverse", root.raw()));
        let theme = *self.theme();
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = crate::button(
            reverse_id,
            Rect::new(bounds.max_x() - 120.0, bounds.y + 4.0, 112.0, 20.0),
            format!("{} · Reverse", widget.space().label()),
            input,
            memory,
            &theme,
            inert,
        );
        let reverse = output.response.unwrap_or(gesture.response);
        self.extend(output.primitives);
        for node in output.semantics {
            self.push_semantic_node(node);
        }
        self.push_widget_platform_requests(Some(reverse), output.platform_requests);
        if reverse.clicked || reverse.keyboard_activated {
            intents.push(GradientEditorIntent::Reverse);
        }
        self.paint_gradient_editor(widget);
        self.push_gradient_semantics(widget);
        if !intents.is_empty() {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        let response = gesture.response;
        GradientEditorOutput { response, intents }
    }

    fn handle_gradient_keyboard(
        &self,
        widget: &GradientEditorWidget,
        intents: &mut Vec<GradientEditorIntent>,
    ) {
        if widget.disabled() || widget.read_only() || !self.memory().is_focused(widget.widget_id())
        {
            return;
        }
        let Some(selected) = widget.selected_stop() else {
            return;
        };
        let Some(stop) = widget.stops().iter().find(|stop| stop.id == selected) else {
            return;
        };
        for event in &self.input().keyboard.events {
            if event.state != KeyState::Pressed || event.modifiers.alt || event.modifiers.super_key
            {
                continue;
            }
            match event.key {
                Key::ArrowLeft | Key::ArrowRight => {
                    let direction = if event.key == Key::ArrowLeft {
                        -1.0
                    } else {
                        1.0
                    };
                    intents.push(GradientEditorIntent::MoveStop {
                        id: selected,
                        position: (stop.position + direction * widget.config.keyboard_step)
                            .clamp(0.0, 1.0),
                    });
                }
                Key::Delete if !event.repeat && stop.removable && widget.stops().len() > 2 => {
                    intents.push(GradientEditorIntent::RemoveStop(selected));
                }
                _ => {}
            }
        }
    }

    fn paint_gradient_editor(&mut self, widget: &GradientEditorWidget) {
        let theme = *self.theme();
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: ramp_rect(widget.config.bounds),
            fill: Some(Brush::LinearGradient(widget.gradient)),
            stroke: Some(Stroke::new(
                theme.strokes.default,
                Brush::Solid(theme.colors.border.strong),
            )),
            radius: theme.radii.sm,
        }));
        for stop in widget.stops() {
            let selected = widget.selected_stop() == Some(stop.id);
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: marker_rect(widget.config.bounds, *stop),
                fill: Some(Brush::Solid(stop.color)),
                stroke: Some(Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    Brush::Solid(theme.colors.border.strong),
                )),
                radius: theme.radii.sm,
            }));
        }
    }

    fn push_gradient_semantics(&mut self, widget: &GradientEditorWidget) {
        let mut root = SemanticNode::new(
            widget.widget_id(),
            SemanticRole::Custom("gradient-editor".to_owned()),
            widget.bounds(),
        )
        .with_label("Gradient editor")
        .focusable(!widget.disabled());
        root.state.focused = self.memory().is_focused(widget.widget_id());
        root.state.disabled = widget.disabled();
        root.state.selected = widget.selected_stop().is_some();
        self.push_semantic_node(root);
        for stop in widget.stops() {
            let mut node = SemanticNode::new(
                stop_widget_id(widget.config.id, stop.id),
                SemanticRole::Custom("gradient-stop".to_owned()),
                marker_rect(widget.config.bounds, *stop),
            )
            .with_label(format!("Gradient stop {}", stop.id.raw()));
            node.state.disabled = widget.disabled();
            node.state.selected = widget.selected_stop() == Some(stop.id);
            node.state.value = Some(SemanticValue::Number {
                current: stop.position,
                min: 0.0,
                max: 1.0,
            });
            self.push_semantic_node(node);
        }
    }
}

fn hit_stop(widget: &GradientEditorWidget, position: Point) -> Option<GradientEditorStop> {
    widget
        .stops()
        .iter()
        .rev()
        .copied()
        .find(|stop| marker_rect(widget.config.bounds, *stop).contains_point(position))
}

fn push_move(
    widget: &GradientEditorWidget,
    active: Option<GradientEditorStopId>,
    position: Option<Point>,
    intents: &mut Vec<GradientEditorIntent>,
) {
    let (Some(id), Some(position)) = (active, position) else {
        return;
    };
    let ramp = ramp_rect(widget.config.bounds);
    intents.push(GradientEditorIntent::MoveStop {
        id,
        position: ((position.x - ramp.x) / ramp.width).clamp(0.0, 1.0),
    });
}
