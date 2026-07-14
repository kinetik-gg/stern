use stern_core::{
    Brush, ClipId, ComponentState, Key, KeyState, MouseButton, Point, Primitive, Rect,
    RectPrimitive, RepaintRequest, SemanticAction, SemanticActionKind, SemanticNode,
    TextInputEvent, TextPrimitive, TextRole, UiInput, UiInputEvent, WidgetId, pressable,
};

use super::Ui;
use crate::overlays::{
    OverlayKind, OverlayNavigationInput, OverlayScene, OverlaySceneIntent, OverlaySceneOutput,
    OverlaySceneRow, OverlaySceneRowKind, overlay_semantics,
};

impl Ui<'_> {
    /// Paints and evaluates one public overlay scene after its targets joined the frame plan.
    ///
    /// Call [`OverlayScene::declare_pointer_targets`] from the closure passed to
    /// [`Self::resolve_pointer_targets`] before evaluating lower UI and this scene. Action intents
    /// are also appended to the frame's application-owned action queue.
    #[allow(clippy::too_many_lines)]
    pub fn overlay_scene(&mut self, scene: &mut OverlayScene) -> OverlaySceneOutput {
        let mut output = OverlaySceneOutput::default();
        let keyboard_events = self.input().keyboard.events.clone();
        let text_events = self.input().text_events.clone();
        let outside_activation = primary_activation(self.input());
        let escape_pressed = keyboard_events
            .iter()
            .any(|event| event.state == KeyState::Pressed && matches!(event.key, Key::Escape));
        let now_millis = u64::try_from(self.time().now.as_millis()).unwrap_or(u64::MAX);

        if let Some(surface_index) = scene.top_keyboard_surface() {
            for event in &keyboard_events {
                if event.state != KeyState::Pressed || matches!(event.key, Key::Escape) {
                    continue;
                }
                if let Some(input) = navigation_input(&event.key) {
                    if input == OverlayNavigationInput::Activate && event.repeat {
                        continue;
                    }
                    let navigation = scene.navigate(surface_index, input);
                    if navigation.changed {
                        self.request_repaint(RepaintRequest::NextFrame);
                    }
                    if let Some(intent) = navigation.intent {
                        self.record_overlay_intent(&mut output, intent);
                        continue;
                    }
                    if input == OverlayNavigationInput::Activate {
                        let focused = self.memory().focused();
                        let focused_row = focused.and_then(|focused| {
                            scene
                                .rows(surface_index)
                                .into_iter()
                                .find(|row| row.id == focused && row.actionable())
                        });
                        if let Some(row) = focused_row
                            && let Some(intent) = scene.activate_row(surface_index, &row)
                        {
                            self.record_overlay_intent(&mut output, intent);
                            continue;
                        }
                    }
                    continue;
                }

                if event.modifiers.ctrl || event.modifiers.alt || event.modifiers.super_key {
                    continue;
                }
                let text = event.text.as_deref().or(match &event.key {
                    Key::Character(text) => Some(text.as_str()),
                    _ => None,
                });
                if text.is_some_and(|text| scene.typeahead(surface_index, text, now_millis)) {
                    self.request_repaint(RepaintRequest::NextFrame);
                }
            }

            for event in &text_events {
                if let TextInputEvent::Commit(text) = event
                    && scene.typeahead(surface_index, text, now_millis)
                {
                    self.request_repaint(RepaintRequest::NextFrame);
                }
            }
        }

        if let Some(request) = scene.dismissal_request(outside_activation, escape_pressed) {
            self.record_overlay_intent(&mut output, OverlaySceneIntent::Dismiss(request));
        }

        for surface_index in 0..scene.surfaces().len() {
            let entry = scene.surfaces()[surface_index].entry().clone();
            let label = scene.surfaces()[surface_index].label().to_owned();
            let rows = scene.rows(surface_index);
            self.paint_overlay_surface(&entry);

            let children = rows.iter().map(|row| row.id).collect::<Vec<_>>();
            self.push_semantic_node(overlay_semantics(&entry, label).with_children(children));

            let clip = ClipId::from_raw(
                WidgetId::from_raw(entry.id.raw())
                    .child("overlay-scene-clip")
                    .raw(),
            );
            self.primitive(Primitive::ClipBegin {
                id: clip,
                rect: entry.rect,
            });
            for row in rows {
                let response = if row.actionable() {
                    let id = self.register_id(row.id);
                    let (input, memory) = self.runtime.input_and_memory_mut();
                    let response = pressable(id, row.rect, input, memory, false);
                    if response.clicked || response.state.pressed {
                        self.request_repaint(RepaintRequest::NextFrame);
                    }
                    output.responses.push(response);
                    Some(response)
                } else {
                    None
                };

                self.paint_overlay_row(&row, response.as_ref());
                self.push_semantic_node(overlay_row_semantics(&row, response.as_ref()));

                if response.is_some_and(|response| response.clicked && !response.keyboard_activated)
                    && let Some(intent) = scene.activate_row(surface_index, &row)
                {
                    self.record_overlay_intent(&mut output, intent);
                }
            }
            self.primitive(Primitive::ClipEnd { id: clip });
        }

        output
    }

    fn record_overlay_intent(
        &mut self,
        output: &mut OverlaySceneOutput,
        intent: OverlaySceneIntent,
    ) {
        if let OverlaySceneIntent::Action(invocation) = &intent {
            self.push_action(invocation.clone());
        }
        self.request_repaint(RepaintRequest::NextFrame);
        output.intents.push(intent);
    }

    fn paint_overlay_surface(&mut self, entry: &crate::overlays::OverlayEntry) {
        if entry.kind == OverlayKind::Modal || entry.modal {
            let viewport = self.viewport().logical_size;
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: Rect::new(0.0, 0.0, viewport.width, viewport.height),
                fill: Some(Brush::Solid(
                    self.theme
                        .colors
                        .overlay
                        .scrim
                        .with_alpha(self.theme.opacity.overlay_scrim),
                )),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }
        if let Some(shadow) = self
            .theme
            .elevation_shadow(self.theme.elevation.overlay, self.theme.radii.md.top_left)
        {
            self.primitive(Primitive::Shadow(shadow.primitive(entry.rect)));
        }
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: entry.rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.overlay)),
            stroke: Some(stern_core::Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.default),
            )),
            radius: self.theme.radii.md,
        }));
    }

    fn paint_overlay_row(
        &mut self,
        row: &OverlaySceneRow,
        response: Option<&stern_core::Response>,
    ) {
        if row.kind == OverlaySceneRowKind::Separator {
            let height = self.theme.controls.separator_width.max(1.0);
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: Rect::new(
                    row.rect.x,
                    row.rect.y + (row.rect.height - height).max(0.0) * 0.5,
                    row.rect.width,
                    height,
                ),
                fill: Some(Brush::Solid(self.theme.colors.border.subtle)),
                stroke: None,
                radius: self.theme.radii.none,
            }));
            return;
        }

        let foreground = if row.kind == OverlaySceneRowKind::Action {
            let recipe = self.theme.row(ComponentState {
                hovered: response.is_some_and(|response| response.state.hovered),
                pressed: response.is_some_and(|response| response.state.pressed),
                focused: response.is_some_and(|response| response.state.focused),
                disabled: !row.enabled,
                selected: row.selected,
            });
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: row.rect,
                fill: Some(recipe.background),
                stroke: Some(recipe.border),
                radius: recipe.radius,
            }));
            recipe.foreground
        } else {
            self.theme.label(TextRole::Label, false).foreground
        };
        let font = self.theme.font(TextRole::Label);
        let extra = (row.rect.height - font.line_height).max(0.0) * 0.5;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                row.rect.x + self.theme.controls.padding_x,
                row.rect.y + extra + font.size,
            ),
            text: row.label.clone(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(foreground),
        }));
    }
}

