use stern_core::{
    Brush, ClipId, ComponentState, DomainDragGesturePhase, Key, KeyState, Point, Primitive, Rect,
    RectPrimitive, RepaintRequest, SemanticNode, SemanticRole, Stroke, TextPrimitive, TextRole,
    context_menu_trigger, drop_target,
};

use super::Ui;
use crate::dock::{
    Dock, DockController, DockControllerConfig, DockControllerFocus, DockControllerOutput,
    DockDropTarget, DockNeighborDirection, DockScene, DockSceneFrame, DockScenePanel,
    DockScenePreviewKind, DockSceneTab, DockSplitterContextRequest, FrameId, FrameLayout, PanelId,
    PanelInstanceLocation, frame_neighbor, resolve_dock_drop_target_with_policy,
    resolve_dock_splitter_context_actions_with_policy, solve_dock_layout,
    solve_dock_splitters_with_style,
};

fn dock_drop_target_frame(target: DockDropTarget) -> FrameId {
    match target {
        DockDropTarget::Tab { frame } | DockDropTarget::Split { frame, .. } => frame,
    }
}

fn dock_drop_target_is_current(dock: &Dock, target: DockDropTarget) -> bool {
    match target {
        DockDropTarget::Tab { frame } => dock.frame(frame).is_some(),
        DockDropTarget::Split {
            frame, new_frame, ..
        } => dock.frame(frame).is_some() && dock.frame(new_frame).is_none(),
    }
}

fn active_dock_focus(scene: &DockScene, dock: &Dock) -> Option<DockControllerFocus> {
    let frame = dock.active_frame()?;
    let panel = dock.frame(frame)?.active_panel()?.id;
    Some(DockControllerFocus {
        frame,
        panel,
        widget: scene.tab_widget_id(panel),
    })
}

fn scene_focus_for_widget(
    scene: &DockScene,
    widget: Option<stern_core::WidgetId>,
) -> Option<DockControllerFocus> {
    let widget = widget?;
    scene.layout().frames.iter().find_map(|frame| {
        frame.tabs.iter().find_map(|tab| {
            (tab.id == widget).then_some(DockControllerFocus {
                frame: frame.frame,
                panel: tab.panel,
                widget,
            })
        })
    })
}

