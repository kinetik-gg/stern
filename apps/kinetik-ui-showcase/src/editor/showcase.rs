use super::{
    ACTION_BUILD, ACTION_CANCEL_ACTIVE_FIXTURE_JOB, ACTION_CANCEL_QUEUED_FIXTURE_JOB,
    ACTION_DISMISS_FEEDBACK_REPORT, ACTION_DOCK_JOIN, ACTION_DOCK_SWAP, ACTION_GRID,
    ACTION_OPEN_FEEDBACK_REPORT, ACTION_PALETTE, ACTION_PLAY, ACTION_SAVE, ACTION_STOP,
    ACTION_TOOL_MOVE, ACTION_TOOL_ROTATE, ACTION_TOOL_SCALE, ACTION_TOOL_SELECT,
    ACTION_VIEWPORT_ACTUAL_SIZE, ACTION_VIEWPORT_FIT_CONTENT, ACTION_VIEWPORT_FIT_SELECTION,
    ACTION_VIEWPORT_FOCUS_SELECTED, ACTION_VIEWPORT_PAN, ACTION_VIEWPORT_ZOOM_IN,
    ACTION_VIEWPORT_ZOOM_OUT, ASSETS, ActionContext, ActionDescriptor, ActionId, ActionInvocation,
    ActionQueue, ActionSource, AssetSlotAsset, AssetSlotConfig, ClipId, CornerRadius,
    DENSE_ICON_SIZE, DiagnosticSource, DiagnosticStrip, DiagnosticStripItem,
    DiagnosticStripSeverity, DockDropTarget, DockDropZone, DockPlacement,
    DockSplitterContextActionKind, DockTabDrag, DropdownItem, DropdownItemId, DropdownModel,
    Duration, EDITOR_TOOL_BUTTONS, EdgeDescriptor, EdgeId, EditorChromeMetrics, EditorInvocation,
    EditorMenuKind, EditorShowcase, EditorStatusItemKind, EditorTool, EditorToolbarGroupKind,
    FeedbackAction, FeedbackDismiss, FeedbackItem, FeedbackKind, FeedbackStack, Frame, FrameId,
    FrameLayout, GraphPoint, GraphRect, GraphVector, GridColumns, GridLayout, Guide, JobCancel,
    JobList, JobPhase, JobProgress, JobRow, Key, KeyState, LOGS, ListLayout, Menu, MenuBar,
    MenuBarMenu, MenuBarOverlayRequest, MenuItem, MenuOverlay, ModalAction, ModalActionRole,
    ModalDialog, ModalDialogOverlay, NodeDescriptor, NodeFrameDescriptor, NodeFrameId,
    NodeGraphDescriptor, NodeGraphEdgeRoutePoint, NodeGraphEmissionError, NodeGraphPanZoom,
    NodeGraphSelection, NodeGraphSelectionTarget, NodeGraphStaticOutput, NodeGraphStaticView,
    NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId, NumericScrubInputConfig,
    OverlayDismissal, OverlayId, OverlayKind, OverlayStack, PANEL_ASSETS, PANEL_CONSOLE,
    PANEL_INSPECTOR, PANEL_NODE_GRAPH, PANEL_SCENE, PANEL_TIMELINE, PANEL_VIEWPORT, PanZoomExt,
    PanelId, PanelOpenDecision, PanelTypeId, PanelWorkspaceContext, PathFieldConfig, Point,
    PopoverPlacement, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
    PropertyGridAffordanceLayout, PropertyGridLayout, PropertyGridRow, Rect, RectExt,
    RepaintRequest, RerouteDescriptor, RerouteId, SelectFieldConfig, SemanticNode, SemanticRole,
    Size, StatusBar, StatusItem, StatusItemKind, StatusProgress, TOOLBAR_Y, TableColumn,
    TableLayout, Toolbar, ToolbarGroup, ToolbarIcon, ToolbarItem, ToolbarItemPresentation,
    TreeLayout, Ui, VIEWPORT_SIZE, VIEWPORT_TEXTURE, Vec2, VectorScrubInputConfig,
    ViewportComposition, ViewportSurface, WidgetId, ctrl_char, diagnostic_item_id,
    dock_drop_status, draw_dock_drop_affordance, draw_icon, editor_dock_chrome_style,
    editor_dock_interaction_policy, editor_open_panel_metadata, editor_panel_instances,
    editor_panel_registry, editor_workspace_rect, feedback_id, feedback_kind_color,
    feedback_kind_label, frame_tab_rects, frame_tab_strip, inspector_label_width,
    inspector_numeric_scrub, inspector_rows, inspector_value_label, item_id, job_phase_color,
    job_phase_label, job_row_id, line, log_color, menu, menu_action,
    menu_action_from_panel_metadata, menu_anchor, menu_bar_rect, menu_header_rects, menu_size,
    modal_action, paint_toolbar_icon_button_sized, panel_category_label,
    panel_type_for_open_action, property_grid_row_affordance_rects, rect, rect_fill,
    resolve_dock_splitter_context_actions_with_policy, resolve_frame_drop_zone_with_policy, rgb,
    rgba, run_toolbar_buttons, scene_icon, scene_label, scene_model, severity_color,
    severity_label, shortcut, shortcut_label, showcase_feedback_now, solve_dock_layout,
    solve_dock_splitters_with_style, status_item_text_width, text, toolbar_action,
    toolbar_icon_button, toolbar_icon_button_sized,
};

