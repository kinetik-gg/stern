use stern_core::{
    Brush, ClipId, Color, ComponentState, InteractionState, Key, KeyState, MouseButton, Point,
    Primitive, Rect, RectPrimitive, RepaintRequest, Response, SemanticAction, SemanticActionKind,
    SemanticNode, SemanticRole, SemanticValue, Stroke, TextPrimitive, TextRole, UiInput,
    UiInputEvent, WidgetId, pressable,
};

use super::Ui;
use crate::components::{AssetSlotOutput, ColorFieldOutput, PathFieldOutput, SelectFieldOutput};
use crate::inspector::pickers::{
    AssetPickerItem, ColorPickerAction, ColorPickerChannel, ColorPickerScene,
    InspectorPickerCancelReason, InspectorPickerCommit, InspectorPickerKind, InspectorPickerOutput,
    InspectorPickerScene, InspectorPickerSceneKind, InspectorPickerState, PathPickerKind,
    PathPickerResult,
};
use crate::overlays::{
    DropdownModel, OverlayId, OverlayScene, OverlaySceneDismissReason, OverlaySceneIntent,
    overlay_semantics,
};

impl Ui<'_> {
    /// Connects a live select entry request to a retained dropdown scene.
    pub fn select_picker(
        &mut self,
        state: &mut InspectorPickerState,
        field: &SelectFieldOutput,
        overlay_id: OverlayId,
        bounds: Rect,
        label: impl Into<String>,
        model: &DropdownModel,
    ) -> bool {
        let opened = state.open_select_from(field, overlay_id, bounds, label, model);
        if opened {
            state.mark_scene_opened_frame(self.time().frame_index);
            self.request_repaint(RepaintRequest::NextFrame);
        }
        opened
    }

    /// Connects a live color entry request to a retained draft overlay.
    pub fn color_picker(
        &mut self,
        state: &mut InspectorPickerState,
        field: &ColorFieldOutput,
        overlay_id: OverlayId,
        bounds: Rect,
    ) -> bool {
        let opened = state.open_color_from(field, overlay_id, bounds);
        if opened {
            state.mark_scene_opened_frame(self.time().frame_index);
            self.request_repaint(RepaintRequest::NextFrame);
        }
        opened
    }

    /// Connects a live asset entry request to a retained asset-choice overlay.
    #[allow(clippy::too_many_arguments)]
    pub fn asset_picker(
        &mut self,
        state: &mut InspectorPickerState,
        field: &AssetSlotOutput,
        overlay_id: OverlayId,
        bounds: Rect,
        label: impl Into<String>,
        items: &[AssetPickerItem],
    ) -> bool {
        let opened = state.open_asset_from(field, overlay_id, bounds, label, items);
        if opened {
            state.mark_scene_opened_frame(self.time().frame_index);
            self.request_repaint(RepaintRequest::NextFrame);
        }
        opened
    }

    /// Connects a live path browse request to a redacted host-service session.
    pub fn path_picker(
        &mut self,
        state: &mut InspectorPickerState,
        field: &PathFieldOutput,
        kind: PathPickerKind,
    ) -> bool {
        let opened = state.open_path_from(field, kind);
        if opened {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        opened
    }

    /// Paints and evaluates the active select, color, or asset scene.
    ///
    /// Call [`crate::inspector::InspectorPickerScene::declare_pointer_targets`] before lower UI
    /// whenever [`InspectorPickerState::scene`] returns a scene.
    pub fn inspector_picker_scene(
        &mut self,
        state: &mut InspectorPickerState,
    ) -> InspectorPickerOutput {
        let active = state.kind();
        let service_request = state.take_path_service_request();
        let Some(mut scene) = state.take_scene() else {
            return InspectorPickerOutput {
                active,
                service_request,
                ..InspectorPickerOutput::default()
            };
        };
        if scene.opened_frame == Some(self.time().frame_index) {
            scene.opened_frame = None;
            self.paint_picker_opening_frame(&scene);
            state.restore_scene(scene);
            return InspectorPickerOutput {
                active,
                service_request,
                ..InspectorPickerOutput::default()
            };
        }
        scene.opened_frame = None;
        let resolution = match &mut scene.kind {
            InspectorPickerSceneKind::Select {
                trigger,
                overlay_id,
                scene,
            } => self.evaluate_dropdown_picker(
                scene,
                *overlay_id,
                *trigger,
                InspectorPickerKind::Select,
                None,
            ),
            InspectorPickerSceneKind::Asset {
                trigger,
                overlay_id,
                scene,
                identities,
            } => self.evaluate_dropdown_picker(
                scene,
                *overlay_id,
                *trigger,
                InspectorPickerKind::Asset,
                Some(identities),
            ),
            InspectorPickerSceneKind::Color(scene) => self.evaluate_color_picker(scene),
        };

        if let Some(resolution) = resolution {
            self.runtime.memory_mut().focus(resolution.focus_return);
            self.request_repaint(RepaintRequest::NextFrame);
            InspectorPickerOutput {
                active,
                commit: resolution.commit,
                cancel: resolution.cancel,
                service_request,
                focus_return: Some(resolution.focus_return),
            }
        } else {
            state.restore_scene(scene);
            InspectorPickerOutput {
                active,
                service_request,
                ..InspectorPickerOutput::default()
            }
        }
    }

    /// Resolves a matching host path result and restores trigger focus.
    pub fn resolve_path_picker_result(
        &mut self,
        state: &mut InspectorPickerState,
        result: PathPickerResult,
    ) -> Option<InspectorPickerOutput> {
        let output = state.resolve_path_result(result)?;
        if let Some(focus_return) = output.focus_return {
            self.runtime.memory_mut().focus(focus_return);
        }
        self.request_repaint(RepaintRequest::NextFrame);
        Some(output)
    }

    fn evaluate_dropdown_picker(
        &mut self,
        scene: &mut OverlayScene,
        overlay_id: OverlayId,
        trigger: WidgetId,
        kind: InspectorPickerKind,
        identities: Option<&std::collections::BTreeMap<crate::DropdownItemId, String>>,
    ) -> Option<PickerResolution> {
        let output = self.overlay_scene(scene);
        for intent in output.intents {
            match intent {
                OverlaySceneIntent::SelectDropdown(selection)
                    if selection.overlay_id == overlay_id =>
                {
                    let commit = if kind == InspectorPickerKind::Asset {
                        let identity = identities?.get(&selection.item_id)?.clone();
                        InspectorPickerCommit::Asset(identity)
                    } else {
                        InspectorPickerCommit::Select(selection.item_id)
                    };
                    return Some(PickerResolution::commit(commit, selection.focus_return));
                }
                OverlaySceneIntent::Dismiss(request) if request.overlay_id == overlay_id => {
                    let reason = match request.reason {
                        OverlaySceneDismissReason::Escape => InspectorPickerCancelReason::Escape,
                        OverlaySceneDismissReason::OutsideClick => {
                            InspectorPickerCancelReason::OutsideClick
                        }
                    };
                    return Some(PickerResolution::cancel(
                        reason,
                        request.focus_return.unwrap_or(trigger),
                    ));
                }
                OverlaySceneIntent::Action(_)
                | OverlaySceneIntent::OpenSubmenu(_)
                | OverlaySceneIntent::SelectDropdown(_)
                | OverlaySceneIntent::Dismiss(_) => {}
            }
        }
        None
    }

    fn evaluate_color_picker(&mut self, scene: &mut ColorPickerScene) -> Option<PickerResolution> {
        let keyboard_events = self.input().keyboard.events.clone();
        let outside = primary_activation(self.input())
            .is_some_and(|point| !scene.bounds.contains_point(point));
        let escape = keyboard_events
            .iter()
            .any(|event| event.state == KeyState::Pressed && matches!(event.key, Key::Escape));
        let apply_from_keyboard = keyboard_events.iter().any(|event| {
            event.state == KeyState::Pressed && !event.repeat && matches!(event.key, Key::Enter)
        });

        self.paint_color_picker_surface(scene);
        let controls = scene.controls();
        self.push_color_picker_root(scene, &controls);

        let clip = ClipId::from_raw(scene.root.child("clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: scene.bounds,
        });
        self.paint_color_channel_labels(scene);
        let mut resolution = None;
        for control in controls {
            if control.rect.intersection(scene.bounds).is_none() {
                continue;
            }
            let id = self.register_id(control.id);
            let (input, memory) = self.runtime.input_and_memory_mut();
            let response = pressable(id, control.rect, input, memory, false);
            self.paint_color_control(&control, &response);
            self.push_semantic_node(color_control_semantics(&control, &response));
            if response.clicked {
                match control.action {
                    ColorPickerAction::Adjust(channel, delta) => {
                        scene.adjust(channel, delta);
                        self.request_repaint(RepaintRequest::NextFrame);
                    }
                    ColorPickerAction::Apply => {
                        resolution = Some(PickerResolution::commit(
                            InspectorPickerCommit::Color(scene.draft),
                            scene.trigger,
                        ));
                    }
                    ColorPickerAction::Cancel => {
                        resolution = Some(PickerResolution::cancel(
                            InspectorPickerCancelReason::Explicit,
                            scene.trigger,
                        ));
                    }
                }
            }
        }
        self.primitive(Primitive::ClipEnd { id: clip });

        if resolution.is_some() {
            resolution
        } else if escape {
            Some(PickerResolution::cancel(
                InspectorPickerCancelReason::Escape,
                scene.trigger,
            ))
        } else if outside {
            Some(PickerResolution::cancel(
                InspectorPickerCancelReason::OutsideClick,
                scene.trigger,
            ))
        } else if apply_from_keyboard {
            Some(PickerResolution::commit(
                InspectorPickerCommit::Color(scene.draft),
                scene.trigger,
            ))
        } else {
            None
        }
    }

    fn paint_color_picker_surface(&mut self, scene: &ColorPickerScene) {
        if let Some(shadow) = self.theme.elevation_shadow(
            stern_core::ElevationLevel::Medium,
            self.theme.radii.md.top_left,
        ) {
            self.primitive(Primitive::Shadow(shadow.primitive(scene.bounds)));
        }
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: scene.bounds,
            fill: Some(Brush::Solid(self.theme.colors.surface.overlay)),
            stroke: Some(Stroke::new(
                self.theme.strokes.default,
                Brush::Solid(self.theme.colors.border.default),
            )),
            radius: self.theme.radii.md,
        }));
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: Rect::new(scene.bounds.x + 8.0, scene.bounds.y + 8.0, 32.0, 20.0),
            fill: Some(Brush::Solid(scene.draft)),
            stroke: Some(Stroke::new(
                self.theme.strokes.default,
                Brush::Solid(self.theme.colors.border.default),
            )),
            radius: self.theme.radii.sm,
        }));
    }

    fn paint_picker_opening_frame(&mut self, scene: &InspectorPickerScene) {
        match &scene.kind {
            InspectorPickerSceneKind::Select { scene, .. }
            | InspectorPickerSceneKind::Asset { scene, .. } => {
                self.paint_passive_dropdown_scene(scene);
            }
            InspectorPickerSceneKind::Color(scene) => {
                self.paint_color_picker_surface(scene);
                let controls = scene.controls();
                self.push_color_picker_root(scene, &controls);
                let clip = ClipId::from_raw(scene.root.child("clip").raw());
                self.primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: scene.bounds,
                });
                self.paint_color_channel_labels(scene);
                for control in controls {
                    let response =
                        Response::new(control.id, control.rect, InteractionState::default());
                    self.paint_color_control(&control, &response);
                    self.push_semantic_node(color_control_semantics(&control, &response));
                }
                self.primitive(Primitive::ClipEnd { id: clip });
            }
        }
    }

    fn paint_passive_dropdown_scene(&mut self, scene: &OverlayScene) {
        for surface_index in 0..scene.surfaces().len() {
            let surface = &scene.surfaces()[surface_index];
            let entry = surface.entry();
            self.paint_picker_panel(entry.rect);
            let rows = scene.rows(surface_index);
            let children = rows.iter().map(|row| row.id).collect::<Vec<_>>();
            self.push_semantic_node(
                overlay_semantics(entry, surface.label()).with_children(children),
            );
            let clip = ClipId::from_raw(
                WidgetId::from_raw(entry.id.raw())
                    .child("inspector-picker-opening-clip")
                    .raw(),
            );
            self.primitive(Primitive::ClipBegin {
                id: clip,
                rect: entry.rect,
            });
            for row in rows {
                let recipe = self.theme.row(ComponentState {
                    hovered: false,
                    pressed: false,
                    focused: false,
                    disabled: !row.enabled,
                    selected: row.selected,
                });
                self.primitive(Primitive::Rect(RectPrimitive {
                    rect: row.rect,
                    fill: Some(recipe.background),
                    stroke: Some(recipe.border),
                    radius: recipe.radius,
                }));
                let font = self.theme.font(TextRole::Label);
                self.primitive(Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: Point::new(
                        row.rect.x + self.theme.controls.padding_x,
                        row.rect.y
                            + (row.rect.height - font.line_height).max(0.0) * 0.5
                            + font.size,
                    ),
                    text: row.label.clone(),
                    family: font.family.to_owned(),
                    size: font.size,
                    line_height: font.line_height,
                    brush: Brush::Solid(recipe.foreground),
                }));
                let mut node = SemanticNode::new(row.id, row.role.clone(), row.rect)
                    .with_label(&row.label)
                    .focusable(row.enabled);
                node.state.disabled = !row.enabled;
                node.state.selected = row.selected;
                node.state.checked = row.checked;
                node.state.expanded = row.expanded;
                node.state.value = Some(SemanticValue::Text(row.label.clone()));
                if row.enabled {
                    node.actions.push(SemanticAction::new(
                        SemanticActionKind::Invoke,
                        "Select item",
                    ));
                }
                self.push_semantic_node(node);
            }
            self.primitive(Primitive::ClipEnd { id: clip });
        }
    }

    fn paint_picker_panel(&mut self, bounds: Rect) {
        if let Some(shadow) = self.theme.elevation_shadow(
            stern_core::ElevationLevel::Medium,
            self.theme.radii.md.top_left,
        ) {
            self.primitive(Primitive::Shadow(shadow.primitive(bounds)));
        }
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: bounds,
            fill: Some(Brush::Solid(self.theme.colors.surface.overlay)),
            stroke: Some(Stroke::new(
                self.theme.strokes.default,
                Brush::Solid(self.theme.colors.border.default),
            )),
            radius: self.theme.radii.md,
        }));
    }

    fn push_color_picker_root(
        &mut self,
        scene: &ColorPickerScene,
        controls: &[crate::inspector::pickers::ColorPickerControl],
    ) {
        let children = controls
            .iter()
            .map(|control| control.id)
            .collect::<Vec<_>>();
        let mut root = SemanticNode::new(
            scene.root,
            SemanticRole::Custom("color-picker".to_owned()),
            scene.bounds,
        )
        .with_label("Color picker")
        .with_children(children);
        root.description = Some(color_description(scene.draft));
        root.state.expanded = Some(true);
        root.actions.push(SemanticAction::new(
            SemanticActionKind::Dismiss,
            "Cancel color picker",
        ));
        self.push_semantic_node(root);
    }

    fn paint_color_channel_labels(&mut self, scene: &ColorPickerScene) {
        let font = self.theme.font(TextRole::Label);
        for (channel, offset) in ColorPickerChannel::ALL
            .into_iter()
            .zip([0.0_f32, 1.0, 2.0, 3.0])
        {
            let y = scene.bounds.y + 4.0 + offset * 28.0;
            self.primitive(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(scene.bounds.x + 46.0, y + font.size + 5.0),
                text: format!("{} {:.3}", channel.label(), channel.value(scene.draft)),
                family: font.family.to_owned(),
                size: font.size,
                line_height: font.line_height,
                brush: Brush::Solid(self.theme.colors.content.primary),
            }));
        }
    }

    fn paint_color_control(
        &mut self,
        control: &crate::inspector::pickers::ColorPickerControl,
        response: &stern_core::Response,
    ) {
        let recipe = self.theme.button(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: false,
            selected: false,
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: control.rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        let font = self.theme.font(TextRole::Label);
        let visible_label = match control.action {
            ColorPickerAction::Adjust(_, delta) if delta < 0.0 => "-",
            ColorPickerAction::Adjust(_, _) => "+",
            ColorPickerAction::Apply => "Apply",
            ColorPickerAction::Cancel => "Cancel",
        };
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                control.rect.x + self.theme.controls.padding_x.min(control.rect.width),
                control.rect.y
                    + (control.rect.height - font.line_height).max(0.0) * 0.5
                    + font.size,
            ),
            text: visible_label.to_owned(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));
    }
}

