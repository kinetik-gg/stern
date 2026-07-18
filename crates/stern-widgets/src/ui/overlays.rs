use stern_core::{
    Brush, ClipId, Color, ComponentState, ElevationLevel, FontToken, Key, KeyState, LinePrimitive,
    MouseButton, Point, Primitive, Rect, RectPrimitive, RepaintRequest, SemanticAction,
    SemanticActionKind, SemanticNode, ShortcutLabelLocalizer, ShortcutPlatform, Stroke,
    TextInputEvent, TextPrimitive, TextRole, UiInput, UiInputEvent, WidgetId, pressable,
};

use super::Ui;
use crate::overlays::{
    OverlayKind, OverlayNavigationInput, OverlayScene, OverlaySceneIntent, OverlaySceneOutput,
    OverlaySceneRow, OverlaySceneRowKind, overlay_semantics,
};

#[derive(Debug, Clone, Copy, PartialEq)]
struct MenuColumnGeometry {
    state: Rect,
    icon: Rect,
    label: Rect,
    status: Rect,
    shortcut: Rect,
    disclosure: Rect,
}

#[derive(Clone, Copy)]
struct MenuPresentation<'a> {
    platform: ShortcutPlatform,
    localizer: &'a dyn ShortcutLabelLocalizer,
}

fn menu_column_geometry(row: Rect) -> Option<MenuColumnGeometry> {
    if !row.width.is_finite() || row.width < 272.0 {
        return None;
    }

    let state = Rect::new(row.x + 8.0, row.y, 16.0, row.height);
    let icon = Rect::new(state.max_x() + 8.0, row.y, 16.0, row.height);
    let label = Rect::new(icon.max_x() + 8.0, row.y, row.width - 232.0, row.height);
    let status = Rect::new(label.max_x() + 8.0, row.y, 16.0, row.height);
    let shortcut = Rect::new(status.max_x() + 8.0, row.y, 112.0, row.height);
    let disclosure = Rect::new(shortcut.max_x() + 8.0, row.y, 16.0, row.height);

    Some(MenuColumnGeometry {
        state,
        icon,
        label,
        status,
        shortcut,
        disclosure,
    })
}

impl Ui<'_> {
    /// Paints and evaluates one public overlay scene after its targets joined the frame plan.
    ///
    /// Call [`OverlayScene::declare_pointer_targets`] from the closure passed to
    /// [`Self::resolve_pointer_targets`] before evaluating lower UI and this scene. Action intents
    /// are also appended to the frame's application-owned action queue.
    pub fn overlay_scene(&mut self, scene: &mut OverlayScene) -> OverlaySceneOutput {
        self.overlay_scene_impl(scene, None)
    }

    /// Paints a scene with an explicit one-evaluation menu shortcut presentation policy.
    ///
    /// Wide menu rows use [`stern_core::Shortcut::localized_label`] with the supplied platform
    /// and caller-owned localizer. Other surfaces and narrow menu rows retain legacy painting.
    pub fn overlay_scene_with_menu_presentation(
        &mut self,
        scene: &mut OverlayScene,
        platform: ShortcutPlatform,
        localizer: &dyn ShortcutLabelLocalizer,
    ) -> OverlaySceneOutput {
        self.overlay_scene_impl(
            scene,
            Some(MenuPresentation {
                platform,
                localizer,
            }),
        )
    }

    #[allow(clippy::too_many_lines)]
    fn overlay_scene_impl(
        &mut self,
        scene: &mut OverlayScene,
        menu_presentation: Option<MenuPresentation<'_>>,
    ) -> OverlaySceneOutput {
        let mut output = OverlaySceneOutput::default();
        let keyboard_events = self.input().keyboard.events.clone();
        let text_events = self.input().text_events.clone();
        let outside_activation = primary_activation(self.input());
        let escape_pressed = keyboard_events
            .iter()
            .any(|event| event.state == KeyState::Pressed && matches!(event.key, Key::Escape));
        let now_millis = u64::try_from(self.time().now.as_millis()).unwrap_or(u64::MAX);

        let mut escape_consumed = false;
        if let Some(surface_index) = scene.top_keyboard_surface() {
            if escape_pressed && scene.clear_command_palette_query(surface_index) {
                self.request_repaint(RepaintRequest::NextFrame);
                escape_consumed = true;
            }
            for event in &keyboard_events {
                if escape_consumed
                    || event.state != KeyState::Pressed
                    || matches!(event.key, Key::Escape)
                {
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
                if !escape_consumed
                    && let TextInputEvent::Commit(text) = event
                    && scene.typeahead(surface_index, text, now_millis)
                {
                    self.request_repaint(RepaintRequest::NextFrame);
                }
            }
        }

        if let Some(request) =
            scene.dismissal_request(outside_activation, escape_pressed && !escape_consumed)
        {
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

                self.paint_overlay_row(&row, response.as_ref(), menu_presentation);
                self.push_semantic_node(overlay_row_semantics(&row, response.as_ref()));

                if !escape_consumed
                    && response
                        .is_some_and(|response| response.clicked && !response.keyboard_activated)
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
            .elevation_shadow(overlay_elevation_level(entry), self.theme.radii.md.top_left)
        {
            self.primitive(Primitive::Shadow(shadow.primitive(entry.rect)));
        }
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: entry.rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.overlay)),
            stroke: Some(stern_core::Stroke::new(
                self.theme.strokes.default,
                Brush::Solid(self.theme.colors.border.default),
            )),
            radius: self.theme.radii.md,
        }));
    }

    fn paint_overlay_row(
        &mut self,
        row: &OverlaySceneRow,
        response: Option<&stern_core::Response>,
        menu_presentation: Option<MenuPresentation<'_>>,
    ) {
        if row.kind == OverlaySceneRowKind::Separator {
            let height = self.theme.strokes.hairline;
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
        let baseline = row.rect.y + extra + font.size;

        if row.menu_columns
            && let Some(presentation) = menu_presentation
            && let Some(columns) = menu_column_geometry(row.rect)
        {
            if row.checked == Some(true) {
                let center = columns.state.center();
                let stroke = Stroke::new(self.theme.strokes.default, Brush::Solid(foreground));
                self.primitive(Primitive::Line(LinePrimitive {
                    from: Point::new(center.x - 5.0, center.y),
                    to: Point::new(center.x - 1.5, center.y + 3.0),
                    stroke,
                }));
                self.primitive(Primitive::Line(LinePrimitive {
                    from: Point::new(center.x - 1.5, center.y + 3.0),
                    to: Point::new(center.x + 5.0, center.y - 4.0),
                    stroke,
                }));
            }
            self.paint_clipped_overlay_text(
                row.id.child("menu-label-clip"),
                columns.label,
                row.label.clone(),
                Point::new(columns.label.x, baseline),
                font,
                foreground,
            );
            let shortcut_label = row.shortcut.as_ref().and_then(|shortcut| {
                shortcut.localized_label(presentation.platform, presentation.localizer)
            });
            if let Some(shortcut_label) = shortcut_label.filter(|label| !label.is_empty()) {
                self.paint_clipped_overlay_text(
                    row.id.child("menu-shortcut-clip"),
                    columns.shortcut,
                    shortcut_label,
                    Point::new(columns.shortcut.x, baseline),
                    font,
                    foreground,
                );
            }
            if row.expanded.is_some() {
                self.paint_overlay_text(
                    "›".to_owned(),
                    Point::new(columns.disclosure.x, baseline),
                    font,
                    foreground,
                );
            }
            return;
        }

        self.paint_overlay_text(
            row.label.clone(),
            Point::new(row.rect.x + self.theme.controls.padding_x, baseline),
            font,
            foreground,
        );
    }

    fn paint_clipped_overlay_text(
        &mut self,
        clip_owner: WidgetId,
        clip_rect: Rect,
        text: String,
        origin: Point,
        font: FontToken,
        foreground: Color,
    ) {
        let clip = ClipId::from_raw(clip_owner.raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: clip_rect,
        });
        self.paint_overlay_text(text, origin, font, foreground);
        self.primitive(Primitive::ClipEnd { id: clip });
    }

    fn paint_overlay_text(
        &mut self,
        text: String,
        origin: Point,
        font: FontToken,
        foreground: Color,
    ) {
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin,
            text,
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(foreground),
        }));
    }
}

