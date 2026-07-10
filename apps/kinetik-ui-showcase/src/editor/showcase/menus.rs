impl EditorShowcase {
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
                    if response.clicked && enabled {
                        let mut queue = ActionQueue::new();
                        if overlay.invoke_visible(index, &mut queue) {
                            self.handle_action_queue(invocations, &mut queue);
                            self.open_menu = None;
                            ui.request_repaint(RepaintRequest::NextFrame);
                            return;
                        }
                    }
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
                    ACTION_NEW_SCENE,
                    "New Scene (Experimental)",
                    Some(ctrl_char("n")),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_OPEN_PROJECT,
                    "Open Project... (Experimental)",
                    Some(ctrl_char("o")),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_SAVE,
                    "Save Scene (Experimental)",
                    Some(ctrl_char("s")),
                    None,
                    false,
                ),
                MenuItem::Separator,
                menu_action(
                    ACTION_IMPORT_ASSET,
                    "Import Asset... (Experimental)",
                    None,
                    None,
                    false,
                ),
                menu_action(
                    ACTION_EXPORT,
                    "Export Build (Experimental)",
                    Some(ctrl_char("b")),
                    None,
                    false,
                ),
                MenuItem::Separator,
                menu_action(ACTION_QUIT, "Quit (Experimental)", None, None, false),
            ]),
            EditorMenuKind::Edit => menu([
                menu_action(
                    ACTION_UNDO,
                    "Undo (Experimental)",
                    Some(ctrl_char("z")),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_REDO,
                    "Redo (Experimental)",
                    Some(ctrl_char("y")),
                    None,
                    false,
                ),
                MenuItem::Separator,
                menu_action(
                    ACTION_DUPLICATE,
                    "Duplicate (Experimental)",
                    Some(ctrl_char("d")),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_DELETE,
                    "Delete (Experimental)",
                    Some(shortcut(Key::Delete)),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_PREFERENCES,
                    "Preferences (Experimental)",
                    Some(ctrl_char(",")),
                    None,
                    false,
                ),
            ]),
            EditorMenuKind::View => menu([
                menu_action(
                    ACTION_VIEW_PERSPECTIVE,
                    "Perspective View (Experimental)",
                    None,
                    None,
                    false,
                ),
                menu_action(
                    ACTION_VIEWPORT_FOCUS_SELECTED,
                    "Frame Selected (Experimental)",
                    Some(shortcut(Key::Character("f".to_owned()))),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_GRID,
                    "Show Grid",
                    Some(shortcut(Key::Character("g".to_owned()))),
                    Some(self.grid_visible),
                    true,
                ),
                menu_action(
                    ACTION_SHOW_OVERLAYS,
                    "Show Overlays (Experimental)",
                    None,
                    None,
                    false,
                ),
                menu_action(
                    ACTION_VIEWPORT_FIT_CONTENT,
                    "Reset View (Experimental)",
                    None,
                    None,
                    false,
                ),
            ]),
            EditorMenuKind::Project => menu([
                menu_action(
                    ACTION_PLAY,
                    "Play",
                    Some(shortcut(Key::Function(5))),
                    None,
                    !self.running,
                ),
                menu_action(
                    ACTION_STOP,
                    "Stop",
                    None,
                    None,
                    self.running || self.timeline > 0.0,
                ),
                MenuItem::Separator,
                menu_action(
                    ACTION_PROJECT_SETTINGS,
                    "Project Settings... (Experimental)",
                    None,
                    None,
                    false,
                ),
            ]),
            EditorMenuKind::Build => menu([
                menu_action(
                    ACTION_BUILD,
                    "Build Project (Experimental)",
                    Some(ctrl_char("b")),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_PACKAGE_WINDOWS,
                    "Package Windows x64 (Experimental)",
                    None,
                    None,
                    false,
                ),
                menu_action(
                    ACTION_RUN_PROFILER,
                    "Run Profiler (Experimental)",
                    None,
                    None,
                    false,
                ),
            ]),
            EditorMenuKind::Window => self.window_menu_model(),
            EditorMenuKind::Help => menu([
                menu_action(
                    ACTION_DOCS,
                    "Online Docs (Experimental)",
                    Some(shortcut(Key::Function(1))),
                    None,
                    false,
                ),
                menu_action(
                    ACTION_KEYBOARD_SHORTCUTS,
                    "Keyboard Shortcuts (Experimental)",
                    None,
                    None,
                    false,
                ),
                MenuItem::Separator,
                menu_action(
                    ACTION_ABOUT,
                    "About Kinetik Forge",
                    None,
                    None,
                    true,
                ),
            ]),
        }
    }

    pub(super) fn window_menu_model(&self) -> Menu {
        let registry = editor_panel_registry();
        let open_metadata = editor_open_panel_metadata();
        let mut menu = Menu::new();
        menu.push(menu_action(
            ACTION_PALETTE,
            "Command Palette (Experimental)",
            Some(ctrl_char("p")),
            None,
            false,
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
}