struct PickerResolution {
    commit: Option<InspectorPickerCommit>,
    cancel: Option<InspectorPickerCancelReason>,
    focus_return: WidgetId,
}

impl PickerResolution {
    fn commit(commit: InspectorPickerCommit, focus_return: WidgetId) -> Self {
        Self {
            commit: Some(commit),
            cancel: None,
            focus_return,
        }
    }

    const fn cancel(cancel: InspectorPickerCancelReason, focus_return: WidgetId) -> Self {
        Self {
            commit: None,
            cancel: Some(cancel),
            focus_return,
        }
    }
}

fn color_control_semantics(
    control: &crate::inspector::pickers::ColorPickerControl,
    response: &stern_core::Response,
) -> SemanticNode {
    let mut node = SemanticNode::new(control.id, SemanticRole::Button, control.rect)
        .with_label(&control.label)
        .focusable(true);
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    node.actions.push(SemanticAction::new(
        match control.action {
            ColorPickerAction::Adjust(_, delta) if delta < 0.0 => SemanticActionKind::Decrement,
            ColorPickerAction::Adjust(_, _) => SemanticActionKind::Increment,
            ColorPickerAction::Apply | ColorPickerAction::Cancel => SemanticActionKind::Invoke,
        },
        control.label.clone(),
    ));
    node
}

fn color_description(color: Color) -> String {
    format!(
        "RGBA {:.3}, {:.3}, {:.3}, {:.3}",
        color.r, color.g, color.b, color.a
    )
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
