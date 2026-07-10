impl EditorShowcase {
    /// Creates an editor showcase.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports whether the editor is currently in play mode.
    #[must_use]
    pub(crate) const fn is_running(&self) -> bool {
        self.running
    }

    /// Applies an editor-owned state transition for an action ID.
    pub fn apply_action(&mut self, action_id: &str) -> bool {
        match action_id {
            ACTION_PLAY => {
                self.running = true;
                "Play mode running".clone_into(&mut self.status);
                true
            }
            ACTION_STOP => {
                self.running = false;
                self.timeline = 0.0;
                "Play mode stopped".clone_into(&mut self.status);
                true
            }
            ACTION_GRID => {
                self.grid_visible = !self.grid_visible;
                let status = if self.grid_visible {
                    "Viewport grid enabled"
                } else {
                    "Viewport grid hidden"
                };
                status.clone_into(&mut self.status);
                true
            }
            ACTION_TOOL_SELECT => self.select_tool(EditorTool::Select),
            ACTION_TOOL_MOVE => self.select_tool(EditorTool::Move),
            ACTION_TOOL_ROTATE => self.select_tool(EditorTool::Rotate),
            ACTION_TOOL_SCALE => self.select_tool(EditorTool::Scale),
            _ => panel_type_for_open_action(action_id)
                .is_some_and(|panel_type| self.open_or_focus_panel(panel_type)),
        }
    }

    /// Renders the editor and returns application action invocations.
    pub fn render(&mut self, ui: &mut Ui<'_>, action_count: u32) -> Vec<EditorInvocation> {
        let viewport = Rect::new(
            0.0,
            0.0,
            ui.viewport().logical_size.width,
            ui.viewport().logical_size.height,
        );
        let mut invocations = Vec::new();
        Self::background(ui, viewport);
        self.dismiss_menu_for_input(ui, viewport);
        self.tool_bar(ui, viewport, &mut invocations);
        self.menu_bar(ui, viewport);
        self.workspace(ui, viewport);
        self.menu_overlay(ui, viewport, &mut invocations);
        let _modal_metadata = self.about_modal_overlay_model(viewport);
        self.status_bar(ui, viewport, action_count + invocations.len() as u32);
        invocations
    }

    pub(super) fn select_tool(&mut self, tool: EditorTool) -> bool {
        self.selected_tool = tool;
        let status = match tool {
            EditorTool::Select => "Select tool active",
            EditorTool::Move => "Move tool active",
            EditorTool::Rotate => "Rotate tool active",
            EditorTool::Scale => "Scale tool active",
        };
        status.clone_into(&mut self.status);
        true
    }

    pub(super) fn trigger(
        &mut self,
        invocations: &mut Vec<EditorInvocation>,
        action_id: &'static str,
        source: ActionSource,
    ) {
        if self.apply_action(action_id) {
            invocations.push(ActionInvocation::new(
                ActionId::new(action_id),
                source,
                ActionContext::Editor,
            ));
        }
    }

    pub(super) fn menu_bar_model(&self) -> MenuBar {
        let mut menu_bar =
            MenuBar::from_menus(menu_header_rects().into_iter().map(|(kind, label, _)| {
                MenuBarMenu::new(kind.menu_bar_id(), label, self.menu_model(kind))
            }));
        if let Some(kind) = self.open_menu {
            menu_bar.open(kind.menu_bar_id());
        }
        menu_bar
    }

    pub(super) fn toolbar_model(&self) -> Toolbar {
        let tool_items = EDITOR_TOOL_BUTTONS
            .into_iter()
            .map(|(tool, icon, label, action)| {
                ToolbarItem::new(toolbar_action(
                    action,
                    label,
                    icon,
                    Some(self.selected_tool == tool),
                    true,
                ))
                .with_presentation(ToolbarItemPresentation::IconOnly)
            });

        let viewport_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_GRID,
                "Toggle grid",
                ToolbarIcon::Grid,
                Some(self.grid_visible),
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_VIEWPORT_FIT_SELECTION,
                "Frame selected (Experimental)",
                ToolbarIcon::Crosshair,
                None,
                false,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_VIEWPORT_FIT_CONTENT,
                "Reset view (Experimental)",
                ToolbarIcon::Reset,
                None,
                false,
            )),
        ];

        let dock_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_DOCK_JOIN,
                "Join dock splitter",
                ToolbarIcon::Component,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_DOCK_SWAP,
                "Swap dock frames",
                ToolbarIcon::Layers,
                None,
                true,
            )),
        ];

        let run_items = [
            ToolbarItem::new(toolbar_action(
                ACTION_PLAY,
                "Play",
                ToolbarIcon::Play,
                Some(self.running),
                !self.running,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_PAUSE,
                "Pause (Experimental)",
                ToolbarIcon::Pause,
                None,
                false,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_STOP,
                "Stop",
                ToolbarIcon::Stop,
                None,
                self.running || self.timeline > 0.0,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_BUILD,
                "Build (Experimental)",
                ToolbarIcon::Rocket,
                None,
                false,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_EXPORT,
                "Export (Experimental)",
                ToolbarIcon::Download,
                None,
                false,
            )),
        ];

        Toolbar::from_groups([
            ToolbarGroup::new(EditorToolbarGroupKind::Tools.id(), "Tools", tool_items),
            ToolbarGroup::new(
                EditorToolbarGroupKind::Viewport.id(),
                "Viewport",
                viewport_items,
            ),
            ToolbarGroup::new(EditorToolbarGroupKind::Dock.id(), "Dock", dock_items),
            ToolbarGroup::new(EditorToolbarGroupKind::Run.id(), "Run", run_items),
        ])
    }

    pub(super) fn status_bar_model(&self, action_count: u32) -> StatusBar {
        let jobs = Self::showcase_job_list();
        let job_summary = jobs.summary();
        let job_progress = jobs.active_status_progress();
        let diagnostics = Self::showcase_diagnostics();
        let diagnostic_summary = diagnostics.summary();
        let feedback = Self::showcase_feedback_stack();
        let active_feedback = feedback.active_items(showcase_feedback_now());

        StatusBar::from_items([
            StatusItem::new(
                EditorStatusItemKind::Message.id(),
                "Status",
                self.status.clone(),
                StatusItemKind::Message,
            ),
            StatusItem::new(
                EditorStatusItemKind::Actions.id(),
                "Actions",
                format!("Actions: {action_count}"),
                StatusItemKind::ActionCount,
            )
            .with_count(action_count),
            StatusItem::new(
                EditorStatusItemKind::Snap.id(),
                "Snap",
                if self.snap_enabled {
                    "Snap 1m"
                } else {
                    "Snap off"
                },
                if self.snap_enabled {
                    StatusItemKind::Ready
                } else {
                    StatusItemKind::Stale
                },
            ),
            StatusItem::new(
                EditorStatusItemKind::Backend.id(),
                "Backend",
                "Vello / winit",
                StatusItemKind::Ready,
            ),
            StatusItem::new(
                EditorStatusItemKind::Jobs.id(),
                "Jobs",
                format!(
                    "Jobs: {} active / {} total",
                    job_summary.active(),
                    job_summary.total()
                ),
                StatusItemKind::JobCount,
            )
            .with_count(job_summary.active())
            .with_progress(job_progress.unwrap_or_else(|| StatusProgress::new(0.0))),
            StatusItem::new(
                EditorStatusItemKind::Diagnostics.id(),
                "Diagnostics",
                format!(
                    "Diagnostics: {}E {}W {}I",
                    diagnostic_summary.errors, diagnostic_summary.warnings, diagnostic_summary.info
                ),
                if diagnostic_summary.errors > 0 {
                    StatusItemKind::Error
                } else if diagnostic_summary.warnings > 0 {
                    StatusItemKind::Stale
                } else {
                    StatusItemKind::Ready
                },
            )
            .with_count(diagnostic_summary.total()),
            StatusItem::new(
                EditorStatusItemKind::Feedback.id(),
                "Feedback",
                format!("Feedback: {}", active_feedback.len()),
                StatusItemKind::Message,
            )
            .with_count(active_feedback.len() as u32),
            StatusItem::new(
                EditorStatusItemKind::Timeline.id(),
                "Timeline",
                format!("Timeline: {:.0}%", self.timeline * 100.0),
                StatusItemKind::Progress,
            )
            .with_progress(StatusProgress::new(self.timeline))
            .with_visible(false),
        ])
    }

    pub(super) fn about_modal_overlay_model(&self, viewport: Rect) -> ModalDialogOverlay {
        let _ = self;
        let dialog = ModalDialog::new(WidgetId::from_raw(40_001), "About Kinetik Forge")
            .with_body("Kinetik Forge editor showcase chrome is action-driven and data-only.")
            .with_actions([
                ModalAction::new(
                    modal_action(ACTION_DOCS, "Open Docs (Experimental)", false),
                    ModalActionRole::Primary,
                ),
                ModalAction::new(
                    modal_action(ACTION_ABOUT_CLOSE, "Close (Experimental)", false),
                    ModalActionRole::Cancel,
                ),
            ]);
        let size = Size::new(360.0, 168.0);
        let rect = Rect::new(
            viewport.x + (viewport.width - size.width).max(0.0) * 0.5,
            viewport.y + (viewport.height - size.height).max(0.0) * 0.5,
            size.width,
            size.height,
        );
        ModalDialogOverlay::placed(
            OverlayId::from_raw(30_001),
            rect,
            dialog,
            OverlayDismissal::OutsideClickOrEscape,
            ActionContext::Editor,
        )
    }

    pub(super) fn background(ui: &mut Ui<'_>, viewport: Rect) {
        rect(ui, viewport, rgb(20, 21, 23), None);
        rect(
            ui,
            Rect::new(0.0, 0.0, viewport.width, 28.0),
            rgb(32, 34, 37),
            None,
        );
        rect(
            ui,
            Rect::new(0.0, 28.0, viewport.width, 36.0),
            rgb(25, 26, 29),
            None,
        );
        rect(
            ui,
            Rect::new(0.0, 63.0, viewport.width, 1.0),
            rgb(55, 58, 64),
            None,
        );
    }

    pub(super) fn menu_bar(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        text(ui, 12.0, 18.0, "Kinetik Forge", 13.0, rgb(226, 229, 234));
        let menu_bar = self.menu_bar_model();
        for ((kind, _label, rect), menu) in
            menu_header_rects().into_iter().zip(menu_bar.menus().iter())
        {
            let response = ui.pressable(("editor.menu-header", kind), rect, false);
            let was_active = self.open_menu == Some(kind);
            if response.clicked {
                self.open_menu = if was_active { None } else { Some(kind) };
                ui.request_repaint(RepaintRequest::NextFrame);
            } else if self.open_menu.is_some()
                && response.state.hovered
                && self.open_menu != Some(kind)
            {
                self.open_menu = Some(kind);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
            let active = self.open_menu == Some(kind);
            debug_assert_eq!(menu.id, kind.menu_bar_id());
            if active || response.state.hovered {
                rect_fill(
                    ui,
                    rect,
                    if active {
                        rgb(44, 47, 52)
                    } else {
                        rgb(34, 36, 40)
                    },
                    Some(rgb(66, 70, 78)),
                    CornerRadius::all(0.0),
                );
            }
            text(
                ui,
                rect.x + 10.0,
                17.0,
                &menu.title,
                11.0,
                if active {
                    rgb(238, 240, 244)
                } else {
                    rgb(196, 200, 207)
                },
            );
        }

        let hint = if self.running {
            "Play Mode: Running"
        } else {
            "Play Mode: Edit"
        };
        text(
            ui,
            viewport.max_x() - 190.0,
            18.0,
            hint,
            11.0,
            if self.running {
                rgb(110, 205, 126)
            } else {
                rgb(170, 175, 182)
            },
        );
    }

    pub(super) fn dismiss_menu_for_input(&mut self, ui: &mut Ui<'_>, viewport: Rect) {
        let Some(kind) = self.open_menu else {
            return;
        };
        let escape_pressed = ui
            .input()
            .keyboard
            .events
            .iter()
            .any(|event| event.state == KeyState::Pressed && matches!(event.key, Key::Escape));
        let outside_activation = ui.input().pointer.position.filter(|point| {
            ui.input().pointer.primary.released && !menu_bar_rect().contains_point(*point)
        });
        let overlay = self.menu_overlay_model(kind, viewport);
        let mut stack = OverlayStack::new();
        overlay.open_in(&mut stack);
        if !stack
            .dismissal_requests(outside_activation, escape_pressed)
            .is_empty()
        {
            self.open_menu = None;
            ui.request_repaint(RepaintRequest::NextFrame);
        }
    }

}