fn overlay_row_semantics(
    row: &OverlaySceneRow,
    response: Option<&stern_core::Response>,
) -> SemanticNode {
    let mut node = SemanticNode::new(row.id, row.role.clone(), row.rect).with_label(&row.label);
    node.state.disabled = row.kind == OverlaySceneRowKind::Action && !row.enabled;
    node.state.selected = row.selected;
    node.state.checked = row.checked;
    node.state.expanded = row.expanded;
    node.state.focused = response.is_some_and(|response| response.state.focused);
    node.state.pressed = response.is_some_and(|response| response.state.pressed);
    if row.actionable() {
        node = node.focusable(true);
        let kind = if row.expanded.is_some() {
            SemanticActionKind::Open
        } else {
            SemanticActionKind::Invoke
        };
        node.actions.push(SemanticAction {
            kind,
            label: if row.expanded.is_some() {
                "Open submenu".to_owned()
            } else {
                "Invoke".to_owned()
            },
            action_id: row.action_id.clone(),
        });
    }
    node
}

fn navigation_input(key: &Key) -> Option<OverlayNavigationInput> {
    match key {
        Key::ArrowUp => Some(OverlayNavigationInput::Previous),
        Key::ArrowDown => Some(OverlayNavigationInput::Next),
        Key::Home => Some(OverlayNavigationInput::First),
        Key::End => Some(OverlayNavigationInput::Last),
        Key::Enter => Some(OverlayNavigationInput::Activate),
        _ => None,
    }
}

fn primary_activation(input: &UiInput) -> Option<Point> {
    if input.events.is_empty() {
        return input
            .pointer
            .primary
            .released
            .then_some(input.pointer.position)
            .flatten();
    }
    input.events.iter().rev().find_map(|event| match event {
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            position,
            ..
        } => (*position).or(input.pointer.position),
        _ => None,
    })
}
