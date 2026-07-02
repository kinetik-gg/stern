impl EditorShowcase {
    pub(super) fn tool_bar(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let toolbar = self.toolbar_model();
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        let mut x = 10.0;
        let tool_items = toolbar
            .group(EditorToolbarGroupKind::Tools.id())
            .expect("editor toolbar declares tool group")
            .visible_items();
        let mut tool_responses = Vec::new();
        for (visible_index, ((_, icon, _label, action), item)) in
            EDITOR_TOOL_BUTTONS.into_iter().zip(tool_items).enumerate()
        {
            let button = Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button);
            let id = ui.id(("editor.tool", action));
            let disabled = !item.enabled();
            let response = ui.pressable_with_id(id, button, disabled);
            if response.clicked {
                ui.request_repaint(RepaintRequest::NextFrame);
                let mut queue = ActionQueue::new();
                if toolbar.invoke_group_visible(
                    EditorToolbarGroupKind::Tools.id(),
                    visible_index,
                    &mut queue,
                    ActionContext::Editor,
                ) {
                    self.handle_action_queue(invocations, &mut queue);
                }
            }
            tool_responses.push((
                id,
                response,
                button,
                EDITOR_TOOL_BUTTONS[visible_index].0,
                icon,
                item.label(),
                disabled,
            ));
            x += chrome.toolbar_stride;
        }
        for (id, response, button, tool, icon, label, disabled) in tool_responses {
            paint_toolbar_icon_button_sized(
                ui,
                id,
                response,
                button,
                icon,
                label,
                self.selected_tool == tool,
                disabled,
                chrome.toolbar_icon,
            );
        }

        rect(
            ui,
            Rect::new(x + 4.0, TOOLBAR_Y + 3.0, 1.0, chrome.toolbar_button - 6.0),
            rgb(57, 60, 66),
            None,
        );
        x += 18.0;
        let viewport_items = toolbar
            .group(EditorToolbarGroupKind::Viewport.id())
            .expect("editor toolbar declares viewport group")
            .visible_items();
        for ((icon, _label, action), item) in [
            (ToolbarIcon::Grid, "Toggle grid", ACTION_GRID),
            (
                ToolbarIcon::Crosshair,
                "Frame selected",
                ACTION_VIEWPORT_FIT_SELECTION,
            ),
            (
                ToolbarIcon::Reset,
                "Reset view",
                ACTION_VIEWPORT_FIT_CONTENT,
            ),
        ]
        .into_iter()
        .zip(viewport_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.viewport-tool", action, icon.raw()),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked && item.can_invoke() {
                self.trigger(invocations, action, ActionSource::Button);
            }
            x += chrome.toolbar_stride;
        }

        rect(
            ui,
            Rect::new(x + 4.0, TOOLBAR_Y + 3.0, 1.0, chrome.toolbar_button - 6.0),
            rgb(57, 60, 66),
            None,
        );
        x += 18.0;
        let dock_items = toolbar
            .group(EditorToolbarGroupKind::Dock.id())
            .expect("editor toolbar declares dock group")
            .visible_items();
        for ((kind, icon, _label, action), item) in [
            (
                DockSplitterContextActionKind::Join,
                ToolbarIcon::Component,
                "Join dock splitter",
                ACTION_DOCK_JOIN,
            ),
            (
                DockSplitterContextActionKind::Swap,
                ToolbarIcon::Layers,
                "Swap dock frames",
                ACTION_DOCK_SWAP,
            ),
        ]
        .into_iter()
        .zip(dock_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.dock-action", action),
                Rect::new(x, TOOLBAR_Y, chrome.toolbar_button, chrome.toolbar_button),
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked && item.can_invoke() {
                let bounds = editor_workspace_rect(ui.theme(), viewport);
                if self.apply_splitter_context_action(bounds, kind) {
                    invocations.push(ActionInvocation::new(
                        ActionId::new(action),
                        ActionSource::Button,
                        ActionContext::Editor,
                    ));
                }
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            x += chrome.toolbar_stride;
        }

        let run_items = toolbar
            .group(EditorToolbarGroupKind::Run.id())
            .expect("editor toolbar declares run group")
            .visible_items();
        for ((index, icon, _label, action, rect), item) in run_toolbar_buttons(viewport, chrome)
            .into_iter()
            .zip(run_items)
        {
            let response = toolbar_icon_button(
                ui,
                ("editor.run", action, index),
                rect,
                icon,
                item.label(),
                false,
                !item.enabled(),
            );
            if response.clicked {
                let mut queue = ActionQueue::new();
                if toolbar.invoke_group_visible(
                    EditorToolbarGroupKind::Run.id(),
                    index,
                    &mut queue,
                    ActionContext::Editor,
                ) {
                    self.handle_action_queue(invocations, &mut queue);
                }
            }
        }
    }

    pub(super) fn apply_splitter_context_action(
        &mut self,
        bounds: Rect,
        kind: DockSplitterContextActionKind,
    ) -> bool {
        let frame_layouts = solve_dock_layout(&self.dock, bounds);
        let Some(splitter) =
            solve_dock_splitters_with_style(&self.dock, bounds, editor_dock_chrome_style())
                .into_iter()
                .next()
        else {
            "No dock splitter action available".clone_into(&mut self.status);
            return false;
        };
        let policy = editor_dock_interaction_policy();
        let actions = resolve_dock_splitter_context_actions_with_policy(
            &self.dock,
            &frame_layouts,
            &splitter,
            policy,
        );
        let Some(action) = actions
            .into_iter()
            .find(|action| action.kind == kind && action.enabled)
        else {
            match kind {
                DockSplitterContextActionKind::Join => "No dock join action available",
                DockSplitterContextActionKind::Swap => "No dock swap action available",
            }
            .clone_into(&mut self.status);
            return false;
        };

        match kind {
            DockSplitterContextActionKind::Join => {
                let Some(request) = action.join_request() else {
                    "No dock join action available".clone_into(&mut self.status);
                    return false;
                };
                let source = request.source_frame();
                let target = request.target_frame();
                if self
                    .dock
                    .apply_join_request_with_policy(bounds, request, policy)
                {
                    self.status = format!(
                        "Dock splitter joined frame {} into frame {}",
                        source.raw(),
                        target.raw()
                    );
                    true
                } else {
                    "Dock join request rejected".clone_into(&mut self.status);
                    false
                }
            }
            DockSplitterContextActionKind::Swap => {
                let Some(request) = action.swap_request() else {
                    "No dock swap action available".clone_into(&mut self.status);
                    return false;
                };
                let source = request.source_frame();
                let target = request.target_frame();
                if self
                    .dock
                    .apply_swap_request_with_policy(bounds, request, policy)
                {
                    self.status = format!(
                        "Dock splitter swapped frame {} with frame {}",
                        source.raw(),
                        target.raw()
                    );
                    true
                } else {
                    "Dock swap request rejected".clone_into(&mut self.status);
                    false
                }
            }
        }
    }

    pub(super) fn workspace(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        let bounds = editor_workspace_rect(ui.theme(), viewport);
        let dock_semantic_id = ui.id("editor.dock.semantic");
        ui.push_semantic_node(
            SemanticNode::new(dock_semantic_id, SemanticRole::Dock, bounds)
                .with_label("Editor Dock"),
        );
        let frame_layouts = solve_dock_layout(&self.dock, bounds);
        let mut tab_drags = Vec::new();
        for layout in &frame_layouts {
            let frame_rect = layout.rect.inset(2.0);
            let Some(frame_snapshot) = self.dock.frame(layout.frame).cloned() else {
                continue;
            };
            self.frame_tab_interactions(
                ui,
                layout.frame,
                frame_rect,
                26.0,
                &frame_snapshot,
                &mut tab_drags,
            );
        }
        for layout in &frame_layouts {
            self.editor_frame(ui, layout.frame, layout.rect.inset(2.0));
        }
        self.frame_drop_targets(ui, &frame_layouts, &tab_drags);

        let chrome_style = editor_dock_chrome_style();
        let interaction_policy = editor_dock_interaction_policy();
        for splitter in solve_dock_splitters_with_style(&self.dock, bounds, chrome_style) {
            let response = ui.draggable(
                ("editor.splitter", splitter.path.clone()),
                splitter.rect,
                false,
            );
            if response.dragged {
                self.dock.resize_split_with_policy(
                    &splitter.path,
                    bounds,
                    response.drag_delta,
                    interaction_policy,
                );
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            let color = if response.state.hovered || response.state.active {
                rgb(70, 116, 190)
            } else {
                rgb(38, 40, 45)
            };
            rect(ui, splitter.rect, color, None);
        }
    }

    pub(super) fn editor_frame(&mut self, ui: &mut Ui<'_>, frame_id: FrameId, frame_rect: Rect) {
        if frame_rect.width <= 1.0 || frame_rect.height <= 1.0 {
            return;
        }

        let tab_height = 26.0;
        let active_frame = self.dock.active_frame() == Some(frame_id);
        rect(
            ui,
            frame_rect,
            if active_frame {
                rgb(30, 33, 39)
            } else {
                rgb(28, 29, 32)
            },
            Some(if active_frame {
                rgb(78, 128, 210)
            } else {
                rgb(57, 59, 65)
            }),
        );
        let frame_semantic_id = ui.id(("editor.frame.semantic", frame_id.raw()));
        let mut frame_semantics =
            SemanticNode::new(frame_semantic_id, SemanticRole::Frame, frame_rect)
                .with_label(format!("Frame {}", frame_id.raw()))
                .focusable(true);
        frame_semantics.state.focused = active_frame;
        ui.push_semantic_node(frame_semantics);
        let Some(frame_snapshot) = self.dock.frame(frame_id).cloned() else {
            return;
        };
        for (tab, tab_rect) in frame_tab_rects(&frame_snapshot, frame_rect, tab_height) {
            ui.tab_button(
                ("editor.frame-tab", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                tab.title,
                tab.active,
                false,
            );
        }
        rect(
            ui,
            Rect::new(
                frame_rect.x + 1.0,
                frame_rect.y + tab_height + 1.0,
                (frame_rect.width - 2.0).max(0.0),
                1.0,
            ),
            rgb(48, 50, 56),
            None,
        );

        let body = Rect::new(
            frame_rect.x + 1.0,
            frame_rect.y + tab_height + 2.0,
            (frame_rect.width - 2.0).max(0.0),
            (frame_rect.height - tab_height - 3.0).max(0.0),
        );
        let active_panel = self
            .dock
            .frame(frame_id)
            .and_then(Frame::active_panel)
            .cloned();
        if let Some(panel) = active_panel.as_ref() {
            let panel_semantic_id =
                ui.id(("editor.panel.semantic", frame_id.raw(), panel.id.raw()));
            ui.push_semantic_node(
                SemanticNode::new(panel_semantic_id, SemanticRole::Panel, body)
                    .with_label(panel.title.clone()),
            );
        }
        ui.clip_rect(
            ("editor.frame-body", frame_id.raw()),
            body,
            |ui| match active_panel.as_ref().map(|panel| panel.id) {
                Some(PANEL_SCENE) => self.scene_graph(ui, body),
                Some(PANEL_ASSETS) => self.assets_browser(ui, body),
                Some(PANEL_VIEWPORT) => self.viewport_panel(ui, body),
                Some(PANEL_CONSOLE) => Self::console_panel(ui, body),
                Some(PANEL_TIMELINE) => Self::timeline_panel(ui, body),
                Some(PANEL_INSPECTOR) => self.inspector(ui, body),
                Some(PANEL_NODE_GRAPH) => Self::node_graph_panel(ui, body),
                _ => {}
            },
        );
    }

    pub(super) fn frame_tab_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        frame_id: FrameId,
        frame_rect: Rect,
        tab_height: f32,
        frame: &Frame,
        tab_drags: &mut Vec<(WidgetId, DockTabDrag)>,
    ) {
        let tab_strip = frame_tab_strip(frame);
        for (index, (tab, tab_rect)) in frame_tab_rects(frame, frame_rect, tab_height)
            .into_iter()
            .enumerate()
        {
            let response = ui.draggable(
                ("editor.frame-tab.drag", frame_id.raw(), tab.panel.raw()),
                tab_rect,
                false,
            );
            if let Some(target) = tab_strip.drag_target_by_index(index)
                && let Some(drag) = self.dock.begin_tab_drag(frame_id, target.panel)
            {
                tab_drags.push((response.id, drag));
            }
            if response.clicked
                && let Some(target) = tab_strip.activation_target_by_index(index)
            {
                self.dock.select_panel(frame_id, target.panel);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            if response.dragged && tab.draggable {
                self.status = format!("Dragging {} tab", tab.title);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    pub(super) fn frame_drop_targets(
        &mut self,
        ui: &mut Ui<'_>,
        frame_layouts: &[FrameLayout],
        tab_drags: &[(WidgetId, DockTabDrag)],
    ) {
        let Some(pointer) = ui.input().pointer.position else {
            return;
        };
        for layout in frame_layouts {
            let frame_rect = layout.rect.inset(2.0);
            let drop = ui.drop_target(
                ("editor.frame.drop-target", layout.frame.raw()),
                frame_rect,
                false,
            );
            let Some(source) = drop.source else {
                continue;
            };
            let Some((_, drag)) = tab_drags.iter().find(|(drag_id, _)| *drag_id == source) else {
                continue;
            };
            let Some(target) = self.dock_drop_target(layout.frame, frame_rect, pointer) else {
                continue;
            };

            if drop.dropped {
                if self.dock.drop_tab(*drag, target) {
                    if matches!(target, DockDropTarget::Split { .. }) {
                        self.next_drop_frame += 1;
                    }
                    self.status = dock_drop_status(target);
                    ui.request_repaint(RepaintRequest::NextFrame);
                }
                return;
            }

            if drop.response.state.hovered {
                draw_dock_drop_affordance(ui, frame_rect, target);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    pub(super) fn dock_drop_target(
        &self,
        frame: FrameId,
        frame_rect: Rect,
        pointer: Point,
    ) -> Option<DockDropTarget> {
        let zone = resolve_frame_drop_zone_with_policy(
            frame_rect,
            pointer,
            editor_dock_interaction_policy(),
        )?;
        Some(match zone {
            DockDropZone::Center => DockDropTarget::tab(frame),
            DockDropZone::Left => DockDropTarget::split(
                frame,
                DockPlacement::Left,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Right => DockDropTarget::split(
                frame,
                DockPlacement::Right,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Top => DockDropTarget::split(
                frame,
                DockPlacement::Top,
                FrameId::from_raw(self.next_drop_frame),
            ),
            DockDropZone::Bottom => DockDropTarget::split(
                frame,
                DockPlacement::Bottom,
                FrameId::from_raw(self.next_drop_frame),
            ),
        })
    }
}