impl Ui<'_> {
    /// Evaluates public Dock interactions against an immutable prepared scene.
    ///
    /// The Dock remains application-owned. Close and splitter context actions
    /// are emitted as requests; selection, docking, and resize mutations are
    /// applied through the existing deterministic Dock model API.
    #[allow(clippy::too_many_lines)]
    pub fn dock_controller(
        &mut self,
        scene: &DockScene,
        dock: &mut Dock,
        controller: &mut DockController,
        config: DockControllerConfig,
    ) -> DockControllerOutput {
        let before = dock.snapshot();
        let old_focus = self.memory().focused();
        let old_preview = controller.preview;
        let disabled = scene.config().disabled;
        let bounds = scene.layout().bounds;
        let frozen_layout = scene
            .layout()
            .frames
            .iter()
            .map(|frame| FrameLayout {
                frame: frame.frame,
                rect: frame.rect,
            })
            .collect::<Vec<_>>();
        let mut output = DockControllerOutput::default();

        self.reconcile_dock_controller(scene, dock, controller);
        let drag_was_retained = controller.drag.is_some();

        for frame in &scene.layout().frames {
            let response = self.pressable_with_id(frame.id, frame.rect, disabled);
            if response.clicked
                && let Some(panel) = dock
                    .frame(frame.frame)
                    .and_then(|item| item.active_panel())
                    .map(|panel| panel.id)
            {
                self.select_and_focus_dock_tab(
                    dock,
                    controller,
                    frame.frame,
                    panel,
                    scene.tab_widget_id(panel),
                );
            }
        }

        let mut drag_position = self.input().pointer.position;
        let mut drag_released = false;
        let mut drag_cancelled = false;
        for frame in &scene.layout().frames {
            for tab in &frame.tabs {
                let gesture = self.runtime.captured_domain_drag_gesture(
                    tab.id,
                    tab.rect,
                    disabled || !tab.draggable,
                );
                if gesture.response.clicked || gesture.response.keyboard_activated {
                    self.select_and_focus_dock_tab(
                        dock,
                        controller,
                        frame.frame,
                        tab.panel,
                        tab.id,
                    );
                }
                let source_visible = self.memory().drag_source() == Some(tab.id)
                    || self.memory().released_drag_source() == Some(tab.id)
                    || gesture.response.dragged;
                if source_visible && controller.drag.is_none() {
                    controller.drag = dock.begin_tab_drag(frame.frame, tab.panel);
                }
                for action in gesture.actions {
                    match action.phase {
                        DomainDragGesturePhase::Move => {
                            drag_position = action.position.or(drag_position);
                        }
                        DomainDragGesturePhase::Release => {
                            drag_position = action.position.or(drag_position);
                            drag_released = true;
                        }
                        DomainDragGesturePhase::Cancel => drag_cancelled = true,
                        DomainDragGesturePhase::Press => {}
                    }
                }

                if let Some(close_rect) = tab.close_rect {
                    let close = self.pressable_with_id(
                        tab.close_id,
                        close_rect,
                        disabled || !tab.close_visible,
                    );
                    if close.clicked || close.keyboard_activated {
                        output.close_requests.push(PanelInstanceLocation::new(
                            tab.panel.instance_id(),
                            frame.frame,
                        ));
                    }
                }
            }
        }

        let source_widget = controller.drag.map(|drag| scene.tab_widget_id(drag.panel));
        let mut hovered_frame = None;
        let mut dropped_frame = None;
        if source_widget.is_some() {
            let mut drop_surfaces = Vec::new();
            for frame in &scene.layout().frames {
                drop_surfaces.push((frame.id, frame.rect, frame.frame));
                for tab in &frame.tabs {
                    drop_surfaces.push((tab.id.child("dock-drop"), tab.rect, frame.frame));
                    if let Some(close_rect) = tab.close_rect {
                        drop_surfaces.push((
                            tab.close_id.child("dock-drop"),
                            close_rect,
                            frame.frame,
                        ));
                    }
                }
            }
            for (id, rect, frame) in drop_surfaces {
                let (input, memory) = self.runtime.input_and_memory_mut();
                let response = drop_target(id, rect, input, memory, disabled);
                if response.source == source_widget && response.response.state.hovered {
                    hovered_frame = Some(frame);
                }
                if response.source == source_widget && response.dropped {
                    dropped_frame = Some(frame);
                }
            }
        }

        if drag_cancelled {
            controller.drag = None;
            controller.preview = None;
        } else if let Some(drag) = controller.drag {
            let resolved = drag_position
                .and_then(|position| {
                    resolve_dock_drop_target_with_policy(
                        &frozen_layout,
                        position,
                        config.new_frame,
                        config.policy,
                    )
                })
                .filter(|target| dock_drop_target_is_current(dock, *target))
                .filter(|target| {
                    hovered_frame.is_some_and(|frame| dock_drop_target_frame(*target) == frame)
                        || (!drag_was_retained && self.memory().drag_source() == source_widget)
                });
            controller.preview = resolved;

            if drag_released || self.memory().released_drag_source() == source_widget {
                if let (Some(target), Some(frame)) = (resolved, dropped_frame)
                    && dock_drop_target_frame(target) == frame
                {
                    dock.drop_tab(drag, target);
                }
                controller.drag = None;
                controller.preview = None;
            }
        } else {
            controller.preview = None;
        }

        for splitter in &scene.layout().splitters {
            let gesture = self.runtime.captured_domain_drag_gesture(
                splitter.id,
                splitter.rect,
                disabled || !config.policy.splitters.allow_resize,
            );
            let pointer_context_requested = gesture.response.secondary_clicked;
            for action in gesture.actions {
                if matches!(action.phase, DomainDragGesturePhase::Move) {
                    dock.resize_split_with_policy(
                        &splitter.path,
                        bounds,
                        action.delta,
                        config.policy,
                    );
                }
            }

            let (input, memory) = self.runtime.input_and_memory_mut();
            let context = context_menu_trigger(splitter.id, splitter.rect, input, memory, disabled);
            if pointer_context_requested || context.context_requested {
                let frames = solve_dock_layout(dock, bounds);
                if let Some(model_splitter) =
                    solve_dock_splitters_with_style(dock, bounds, scene.config().chrome_style)
                        .into_iter()
                        .find(|candidate| candidate.path == splitter.path)
                {
                    output
                        .splitter_context_requests
                        .push(DockSplitterContextRequest {
                            path: splitter.path.clone(),
                            actions: resolve_dock_splitter_context_actions_with_policy(
                                dock,
                                &frames,
                                &model_splitter,
                                config.policy,
                            ),
                        });
                }
            }
        }

        self.handle_dock_keyboard(scene, dock, controller, &frozen_layout, disabled);

        output.changed = before != dock.snapshot();
        output.focus_changed = old_focus != self.memory().focused();
        output.drop_preview = controller.preview;
        if output.changed
            || output.focus_changed
            || old_preview != controller.preview
            || !output.close_requests.is_empty()
            || !output.splitter_context_requests.is_empty()
        {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn reconcile_dock_controller(
        &mut self,
        scene: &DockScene,
        dock: &mut Dock,
        controller: &mut DockController,
    ) {
        if dock.active_frame().is_none()
            && let Some(frame) = dock
                .frames()
                .into_iter()
                .find(|frame| frame.active_panel().is_some())
                .map(|frame| frame.id)
        {
            dock.set_active_frame(frame);
        }

        if let Some(drag) = controller.drag
            && dock
                .frame(drag.source_frame)
                .is_none_or(|frame| !frame.panels.iter().any(|panel| panel.id == drag.panel))
        {
            let widget = scene.tab_widget_id(drag.panel);
            if self.memory().drag_source() == Some(widget)
                || self.memory().released_drag_source() == Some(widget)
            {
                self.runtime.memory_mut().clear_drag();
            }
            controller.drag = None;
            controller.preview = None;
        }

        if controller
            .preview
            .is_some_and(|target| !dock_drop_target_is_current(dock, target))
        {
            controller.preview = None;
        }

        if let Some(focus) = controller.focus
            && dock
                .frame(focus.frame)
                .is_none_or(|frame| !frame.panels.iter().any(|panel| panel.id == focus.panel))
        {
            if let Some(frame) = dock.frames().into_iter().find_map(|frame| {
                frame
                    .panels
                    .iter()
                    .any(|panel| panel.id == focus.panel)
                    .then_some(frame.id)
            }) {
                controller.focus = Some(DockControllerFocus { frame, ..focus });
            } else if self.memory().focused() == Some(focus.widget) {
                if let Some(repaired) = active_dock_focus(scene, dock) {
                    self.runtime.memory_mut().focus(repaired.widget);
                    controller.focus = Some(repaired);
                } else {
                    self.runtime.memory_mut().clear_focus();
                    controller.focus = None;
                }
            } else {
                controller.focus = None;
            }
        }

        if let Some(focus) = scene_focus_for_widget(scene, self.memory().focused()) {
            controller.focus = Some(focus);
        }
    }

    fn select_and_focus_dock_tab(
        &mut self,
        dock: &mut Dock,
        controller: &mut DockController,
        frame: FrameId,
        panel: PanelId,
        widget: stern_core::WidgetId,
    ) {
        dock.select_panel(frame, panel);
        self.runtime.memory_mut().focus(widget);
        controller.focus = Some(DockControllerFocus {
            frame,
            panel,
            widget,
        });
    }

    fn handle_dock_keyboard(
        &mut self,
        scene: &DockScene,
        dock: &mut Dock,
        controller: &mut DockController,
        frozen_layout: &[FrameLayout],
        disabled: bool,
    ) {
        if disabled {
            return;
        }
        let Some(mut focus) = controller
            .focus
            .filter(|focus| self.memory().focused() == Some(focus.widget))
        else {
            return;
        };
        let events = self.input().keyboard.events.clone();
        for event in events {
            if event.state != KeyState::Pressed || event.modifiers.alt || event.modifiers.super_key
            {
                continue;
            }

            let target = if event.modifiers.ctrl {
                let direction = match event.key {
                    Key::ArrowLeft => Some(DockNeighborDirection::Left),
                    Key::ArrowRight => Some(DockNeighborDirection::Right),
                    Key::ArrowUp => Some(DockNeighborDirection::Up),
                    Key::ArrowDown => Some(DockNeighborDirection::Down),
                    _ => None,
                };
                direction
                    .and_then(|direction| frame_neighbor(frozen_layout, focus.frame, direction))
                    .and_then(|frame| {
                        dock.frame(frame)
                            .and_then(|item| item.active_panel())
                            .map(|panel| (frame, panel.id))
                    })
            } else {
                dock.frame(focus.frame).and_then(|frame| {
                    let current = frame
                        .panels
                        .iter()
                        .position(|panel| panel.id == focus.panel)?;
                    let last = frame.panels.len().saturating_sub(1);
                    let index = match event.key {
                        Key::ArrowLeft => current.saturating_sub(1),
                        Key::ArrowRight => current.saturating_add(1).min(last),
                        Key::Home => 0,
                        Key::End => last,
                        _ => return None,
                    };
                    Some((focus.frame, frame.panels[index].id))
                })
            };

            if let Some((frame, panel)) = target {
                let widget = scene.tab_widget_id(panel);
                self.select_and_focus_dock_tab(dock, controller, frame, panel, widget);
                focus = DockControllerFocus {
                    frame,
                    panel,
                    widget,
                };
            }
        }
    }

    /// Paints one prepared public Dock → Frame → Panel scene.
    ///
    /// The callback runs exactly once for each active panel body with positive
    /// area, in deterministic dock-tree order, under that panel's exact clip.
    /// This presentation packet does not mutate the caller-owned [`crate::dock::Dock`].
    pub fn dock_scene<T>(
        &mut self,
        scene: &DockScene,
        mut panel_content: impl FnMut(&mut Self, &DockScenePanel) -> T,
    ) -> Vec<T> {
        let layout = scene.layout();
        if layout.bounds == Rect::ZERO {
            return Vec::new();
        }

        self.register_id(scene.root_widget_id());
        self.paint_dock_root(layout.bounds);
        self.push_semantic_node(
            SemanticNode::new(scene.root_widget_id(), SemanticRole::Dock, layout.bounds)
                .with_label("Editor dock")
                .with_children(layout.frames.iter().map(|frame| frame.id)),
        );

        let mut output = Vec::with_capacity(layout.frames.len());
        for frame in &layout.frames {
            self.paint_dock_frame(frame, scene.config().disabled);
            if let Some(panel) = &frame.panel {
                self.register_id(panel.id);
                self.paint_dock_panel(panel.rect);
                self.push_semantic_node(
                    SemanticNode::new(panel.id, SemanticRole::Panel, panel.rect)
                        .with_label(&panel.title),
                );

                let clip = ClipId::from_raw(panel.id.child("clip").raw());
                self.primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: panel.rect,
                });
                self.runtime
                    .push_id_scope(("dock-panel-content", panel.id.raw()));
                output.push(panel_content(self, panel));
                self.runtime.pop_id_scope();
                self.primitive(Primitive::ClipEnd { id: clip });
            }
        }

        for splitter in &layout.splitters {
            self.register_id(splitter.id);
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: splitter.rect,
                fill: Some(Brush::Solid(self.theme.colors.border.default)),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }

        if let Some(preview) = layout.preview {
            self.register_id(preview.id);
            let (alpha, radius) = match preview.kind {
                DockScenePreviewKind::Merge => (0.20, self.theme.radii.sm),
                DockScenePreviewKind::Split(_) => (0.32, self.theme.radii.none),
            };
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: preview.rect,
                fill: Some(Brush::Solid(
                    self.theme.colors.accent.default.with_alpha(alpha),
                )),
                stroke: Some(Stroke::new(
                    self.theme.controls.border_width.max(1.0),
                    Brush::Solid(self.theme.colors.accent.default),
                )),
                radius,
            }));
        }

        output
    }

    fn paint_dock_root(&mut self, rect: Rect) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.workspace)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_dock_frame(&mut self, frame: &DockSceneFrame, disabled: bool) {
        self.register_id(frame.id);
        let fill = if frame.active {
            self.theme.colors.surface.panel_raised
        } else {
            self.theme.colors.surface.panel
        };
        let border = if frame.active {
            self.theme.colors.focus.ring
        } else {
            self.theme.colors.border.subtle
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: frame.rect,
            fill: Some(Brush::Solid(fill)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(border),
            )),
            radius: self.theme.radii.none,
        }));

        self.register_id(frame.tab_list_id);
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: frame.tab_list_rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.sunken)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));

        let mut frame_node = SemanticNode::new(frame.id, SemanticRole::Frame, frame.rect)
            .with_label(
                frame
                    .panel
                    .as_ref()
                    .map_or("Empty frame", |panel| panel.title.as_str()),
            )
            .with_children(
                core::iter::once(frame.tab_list_id).chain(frame.panel.iter().map(|panel| panel.id)),
            );
        frame_node.state.selected = frame.active;
        frame_node.state.disabled = disabled;
        self.push_semantic_node(frame_node);

        self.push_semantic_node(
            SemanticNode::new(
                frame.tab_list_id,
                SemanticRole::TabList,
                frame.tab_list_rect,
            )
            .with_label("Frame tabs")
            .with_children(frame.tabs.iter().map(|tab| tab.id)),
        );

        let strip_clip = ClipId::from_raw(frame.tab_list_id.child("clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: strip_clip,
            rect: frame.tab_list_rect,
        });
        for tab in &frame.tabs {
            self.paint_dock_tab(tab, disabled);
        }
        self.primitive(Primitive::ClipEnd { id: strip_clip });
    }

    fn paint_dock_tab(&mut self, tab: &DockSceneTab, disabled: bool) {
        self.register_id(tab.id);
        let recipe = self.theme.tab(ComponentState {
            disabled,
            selected: tab.selected,
            ..ComponentState::default()
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: tab.rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        if let Some(indicator) = recipe.indicator {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: Rect::new(
                    tab.rect.x,
                    tab.rect.max_y() - recipe.indicator_thickness,
                    tab.rect.width,
                    recipe.indicator_thickness,
                ),
                fill: Some(indicator),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }

        let tab_clip = ClipId::from_raw(tab.id.child("clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: tab_clip,
            rect: tab.rect,
        });
        let font = self.theme.font(TextRole::Label);
        let extra = (tab.rect.height - font.line_height).max(0.0) * 0.5;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                tab.rect.x + self.theme.controls.padding_x,
                tab.rect.y + extra + font.size,
            ),
            text: tab.title.clone(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));
        if let Some(close_rect) = tab.close_rect {
            self.register_id(tab.close_id);
            self.primitive(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(
                    close_rect.x + self.theme.controls.padding_x * 0.5,
                    close_rect.y + extra + font.size,
                ),
                text: "×".to_owned(),
                family: font.family.to_owned(),
                size: font.size,
                line_height: font.line_height,
                brush: Brush::Solid(recipe.foreground),
            }));
        }
        self.primitive(Primitive::ClipEnd { id: tab_clip });

        let mut node = SemanticNode::new(tab.id, SemanticRole::Tab, tab.rect)
            .with_label(&tab.title)
            .focusable(!disabled);
        node.state.selected = tab.selected;
        node.state.focused = self.memory().is_focused(tab.id);
        node.state.disabled = disabled;
        self.push_semantic_node(node);
    }

    fn paint_dock_panel(&mut self, rect: Rect) {
        let recipe = self.theme.panel();
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
    }
}