impl EditorShowcase {
    /// Creates an editor showcase.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies an editor-owned state transition for an action ID.
    pub fn apply_action(&mut self, action_id: &str) -> bool {
        match action_id {
            ACTION_SAVE => {
                "Saved project snapshot".clone_into(&mut self.status);
                true
            }
            ACTION_PLAY => {
                self.running = !self.running;
                let status = if self.running {
                    "Play mode running"
                } else {
                    "Play mode paused"
                };
                status.clone_into(&mut self.status);
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
            ACTION_VIEWPORT_FOCUS_SELECTED => {
                "Viewport focus selected requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_FIT_CONTENT => {
                "Viewport fit content requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_FIT_SELECTION => {
                "Viewport fit selection requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ACTUAL_SIZE => {
                "Viewport actual size requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ZOOM_IN => {
                "Viewport zoom in requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_ZOOM_OUT => {
                "Viewport zoom out requested".clone_into(&mut self.status);
                true
            }
            ACTION_VIEWPORT_PAN => {
                "Viewport pan mode requested".clone_into(&mut self.status);
                true
            }
            ACTION_BUILD => {
                "Build queued for Windows x64".clone_into(&mut self.status);
                true
            }
            ACTION_PALETTE => {
                "Command palette requested".clone_into(&mut self.status);
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
                "Frame selected",
                ToolbarIcon::Crosshair,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_VIEWPORT_FIT_CONTENT,
                "Reset view",
                ToolbarIcon::Reset,
                None,
                true,
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
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_PLAY,
                "Pause",
                ToolbarIcon::Pause,
                Some(!self.running),
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_STOP,
                "Stop",
                ToolbarIcon::Stop,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_BUILD,
                "Build",
                ToolbarIcon::Rocket,
                None,
                true,
            )),
            ToolbarItem::new(toolbar_action(
                ACTION_BUILD,
                "Export",
                ToolbarIcon::Download,
                None,
                true,
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
                    modal_action(ACTION_PALETTE, "Open Docs", true),
                    ModalActionRole::Primary,
                ),
                ModalAction::new(
                    modal_action(ACTION_PALETTE, "Close", true),
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

    pub(super) fn menu_overlay(
        &mut self,
        ui: &mut Ui<'_>,
        viewport: Rect,
        invocations: &mut Vec<EditorInvocation>,
    ) {
        let Some(kind) = self.open_menu else {
            return;
        };
        let overlay = self.menu_overlay_model(kind, viewport);
        if self.menu_overlay_interactions(ui, kind, &overlay, invocations) {
            return;
        }
        let visible_items = overlay.visible_items();
        rect_fill(
            ui,
            overlay.entry.rect.translate(Vec2::new(0.0, 2.0)),
            rgb(0, 0, 0),
            None,
            CornerRadius::all(0.0),
        );
        rect_fill(
            ui,
            overlay.entry.rect,
            rgb(28, 30, 33),
            Some(rgb(74, 78, 86)),
            CornerRadius::all(0.0),
        );

        let mut y = overlay.entry.rect.y + 6.0;
        for (index, item) in visible_items.into_iter().enumerate() {
            match item {
                MenuItem::Label(label) => {
                    text(
                        ui,
                        overlay.entry.rect.x + 10.0,
                        y + 15.0,
                        label,
                        10.0,
                        rgb(145, 150, 158),
                    );
                    y += 22.0;
                }
                MenuItem::Separator => {
                    rect(
                        ui,
                        Rect::new(
                            overlay.entry.rect.x + 8.0,
                            y + 4.0,
                            overlay.entry.rect.width - 16.0,
                            1.0,
                        ),
                        rgb(60, 63, 70),
                        None,
                    );
                    y += 9.0;
                }
                MenuItem::Action(action) => {
                    let row = Rect::new(
                        overlay.entry.rect.x + 4.0,
                        y,
                        overlay.entry.rect.width - 8.0,
                        24.0,
                    );
                    let enabled = action.can_invoke();
                    let response = ui.pressable(
                        ("editor.menu-row", kind, index, action.id.as_str()),
                        row,
                        !enabled,
                    );
                    if response.state.hovered && enabled {
                        rect_fill(ui, row, rgb(43, 78, 132), None, CornerRadius::all(0.0));
                    }
                    if action.state.is_checked() {
                        rect(
                            ui,
                            Rect::new(row.x + 8.0, row.y + 8.0, 8.0, 8.0),
                            rgb(45, 110, 230),
                            None,
                        );
                    }
                    text(
                        ui,
                        row.x + 24.0,
                        row.y + 16.0,
                        &action.label,
                        11.0,
                        if enabled {
                            rgb(224, 227, 232)
                        } else {
                            rgb(112, 117, 126)
                        },
                    );
                    if let Some(shortcut) = action.shortcut.as_ref() {
                        let shortcut = shortcut_label(shortcut);
                        text(
                            ui,
                            row.max_x() - 74.0,
                            row.y + 16.0,
                            &shortcut,
                            10.0,
                            if enabled {
                                rgb(145, 151, 160)
                            } else {
                                rgb(86, 90, 98)
                            },
                        );
                    }
                    y += 24.0;
                }
            }
        }
    }

    pub(super) fn menu_overlay_interactions(
        &mut self,
        ui: &mut Ui<'_>,
        kind: EditorMenuKind,
        overlay: &MenuOverlay,
        invocations: &mut Vec<EditorInvocation>,
    ) -> bool {
        let mut y = overlay.entry.rect.y + 6.0;
        for (index, item) in overlay.visible_items().into_iter().enumerate() {
            match item {
                MenuItem::Label(_) => {
                    y += 22.0;
                }
                MenuItem::Separator => {
                    y += 9.0;
                }
                MenuItem::Action(action) => {
                    let row = Rect::new(
                        overlay.entry.rect.x + 4.0,
                        y,
                        overlay.entry.rect.width - 8.0,
                        24.0,
                    );
                    let enabled = action.can_invoke();
                    let response = ui.pressable(
                        ("editor.menu-row.prepass", kind, index, action.id.as_str()),
                        row,
                        !enabled,
                    );
                    if response.clicked && enabled {
                        let mut queue = ActionQueue::new();
                        if overlay.invoke_visible(index, &mut queue) {
                            self.handle_action_queue(invocations, &mut queue);
                            self.open_menu = None;
                            ui.request_repaint(RepaintRequest::NextFrame);
                            return true;
                        }
                    }
                    y += 24.0;
                }
            }
        }
        false
    }

    pub(super) fn menu_overlay_model(&self, kind: EditorMenuKind, viewport: Rect) -> MenuOverlay {
        let mut menu_bar = self.menu_bar_model();
        menu_bar.open(kind.menu_bar_id());
        menu_bar
            .active_overlay(MenuBarOverlayRequest {
                overlay_id: OverlayId::from_raw(10_000 + kind.raw()),
                kind: OverlayKind::Menu,
                anchor: menu_anchor(kind),
                size: menu_size(kind),
                placement: PopoverPlacement::Below,
                offset: 2.0,
                fit_viewport: true,
                viewport,
                dismissal: OverlayDismissal::OutsideClickOrEscape,
                source: ActionSource::Menu,
                context: ActionContext::Editor,
            })
            .expect("editor menu-bar active menu should convert to overlay")
    }

    pub(super) fn menu_model(&self, kind: EditorMenuKind) -> Menu {
        match kind {
            EditorMenuKind::File => menu([
                menu_action(
                    ACTION_PALETTE,
                    "New Scene",
                    Some(ctrl_char("n")),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Open Project...",
                    Some(ctrl_char("o")),
                    None,
                    true,
                ),
                menu_action(ACTION_SAVE, "Save Scene", Some(ctrl_char("s")), None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Import Asset...", None, None, true),
                menu_action(
                    ACTION_BUILD,
                    "Export Build",
                    Some(ctrl_char("b")),
                    None,
                    true,
                ),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Quit", None, None, false),
            ]),
            EditorMenuKind::Edit => menu([
                menu_action(ACTION_PALETTE, "Undo", Some(ctrl_char("z")), None, false),
                menu_action(ACTION_PALETTE, "Redo", Some(ctrl_char("y")), None, false),
                MenuItem::Separator,
                menu_action(
                    ACTION_PALETTE,
                    "Duplicate",
                    Some(ctrl_char("d")),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Delete",
                    Some(shortcut(Key::Delete)),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_PALETTE,
                    "Preferences",
                    Some(ctrl_char(",")),
                    None,
                    true,
                ),
            ]),
            EditorMenuKind::View => menu([
                menu_action(ACTION_PALETTE, "Perspective View", None, Some(true), true),
                menu_action(
                    ACTION_VIEWPORT_FOCUS_SELECTED,
                    "Frame Selected",
                    Some(shortcut(Key::Character("f".to_owned()))),
                    None,
                    true,
                ),
                menu_action(
                    ACTION_GRID,
                    "Show Grid",
                    Some(shortcut(Key::Character("g".to_owned()))),
                    Some(self.grid_visible),
                    true,
                ),
                menu_action(ACTION_PALETTE, "Show Overlays", None, Some(true), true),
                menu_action(ACTION_VIEWPORT_FIT_CONTENT, "Reset View", None, None, true),
            ]),
            EditorMenuKind::Project => menu([
                menu_action(
                    ACTION_PLAY,
                    "Play",
                    Some(shortcut(Key::Function(5))),
                    None,
                    true,
                ),
                menu_action(ACTION_STOP, "Stop", None, None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "Project Settings...", None, None, true),
            ]),
            EditorMenuKind::Build => menu([
                menu_action(
                    ACTION_BUILD,
                    "Build Project",
                    Some(ctrl_char("b")),
                    None,
                    true,
                ),
                menu_action(ACTION_BUILD, "Package Windows x64", None, None, true),
                menu_action(ACTION_PALETTE, "Run Profiler", None, None, false),
            ]),
            EditorMenuKind::Window => self.window_menu_model(),
            EditorMenuKind::Help => menu([
                menu_action(
                    ACTION_PALETTE,
                    "Online Docs",
                    Some(shortcut(Key::Function(1))),
                    None,
                    true,
                ),
                menu_action(ACTION_PALETTE, "Keyboard Shortcuts", None, None, true),
                MenuItem::Separator,
                menu_action(ACTION_PALETTE, "About Kinetik Forge", None, None, true),
            ]),
        }
    }

    pub(super) fn window_menu_model(&self) -> Menu {
        let registry = editor_panel_registry();
        let open_metadata = editor_open_panel_metadata();
        let mut menu = Menu::new();
        menu.push(menu_action(
            ACTION_PALETTE,
            "Command Palette",
            Some(ctrl_char("p")),
            None,
            true,
        ));
        menu.push(MenuItem::Separator);

        for category in registry.categories() {
            menu.push(MenuItem::Label(panel_category_label(category).to_owned()));
            for metadata in open_metadata
                .iter()
                .filter(|metadata| &metadata.category == category)
            {
                menu.push(menu_action_from_panel_metadata(
                    metadata,
                    self.is_panel_type_open(metadata.panel_type),
                ));
            }
        }

        menu
    }

    pub(super) fn is_panel_type_open(&self, panel_type: PanelTypeId) -> bool {
        let instances = editor_panel_instances();
        instances.iter().any(|instance| {
            let panel = PanelId::from_instance_id(instance.id);
            instance.panel_type == panel_type
                && self
                    .dock
                    .frames()
                    .iter()
                    .any(|frame| frame.panels.iter().any(|item| item.id == panel))
        })
    }

    pub(super) fn open_or_focus_panel(&mut self, panel_type: PanelTypeId) -> bool {
        let registry = editor_panel_registry();
        let instances = editor_panel_instances();
        let Some(decision) = registry.resolve_open_decision(
            panel_type,
            &instances,
            &self.dock,
            PanelWorkspaceContext::Docked,
        ) else {
            "Panel open request unavailable".clone_into(&mut self.status);
            return false;
        };

        match decision {
            PanelOpenDecision::FocusExisting(request) => {
                if self
                    .dock
                    .select_panel(request.target.frame, request.target.panel)
                {
                    self.status = format!("Focused {}", request.metadata.title);
                    true
                } else {
                    self.status = format!("Could not focus {}", request.metadata.title);
                    false
                }
            }
            PanelOpenDecision::OpenNew(request) => {
                self.status = format!("Open {} requested", request.metadata.title);
                true
            }
        }
    }

    pub(super) fn handle_action_queue(
        &mut self,
        invocations: &mut Vec<EditorInvocation>,
        queue: &mut ActionQueue,
    ) {
        for invocation in queue.drain() {
            if self.apply_action(invocation.action_id.as_str()) {
                invocations.push(invocation);
            }
        }
    }

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

    pub(super) fn scene_graph(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        let add = toolbar_icon_button_sized(
            ui,
            "editor.scene.add",
            header.with_width(28.0),
            ToolbarIcon::Plus,
            "Add node",
            false,
            false,
            DENSE_ICON_SIZE,
        );
        if add.clicked {
            "Create node requested".clone_into(&mut self.status);
            ui.request_repaint(RepaintRequest::NextFrame);
        }
        text(
            ui,
            header.x + 36.0,
            header.y + 17.0,
            "Scene",
            13.0,
            rgb(222, 225, 230),
        );
        draw_icon(
            ui,
            header.right_strip(24.0),
            ToolbarIcon::Dots,
            DENSE_ICON_SIZE,
        );

        let rows = scene_model().visible_rows(&self.scene_expansion);
        let layout = TreeLayout::new(22.0, 15.0);
        let content_height = layout.content_height(rows.len()) + 8.0;
        let scroll = Rect::new(
            body.x + 6.0,
            body.y + 38.0,
            body.width - 12.0,
            body.height - 44.0,
        );
        ui.scroll_area(
            "editor.scene.scroll",
            scroll,
            Size::new(scroll.width, content_height.max(scroll.height)),
            false,
            |ui, _| {
                for row_rect in layout.visible_row_rects(scroll, &rows, 0.0, 2) {
                    let row = row_rect.row;
                    let twisty = Rect::new(
                        row_rect.content_rect.x + 3.0,
                        row_rect.rect.y + 5.0,
                        12.0,
                        12.0,
                    );
                    let twist = row.has_children.then(|| {
                        ui.pressable(("editor.scene.expand", row.id.raw()), twisty, false)
                    });
                    let response = ui.list_row_value(
                        ("editor.scene.row", row.id.raw()),
                        row_rect.rect,
                        "",
                        &mut self.selected_node,
                        row.id,
                        false,
                    );
                    if response.clicked {
                        self.status = format!("Selected {}", scene_label(row.id));
                    }
                    if let Some(twist) = twist {
                        let mut expanded = row.expanded;
                        if twist.clicked {
                            self.scene_expansion.toggle(row.id);
                            expanded = !expanded;
                            ui.request_repaint(RepaintRequest::NextFrame);
                        }
                        text(
                            ui,
                            twisty.x + 2.0,
                            twisty.y + 10.0,
                            if expanded { "v" } else { ">" },
                            11.0,
                            rgb(176, 181, 188),
                        );
                    }
                    draw_icon(
                        ui,
                        Rect::new(
                            row_rect.content_rect.x + 17.0,
                            row_rect.rect.y + 3.0,
                            18.0,
                            18.0,
                        ),
                        scene_icon(row.id),
                        DENSE_ICON_SIZE,
                    );
                    text(
                        ui,
                        row_rect.content_rect.x + 38.0,
                        row_rect.rect.y + 15.0,
                        scene_label(row.id),
                        12.0,
                        rgb(218, 221, 226),
                    );
                }
            },
        );
    }

    pub(super) fn assets_browser(&mut self, ui: &mut Ui<'_>, body: Rect) {
        let chrome = EditorChromeMetrics::from_theme(ui.theme());
        rect(ui, body, rgb(24, 25, 27), None);
        let search = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 26.0);
        ui.search_field(
            "editor.assets.search",
            search,
            &mut self.asset_filter,
            false,
        );
        draw_icon(
            ui,
            Rect::new(search.x + 5.0, search.y + 5.0, 18.0, 18.0),
            ToolbarIcon::Search,
            chrome.dense_icon,
        );

        let grid_bounds = Rect::new(
            body.x + 8.0,
            body.y + 44.0,
            body.width - 16.0,
            body.height - 50.0,
        );
        let layout = GridLayout {
            columns: GridColumns::Adaptive { min_width: 92.0 },
            item_size: Size::new(88.0, 74.0),
            gap: 6.0,
        };
        let content_rows = (ASSETS.len() as f32 / layout.column_count(grid_bounds) as f32).ceil();
        ui.scroll_area(
            "editor.assets.scroll",
            grid_bounds,
            Size::new(
                grid_bounds.width,
                (content_rows * 80.0).max(grid_bounds.height),
            ),
            false,
            |ui, _| {
                for item in layout.item_rects(grid_bounds, ASSETS.len(), 0..ASSETS.len()) {
                    let asset = &ASSETS[item.index];
                    let response = ui.selectable_value(
                        ("editor.asset", item.index),
                        item.rect,
                        &mut self.selected_asset,
                        item.index,
                        false,
                    );
                    let selected = response.state.selected;
                    rect(
                        ui,
                        item.rect,
                        if selected {
                            rgb(38, 74, 122)
                        } else if response.state.hovered {
                            rgb(38, 40, 44)
                        } else {
                            rgb(31, 32, 35)
                        },
                        Some(if selected {
                            rgb(82, 140, 220)
                        } else {
                            rgb(53, 55, 61)
                        }),
                    );
                    if response.clicked {
                        self.status = format!("Asset selected: {}", asset.name);
                        ui.request_repaint(RepaintRequest::NextFrame);
                    }
                    draw_icon(
                        ui,
                        Rect::new(
                            item.rect.x + 8.0,
                            item.rect.y + 8.0,
                            chrome.asset_icon,
                            chrome.asset_icon,
                        ),
                        asset.icon,
                        chrome.asset_icon,
                    );
                    text(
                        ui,
                        item.rect.x + 8.0,
                        item.rect.y + 48.0,
                        asset.name,
                        11.0,
                        rgb(224, 226, 230),
                    );
                    text(
                        ui,
                        item.rect.x + 8.0,
                        item.rect.y + 64.0,
                        asset.kind,
                        9.0,
                        rgb(144, 149, 156),
                    );
                }
            },
        );
    }

    pub(super) fn viewport_panel(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(15, 16, 18), None);
        let toolbar = Rect::new(body.x, body.y, body.width, 28.0);
        rect(ui, toolbar, rgb(25, 26, 29), Some(rgb(44, 47, 53)));
        text(
            ui,
            toolbar.x + 10.0,
            toolbar.y + 18.0,
            "Perspective",
            12.0,
            rgb(220, 224, 230),
        );
        text(
            ui,
            toolbar.x + 94.0,
            toolbar.y + 18.0,
            "Lit",
            12.0,
            rgb(151, 158, 166),
        );
        text(
            ui,
            toolbar.x + 136.0,
            toolbar.y + 18.0,
            "1280 x 720",
            11.0,
            rgb(151, 158, 166),
        );

        let surface_bounds = Rect::new(
            body.x + 8.0,
            body.y + 36.0,
            (body.width - 16.0).max(1.0),
            (body.height - 66.0).max(1.0),
        );
        let viewport_semantic_id = ui.id("editor.viewport.surface.semantic");
        ui.push_semantic_node(
            SemanticNode::new(viewport_semantic_id, SemanticRole::Viewport, surface_bounds)
                .with_label("Viewport Surface")
                .focusable(true),
        );
        let drag = ui.draggable("editor.viewport.surface", surface_bounds, false);
        if drag.dragged {
            self.viewport_pan_zoom.pan_by(drag.drag_delta);
            ui.request_repaint(RepaintRequest::NextFrame);
        }
        if drag.state.hovered {
            let wheel = ui.input().pointer.wheel_delta.y;
            if wheel.abs() > f32::EPSILON {
                let current = self.viewport_pan_zoom.content_zoom();
                let next = (current + (-wheel * 0.001)).clamp(0.25, 2.5);
                self.viewport_pan_zoom.set_zoom(next);
                self.status = format!("Viewport zoom {:.0}%", next * 100.0);
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
        let surface = ViewportSurface {
            texture: VIEWPORT_TEXTURE,
            source_size: VIEWPORT_SIZE,
            bounds: surface_bounds,
            pan_zoom: self.viewport_pan_zoom,
        };
        let mut guides = Vec::new();
        if self.grid_visible {
            guides.extend([
                Guide::Vertical(VIEWPORT_SIZE.width * 0.25),
                Guide::Vertical(VIEWPORT_SIZE.width * 0.5),
                Guide::Vertical(VIEWPORT_SIZE.width * 0.75),
                Guide::Horizontal(VIEWPORT_SIZE.height * 0.5),
            ]);
        }
        let composition = ViewportComposition {
            surface,
            guides,
            crosshair: None,
            clip: ClipId::from_raw(8_001),
        };
        ui.extend(composition.primitives_at(ui.viewport().scale_factor));
        self.viewport_overlays(ui, surface, surface_bounds);

        let timeline = Rect::new(body.x + 10.0, body.max_y() - 24.0, body.width - 20.0, 14.0);
        ui.slider(
            "editor.timeline",
            timeline,
            &mut self.timeline,
            0.0..=1.0,
            false,
        );
    }

    pub(super) fn viewport_overlays(
        &self,
        ui: &mut Ui<'_>,
        surface: ViewportSurface,
        bounds: Rect,
    ) {
        if self.grid_visible {
            let content = surface.content_rect_at(ui.viewport().scale_factor);
            let step = (content.width / 8.0).max(1.0);
            for i in 1..8 {
                let x = content.x + step * i as f32;
                line(
                    ui,
                    Point::new(x, content.y),
                    Point::new(x, content.max_y()),
                    rgba(170, 190, 220, 0.20),
                    1.0,
                );
            }
            for i in 1..5 {
                let y = content.y + (content.height / 5.0) * i as f32;
                line(
                    ui,
                    Point::new(content.x, y),
                    Point::new(content.max_x(), y),
                    rgba(170, 190, 220, 0.18),
                    1.0,
                );
            }
        }

        if let Some(selection) = surface.content_rect_to_screen_at(
            Rect::new(720.0, 210.0, 210.0, 280.0),
            ui.viewport().scale_factor,
        ) {
            rect(
                ui,
                selection,
                rgba(78, 142, 245, 0.12),
                Some(rgb(82, 148, 245)),
            );
            line(
                ui,
                Point::new(selection.x + selection.width * 0.5, selection.y),
                Point::new(selection.x + selection.width * 0.5, selection.max_y()),
                rgba(120, 210, 255, 0.75),
                1.0,
            );
            line(
                ui,
                Point::new(selection.x, selection.y + selection.height * 0.5),
                Point::new(selection.max_x(), selection.y + selection.height * 0.5),
                rgba(120, 210, 255, 0.75),
                1.0,
            );
        }
        let gizmo = Rect::new(bounds.x + 18.0, bounds.max_y() - 72.0, 62.0, 52.0);
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 48.0, gizmo.max_y() - 10.0),
            rgb(236, 82, 82),
            2.0,
        );
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 10.0, gizmo.y + 8.0),
            rgb(78, 205, 112),
            2.0,
        );
        line(
            ui,
            Point::new(gizmo.x + 10.0, gizmo.max_y() - 10.0),
            Point::new(gizmo.x + 42.0, gizmo.y + 20.0),
            rgb(90, 140, 245),
            2.0,
        );
        text(
            ui,
            bounds.x + 16.0,
            bounds.y + 24.0,
            "CameraPreview",
            11.0,
            rgb(238, 240, 244),
        );
        text(
            ui,
            bounds.max_x() - 160.0,
            bounds.y + 24.0,
            "Frame 124 / 300",
            11.0,
            rgb(238, 240, 244),
        );
    }

    pub(super) fn inspector(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 34.0);
        rect(ui, header, rgb(37, 39, 43), Some(rgb(55, 58, 64)));
        draw_icon(
            ui,
            Rect::new(header.x + 7.0, header.y + 7.0, 20.0, 20.0),
            scene_icon(self.selected_node),
            DENSE_ICON_SIZE,
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 12.0,
            "Inspector",
            9.0,
            rgb(151, 158, 166),
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 27.0,
            scene_label(self.selected_node),
            12.0,
            rgb(231, 233, 237),
        );
        draw_icon(
            ui,
            Rect::new(header.max_x() - 27.0, header.y + 7.0, 20.0, 20.0),
            ToolbarIcon::Gear,
            DENSE_ICON_SIZE,
        );

        let rows = inspector_rows();
        let grid = Rect::new(
            body.x + 8.0,
            body.y + 52.0,
            body.width - 16.0,
            body.height - 60.0,
        );
        let layout =
            PropertyGridLayout::new(24.0, 26.0, inspector_label_width(grid.width), 6.0, 12.0);
        ui.scroll_area(
            "editor.inspector.scroll",
            grid,
            Size::new(grid.width, layout.content_height(&rows).max(grid.height)),
            false,
            |ui, _| {
                for row in layout.visible_row_rects(grid, &rows, 0.0, 2) {
                    match row.kind {
                        kinetik_ui::widgets::PropertyGridRowKind::Section => {
                            rect(ui, row.rect, rgb(31, 33, 36), Some(rgb(46, 49, 55)));
                            text(
                                ui,
                                row.label_rect.x + 8.0,
                                row.label_rect.y + 17.0,
                                &rows[row.index].label,
                                12.0,
                                rgb(205, 209, 216),
                            );
                        }
                        kinetik_ui::widgets::PropertyGridRowKind::Property { .. } => {
                            let model_row = &rows[row.index];
                            let status = model_row.state.status.presentation();
                            let label_color = match status.severity {
                                kinetik_ui::widgets::PropertyGridStatusSeverity::None => {
                                    rgb(154, 160, 168)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                    rgb(126, 179, 236)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                    rgb(232, 179, 90)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                    rgb(236, 96, 96)
                                }
                            };
                            rect(ui, row.rect, rgb(24, 25, 27), Some(rgb(38, 40, 45)));
                            if status.accented {
                                rect(
                                    ui,
                                    Rect::new(row.rect.x, row.rect.y, 3.0, row.rect.height),
                                    label_color,
                                    None,
                                );
                                text(
                                    ui,
                                    row.label_rect.max_x() - 10.0,
                                    row.label_rect.y + 16.0,
                                    match status.severity {
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                            "i"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                            "!"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                            "x"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::None => "",
                                    },
                                    9.0,
                                    label_color,
                                );
                            }
                            text(
                                ui,
                                row.label_rect.x + 6.0,
                                row.label_rect.y + 16.0,
                                &model_row.label,
                                11.0,
                                label_color,
                            );
                            let affordance_rects = property_grid_row_affordance_rects(
                                model_row,
                                row.value_rect.inset(2.0),
                                PropertyGridAffordanceLayout::default(),
                            );
                            self.inspector_value(ui, model_row, affordance_rects.value_rect);
                            let affordance = ui.property_grid_row_affordance_controls(
                                ("editor.inspector.affordance", row.id.raw()),
                                model_row,
                                affordance_rects,
                            );
                            if affordance.reset_requested {
                                self.status = format!("Reset requested for {}", model_row.label);
                            } else if affordance.keyframe_toggle_requested {
                                let state = if affordance.requested_keyed {
                                    "add"
                                } else {
                                    "remove"
                                };
                                self.status =
                                    format!("Keyframe {state} requested for {}", model_row.label);
                            }
                        }
                    }
                }
            },
        );
    }