fn overlay_elevation_level(entry: &crate::overlays::OverlayEntry) -> ElevationLevel {
    if entry.modal {
        return ElevationLevel::High;
    }

    match entry.kind {
        OverlayKind::Tooltip | OverlayKind::DragPreview => ElevationLevel::Low,
        OverlayKind::Popover
        | OverlayKind::Dropdown
        | OverlayKind::ContextMenu
        | OverlayKind::Menu => ElevationLevel::Medium,
        OverlayKind::CommandPalette | OverlayKind::Modal => ElevationLevel::High,
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

#[cfg(test)]
mod tests {
    use super::{MenuColumnGeometry, Rect, menu_column_geometry};

    #[test]
    fn menu_column_geometry_conformance() {
        assert_eq!(
            menu_column_geometry(Rect::new(0.0, 0.0, 272.0, 28.0)),
            Some(MenuColumnGeometry {
                state: Rect::new(8.0, 0.0, 16.0, 28.0),
                icon: Rect::new(32.0, 0.0, 16.0, 28.0),
                label: Rect::new(56.0, 0.0, 40.0, 28.0),
                status: Rect::new(104.0, 0.0, 16.0, 28.0),
                shortcut: Rect::new(128.0, 0.0, 112.0, 28.0),
                disclosure: Rect::new(248.0, 0.0, 16.0, 28.0),
            })
        );
        assert_eq!(
            menu_column_geometry(Rect::new(0.0, 0.0, 320.0, 36.0)),
            Some(MenuColumnGeometry {
                state: Rect::new(8.0, 0.0, 16.0, 36.0),
                icon: Rect::new(32.0, 0.0, 16.0, 36.0),
                label: Rect::new(56.0, 0.0, 88.0, 36.0),
                status: Rect::new(152.0, 0.0, 16.0, 36.0),
                shortcut: Rect::new(176.0, 0.0, 112.0, 36.0),
                disclosure: Rect::new(296.0, 0.0, 16.0, 36.0),
            })
        );
        assert_eq!(
            menu_column_geometry(Rect::new(13.0, 17.0, 272.0, 24.0)),
            Some(MenuColumnGeometry {
                state: Rect::new(21.0, 17.0, 16.0, 24.0),
                icon: Rect::new(45.0, 17.0, 16.0, 24.0),
                label: Rect::new(69.0, 17.0, 40.0, 24.0),
                status: Rect::new(117.0, 17.0, 16.0, 24.0),
                shortcut: Rect::new(141.0, 17.0, 112.0, 24.0),
                disclosure: Rect::new(261.0, 17.0, 16.0, 24.0),
            })
        );

        let below_threshold = f32::from_bits(272.0_f32.to_bits() - 1);
        for width in [below_threshold, 271.0, 264.0, 0.0] {
            assert_eq!(menu_column_geometry(Rect::new(3.0, 5.0, width, 28.0)), None);
        }
    }
}