    pub(super) fn inspector_value(
        &mut self,
        ui: &mut Ui<'_>,
        row: &PropertyGridRow,
        rect_value: Rect,
    ) {
        let id = row.id;
        let disabled = row.state.disabled;
        let read_only = row.state.read_only;
        match id.raw() {
            2 => {
                ui.vector3_scrub_input(
                    "editor.inspector.position",
                    rect_value,
                    "Position",
                    &mut self.position,
                    &mut self.position_states,
                    VectorScrubInputConfig::new(
                        NumericScrubInputConfig::new(0.1).with_fine_step(0.01),
                    )
                    .disabled(disabled)
                    .read_only(read_only),
                );
            }
            5 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.scale",
                    rect_value,
                    &mut self.scale,
                    NumericScrubInputConfig::new(0.01)
                        .with_fine_step(0.001)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            7 => {
                ui.slider(
                    "editor.inspector.exposure",
                    rect_value,
                    &mut self.exposure,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            8 => {
                ui.slider(
                    "editor.inspector.roughness",
                    rect_value,
                    &mut self.roughness,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            9 => {
                let asset = self.material_asset();
                let slot = ui.asset_slot_field(
                    "editor.inspector.material",
                    rect_value,
                    "Material",
                    Some(&asset),
                    AssetSlotConfig::new("Drop material")
                        .accepts_drop(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if slot.drop_received {
                    "Material drop requested".clone_into(&mut self.status);
                } else if slot.open_requested {
                    self.status = format!("Open material asset: {}", asset.label);
                } else if slot.pick_requested {
                    "Material asset picker requested".clone_into(&mut self.status);
                }
            }
            11 => {
                ui.toggle_value(
                    "editor.inspector.snap",
                    Rect::new(rect_value.x, rect_value.y + 2.0, 42.0, 18.0),
                    &mut self.snap_enabled,
                    disabled || read_only,
                );
            }
            13 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.mass",
                    rect_value,
                    &mut self.mass,
                    NumericScrubInputConfig::new(0.5)
                        .with_fine_step(0.1)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            14 => {
                let model = self.collider_model();
                let select = ui.select_field(
                    "editor.inspector.collider",
                    rect_value,
                    "Collider",
                    &model,
                    SelectFieldConfig::new("Choose collider")
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if select.open_requested {
                    "Collider choices requested".clone_into(&mut self.status);
                }
            }
            15 => {
                let path = ui.path_field(
                    "editor.inspector.script",
                    rect_value,
                    "Script path",
                    &mut self.script_path,
                    PathFieldConfig::default()
                        .open(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if path.browse_requested {
                    "Script path browse requested".clone_into(&mut self.status);
                } else if path.open_requested {
                    self.status = format!("Open script path: {}", self.script_path.text);
                }
            }
            _ => {
                text(
                    ui,
                    rect_value.x + 4.0,
                    rect_value.y + 15.0,
                    inspector_value_label(id),
                    11.0,
                    rgb(218, 221, 226),
                );
            }
        }
    }

    pub(super) fn material_asset(&self) -> AssetSlotAsset {
        let asset = &ASSETS[self.selected_asset.min(ASSETS.len().saturating_sub(1))];
        AssetSlotAsset::new(format!("asset://{}", asset.name), asset.name).with_kind(asset.kind)
    }

    pub(super) fn collider_model(&self) -> DropdownModel {
        let mut model = DropdownModel::from_items([
            DropdownItem::new(DropdownItemId::from_raw(1), "Box"),
            DropdownItem::new(DropdownItemId::from_raw(2), "Capsule"),
            DropdownItem::new(DropdownItemId::from_raw(3), "Sphere"),
            DropdownItem::new(DropdownItemId::from_raw(4), "Mesh").with_enabled(false),
        ]);
        let _ = model.set_selected_id(self.collider_kind);
        model
    }

    pub(super) fn showcase_job_list() -> JobList {
        JobList::from_rows([
            JobRow::new(job_row_id(1), "Active showcase job", JobPhase::Running)
                .with_progress(JobProgress::determinate(0.60))
                .with_detail("Deterministic fixture progress 3/5")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_ACTIVE_FIXTURE_JOB, "Cancel active job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(2), "Queued showcase job", JobPhase::Queued)
                .with_progress(JobProgress::determinate(0.20))
                .with_detail("Waiting in fixture queue")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_QUEUED_FIXTURE_JOB, "Cancel queued job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(3), "Completed showcase job", JobPhase::Succeeded)
                .with_progress(JobProgress::determinate(1.0))
                .with_detail("Finished fixture row"),
            JobRow::new(job_row_id(4), "Failed showcase job", JobPhase::Failed)
                .with_progress(JobProgress::determinate(0.80))
                .with_detail("Fixture failure for diagnostics presentation"),
        ])
    }

    pub(super) fn showcase_diagnostics() -> DiagnosticStrip {
        DiagnosticStrip::from_items([
            DiagnosticStripItem::new(
                diagnostic_item_id(1),
                DiagnosticStripSeverity::Warning,
                "showcase.fixture.warning",
                "Fixture warning keeps diagnostics visible",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("panel", "Console"),
            DiagnosticStripItem::new(
                diagnostic_item_id(2),
                DiagnosticStripSeverity::Info,
                "showcase.fixture.info",
                "Fixture metadata is application-owned",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("state", "deterministic"),
            DiagnosticStripItem::new(
                diagnostic_item_id(3),
                DiagnosticStripSeverity::Error,
                "showcase.fixture.error",
                "Fixture error demonstrates summary counts",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("recoverable", "true"),
        ])
    }

    pub(super) fn showcase_feedback_stack() -> FeedbackStack {
        FeedbackStack::from_items([
            FeedbackItem::timed(
                feedback_id(1),
                FeedbackKind::Success,
                "Saved",
                "Fixture save completed",
                Duration::from_secs(2),
                Duration::from_secs(8),
            )
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss feedback"),
                ActionContext::Editor,
            )),
            FeedbackItem::pinned(
                feedback_id(2),
                FeedbackKind::Warning,
                "Report",
                "Fixture report needs review",
            )
            .with_action(FeedbackAction::new(
                ActionDescriptor::new(ACTION_OPEN_FEEDBACK_REPORT, "Open report"),
                ActionContext::Editor,
            ))
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss report"),
                ActionContext::Editor,
            )),
            FeedbackItem::timed(
                feedback_id(3),
                FeedbackKind::Info,
                "Expired",
                "Expired fixture toast",
                Duration::from_secs(0),
                Duration::from_secs(2),
            ),
        ])
    }

    pub(super) fn console_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let diagnostics = Self::showcase_diagnostics();
        let jobs = Self::showcase_job_list();
        let feedback = Self::showcase_feedback_stack();
        let summary = diagnostics.summary();
        let active_feedback = feedback.active_items(showcase_feedback_now());
        let diagnostics_header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        text(
            ui,
            diagnostics_header.x,
            diagnostics_header.y + 16.0,
            &format!(
                "Diagnostics: {} error, {} warning, {} info",
                summary.errors, summary.warnings, summary.info
            ),
            12.0,
            rgb(222, 225, 230),
        );

        let diagnostics_layout = ListLayout::new(22.0);
        let diagnostic_rows = diagnostics.ordered_items();
        let diagnostics_bounds = Rect::new(
            body.x + 8.0,
            body.y + 36.0,
            body.width - 16.0,
            (diagnostic_rows.len() as f32 * 22.0).min(72.0),
        );
        for item in diagnostics_layout.row_rects(
            diagnostics_bounds,
            diagnostic_rows.len(),
            0..diagnostic_rows.len(),
        ) {
            let diagnostic = diagnostic_rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 15.0,
                severity_label(diagnostic.severity),
                10.0,
                severity_color(diagnostic.severity),
            );
            text(
                ui,
                item.rect.x + 76.0,
                item.rect.y + 15.0,
                &diagnostic.code,
                10.0,
                rgb(178, 183, 190),
            );
            text(
                ui,
                item.rect.x + 216.0,
                item.rect.y + 15.0,
                &diagnostic.message,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let jobs_y = diagnostics_bounds.max_y() + 12.0;
        let job_summary = jobs.summary();
        text(
            ui,
            body.x + 8.0,
            jobs_y + 16.0,
            &format!(
                "Jobs: {} active, {} complete, {} failed",
                job_summary.active(),
                job_summary.succeeded,
                job_summary.failed
            ),
            12.0,
            rgb(222, 225, 230),
        );
        let job_layout = ListLayout::new(24.0);
        let job_bounds = Rect::new(
            body.x + 8.0,
            jobs_y + 28.0,
            body.width - 16.0,
            (jobs.rows().len() as f32 * 24.0).min(96.0),
        );
        for item in job_layout.row_rects(job_bounds, jobs.rows().len(), 0..jobs.rows().len()) {
            let job = &jobs.rows()[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 16.0,
                job_phase_label(job.phase),
                10.0,
                job_phase_color(job.phase),
            );
            text(
                ui,
                item.rect.x + 86.0,
                item.rect.y + 16.0,
                &job.label,
                11.0,
                rgb(218, 221, 226),
            );
            if let Some(progress) = job.progress.status_progress() {
                let bar = Rect::new(item.rect.max_x() - 136.0, item.rect.y + 9.0, 72.0, 6.0);
                rect(ui, bar, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
                rect(
                    ui,
                    Rect::new(bar.x, bar.y, bar.width * progress.value, bar.height),
                    rgb(69, 123, 220),
                    None,
                );
                text(
                    ui,
                    bar.max_x() + 8.0,
                    item.rect.y + 16.0,
                    &format!("{:.0}%", progress.value * 100.0),
                    10.0,
                    rgb(154, 160, 168),
                );
            }
        }

        let feedback_y = job_bounds.max_y() + 12.0;
        text(
            ui,
            body.x + 8.0,
            feedback_y + 16.0,
            &format!("Feedback: {} active toast(s)", active_feedback.len()),
            12.0,
            rgb(222, 225, 230),
        );
        for (index, item) in active_feedback.iter().enumerate() {
            let row = Rect::new(
                body.x + 8.0,
                feedback_y + 28.0 + index as f32 * 22.0,
                body.width - 16.0,
                20.0,
            );
            rect(ui, row, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                row.x + 8.0,
                row.y + 14.0,
                feedback_kind_label(item.kind),
                10.0,
                feedback_kind_color(item.kind),
            );
            text(
                ui,
                row.x + 78.0,
                row.y + 14.0,
                &item.text,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let log_y = feedback_y + 84.0;
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: item_id(1),
                    header: "Time".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(2),
                    header: "Level".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(3),
                    header: "Message".to_owned(),
                    width: (body.width - 160.0).max(120.0),
                },
            ],
            header_height: 24.0,
            row_height: 24.0,
            sort: None,
        };
        let bounds = Rect::new(
            body.x + 8.0,
            log_y,
            body.width - 16.0,
            (body.max_y() - log_y - 8.0).max(0.0),
        );
        for header in table.header_rects(bounds) {
            rect(ui, header.rect, rgb(31, 33, 36), Some(rgb(48, 50, 56)));
            text(
                ui,
                header.rect.x + 8.0,
                header.rect.y + 16.0,
                &table.columns[header.index].header,
                11.0,
                rgb(178, 183, 190),
            );
        }
        for cell in table.visible_body_cells(bounds, LOGS.len(), 0.0, 1) {
            let log = &LOGS[cell.row];
            let value = match cell.column {
                0 => log.time,
                1 => log.level,
                _ => log.message,
            };
            rect(ui, cell.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                cell.rect.x + 8.0,
                cell.rect.y + 16.0,
                value,
                11.0,
                log_color(log.level),
            );
        }
    }

    pub(super) fn timeline_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let rows = [
            ("Intro camera pan", "Frame 001-072", 0.24),
            ("Character pickup", "Frame 073-144", 0.48),
            ("Vehicle reveal", "Frame 145-216", 0.72),
            ("Cut to gameplay", "Frame 217-300", 0.88),
        ];
        let layout = ListLayout::new(28.0);
        for item in layout.row_rects(body.inset(8.0), rows.len(), 0..rows.len()) {
            let (name, range, progress) = rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 18.0,
                name,
                11.0,
                rgb(222, 225, 230),
            );
            text(
                ui,
                item.rect.max_x() - 96.0,
                item.rect.y + 18.0,
                range,
                11.0,
                rgb(154, 160, 168),
            );
            let progress_rect = Rect::new(item.rect.max_x() - 220.0, item.rect.y + 10.0, 96.0, 6.0);
            rect(ui, progress_rect, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
            rect(
                ui,
                Rect::new(
                    progress_rect.x,
                    progress_rect.y,
                    progress_rect.width * progress,
                    progress_rect.height,
                ),
                rgb(69, 123, 220),
                None,
            );
        }
    }

    pub(super) fn node_graph_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        text(
            ui,
            body.x + 10.0,
            body.y + 20.0,
            "Compositor Graph",
            12.0,
            rgb(218, 222, 228),
        );

        match Self::showcase_node_graph_output(
            ui.id("editor.node-graph.static-view"),
            Self::showcase_node_graph_viewport(body),
        ) {
            Ok(output) => {
                ui.extend(output.primitives);
                for semantic in output.semantics {
                    ui.push_semantic_node(semantic);
                }
            }
            Err(_) => {
                text(
                    ui,
                    body.x + 10.0,
                    body.y + 42.0,
                    "Node graph descriptor unavailable",
                    11.0,
                    rgb(236, 96, 96),
                );
            }
        }
    }

    pub(super) fn showcase_node_graph_output(
        id: WidgetId,
        viewport: NodeGraphViewport,
    ) -> Result<NodeGraphStaticOutput, NodeGraphEmissionError> {
        let graph = Self::showcase_node_graph_descriptor();
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(2)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(51)),
            NodeGraphSelectionTarget::Reroute(RerouteId::from_raw(1)),
        ]);

        NodeGraphStaticView::new(id, viewport, &graph)
            .with_selection(selection)
            .with_incompatible_ports([PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(2))])
            .emit()
    }

    pub(super) fn showcase_node_graph_descriptor() -> NodeGraphDescriptor {
        const COLOR: PortTypeId = PortTypeId::from_raw(1);
        const MASK: PortTypeId = PortTypeId::from_raw(2);
        let frame = NodeFrameId::from_raw(1);
        let group = NodeGroupId::from_raw(1);

        NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Texture",
                    GraphRect::new(8.0, 28.0, 92.0, 64.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(1),
                    PortDirection::Output,
                    "Color",
                    COLOR,
                )])
                .with_frame(frame),
                NodeDescriptor::new(
                    NodeId::from_raw(2),
                    "Color Grade",
                    GraphRect::new(142.0, 54.0, 118.0, 76.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "In", COLOR),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Output, "Out", COLOR),
                    PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Mask", MASK)
                        .with_enabled(false),
                ])
                .with_frame(frame)
                .with_group(group)
                .with_label("Selected preview"),
                NodeDescriptor::new(
                    NodeId::from_raw(3),
                    "Output",
                    GraphRect::new(314.0, 36.0, 96.0, 68.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Input,
                        "Surface",
                        COLOR,
                    ),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Mask", MASK),
                ])
                .with_bypassed(true),
            ],
            edges: vec![
                EdgeDescriptor::new(
                    EdgeId::from_raw(50),
                    PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(1)),
                ),
                EdgeDescriptor::new(
                    EdgeId::from_raw(51),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
                    PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
                )
                .with_route_points(vec![NodeGraphEdgeRoutePoint::reroute(
                    RerouteId::from_raw(1),
                )]),
            ],
            reroutes: vec![RerouteDescriptor::new(
                RerouteId::from_raw(1),
                "Route A",
                GraphPoint::new(284.0, 88.0),
            )],
            frames: vec![NodeFrameDescriptor::new(
                frame,
                "Preview Frame",
                GraphRect::new(-4.0, 14.0, 282.0, 132.0),
            )],
            groups: vec![
                NodeGroupDescriptor::new(
                    group,
                    "Look Dev",
                    GraphRect::new(132.0, 44.0, 140.0, 96.0),
                )
                .with_nodes(vec![NodeId::from_raw(2)]),
            ],
        }
    }

    pub(super) fn showcase_node_graph_viewport(body: Rect) -> NodeGraphViewport {
        NodeGraphViewport::new(
            Rect::new(
                body.x + 8.0,
                body.y + 30.0,
                (body.width - 16.0).max(0.0),
                (body.height - 38.0).max(0.0),
            ),
            NodeGraphPanZoom::new(GraphVector::new(12.0, 8.0), 1.0),
        )
    }

    pub(super) fn status_bar(&self, ui: &mut Ui<'_>, viewport: Rect, action_count: u32) {
        let status_bar = self.status_bar_model(action_count);
        let visible_items = status_bar.visible_items();
        let bar = Rect::new(0.0, viewport.max_y() - 24.0, viewport.width, 24.0);
        rect(ui, bar, rgb(27, 29, 32), Some(rgb(52, 55, 62)));
        let message = visible_items
            .iter()
            .find(|item| item.id == EditorStatusItemKind::Message.id())
            .expect("editor status bar exposes message item");
        text(
            ui,
            10.0,
            bar.y + 16.0,
            &message.text,
            11.0,
            rgb(198, 203, 211),
        );
        let mut x = viewport.max_x() - 92.0;
        for item in visible_items
            .iter()
            .filter(|item| item.id != EditorStatusItemKind::Message.id())
            .rev()
        {
            let width = status_item_text_width(&item.text);
            let color = match item.kind {
                StatusItemKind::Error => rgb(236, 96, 96),
                StatusItemKind::Stale => rgb(232, 179, 90),
                StatusItemKind::Ready
                | StatusItemKind::Pending
                | StatusItemKind::Message
                | StatusItemKind::ActionCount
                | StatusItemKind::JobCount
                | StatusItemKind::Progress => rgb(154, 160, 168),
            };
            text(ui, x, bar.y + 16.0, &item.text, 11.0, color);
            x -= width;
        }
    }
}
