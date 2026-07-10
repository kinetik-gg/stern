use super::super::{
    ACTION_SYSTEMS_DISPATCH, ActionContext, ActionDescriptor, ActionQueue, ActionSource,
    CommandPaletteOverlay, Menu, MenuOverlay, OverlayDismissal, OverlayEntry, OverlayId,
    OverlayKind, OverlayStack, Point, PopoverPlacement, PopoverRequest, Rect, ShowcaseApp, Size,
    Ui, inspect_primitives, overlay_semantics, page_rect, panel_title, place_popover, rect,
    rect_from_size, rgb, section_title, showcase_actions, text,
};

impl ShowcaseApp {
    pub(in crate::app) fn systems_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Actions, Overlays, Diagnostics, Stress");

        let actions = showcase_actions();
        if page.width >= 1220.0 {
            self.systems_action_panel(ui, Rect::new(page.x, page.y, 360.0, 240.0), &actions);
            Self::systems_overlay_panel(ui, Rect::new(page.x + 400.0, page.y, 420.0, 240.0));
            self.systems_palette_panel(
                ui,
                Rect::new(page.x + 860.0, page.y, 360.0, 240.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 286.0, 1220.0, 330.0));
        } else if page.width >= 820.0 {
            let column = (page.width - 24.0) * 0.5;
            self.systems_action_panel(ui, Rect::new(page.x, page.y, column, 240.0), &actions);
            Self::systems_overlay_panel(
                ui,
                Rect::new(page.x + column + 24.0, page.y, column, 240.0),
            );
            self.systems_palette_panel(
                ui,
                Rect::new(page.x, page.y + 264.0, page.width, 210.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 498.0, page.width, 360.0));
        } else {
            self.systems_action_panel(ui, Rect::new(page.x, page.y, page.width, 220.0), &actions);
            Self::systems_overlay_panel(ui, Rect::new(page.x, page.y + 244.0, page.width, 240.0));
            self.systems_palette_panel(
                ui,
                Rect::new(page.x, page.y + 508.0, page.width, 210.0),
                &actions,
            );
            self.systems_stress_panel(ui, Rect::new(page.x, page.y + 742.0, page.width, 440.0));
        }
    }

    pub(in crate::app) fn systems_action_panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        actions: &[ActionDescriptor],
    ) {
        panel_title(ui, panel, "Action Router");
        let menu_overlay = MenuOverlay::new(
            OverlayEntry::new(
                OverlayId::from_raw(101),
                OverlayKind::Menu,
                Rect::new(panel.x + 20.0, panel.y + 88.0, 140.0, 28.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
            Menu::from_actions(actions.to_vec()),
            ActionSource::Menu,
            ActionContext::Global,
        );
        let mut queue = ActionQueue::new();
        let dispatch_action = ActionDescriptor::new(ACTION_SYSTEMS_DISPATCH, "Record Dispatch");
        let x = panel.x + 20.0;
        let y = panel.y + 46.0;
        let dispatch = ui.button(
            "systems.dispatch",
            Rect::new(x, y, 140.0, 30.0),
            dispatch_action.label.clone(),
            !dispatch_action.state.enabled,
        );
        if dispatch.clicked {
            queue.invoke(
                dispatch_action.id.clone(),
                ActionSource::Button,
                ActionContext::Global,
            );
        }
        let menu_item = ui.button(
            "systems.menu-save",
            Rect::new(x, y + 44.0, 140.0, 28.0),
            "Menu Save",
            false,
        );
        if menu_item.clicked {
            menu_overlay.invoke_visible(0, &mut queue);
        }
        let invocations = self.handle_action_queue(&mut queue);
        text(
            ui,
            x + 160.0,
            y + 20.0,
            &format!("Dispatches: {}", self.systems_dispatch_count),
            11.0,
            rgb(144, 184, 255),
        );
        for invocation in invocations {
            text(
                ui,
                x,
                y + 112.0,
                invocation.action_id.as_str(),
                10.0,
                rgb(220, 220, 224),
            );
        }
    }

    pub(in crate::app) fn systems_overlay_panel(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Overlay Stack");
        let menu_rect = Rect::new(
            panel.x + 30.0,
            panel.y + 62.0,
            (panel.width - 110.0).clamp(180.0, 260.0),
            54.0,
        );
        let popover_size = Size::new((panel.width - 100.0).clamp(190.0, 230.0), 58.0);
        let popover_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.x + 30.0, menu_rect.y + 26.0, 120.0, 28.0),
                size: popover_size,
                placement: PopoverPlacement::Below,
                offset: 8.0,
                fit_viewport: true,
            },
            panel,
        );
        let palette_width = (panel.width - 80.0).clamp(220.0, 300.0);
        let palette_rect = Rect::new(
            (panel.x + panel.width * 0.28).min(panel.max_x() - palette_width - 30.0),
            panel.y + 132.0,
            palette_width,
            64.0,
        );
        let stack = Self::systems_overlay_stack(panel, menu_rect, popover_rect, palette_rect);
        for (index, entry) in stack.entries().iter().enumerate() {
            let label = match entry.kind {
                OverlayKind::Popover => "Popover",
                OverlayKind::Dropdown => "Dropdown",
                OverlayKind::ContextMenu => "Context Menu",
                OverlayKind::Menu => "Menu",
                OverlayKind::CommandPalette => "Command Palette",
                OverlayKind::Tooltip => "Tooltip",
                OverlayKind::Modal => "Modal",
                OverlayKind::DragPreview => "Drag Preview",
            };
            ui.push_semantic_node(overlay_semantics(entry, label));
            rect(
                ui,
                entry.rect,
                rgb(30 + index as u8 * 10, 32, 38),
                Some(rgb(90, 90, 98)),
            );
            text(
                ui,
                entry.rect.x + 14.0,
                entry.rect.y + 32.0,
                label,
                11.0,
                rgb(236, 236, 238),
            );
        }
    }

    pub(in crate::app) fn systems_overlay_stack(
        panel: Rect,
        menu_rect: Rect,
        popover_rect: Rect,
        palette_rect: Rect,
    ) -> OverlayStack {
        let menu_overlay = MenuOverlay::new(
            OverlayEntry::new(OverlayId::from_raw(1), OverlayKind::Menu, menu_rect)
                .dismiss_on(OverlayDismissal::OutsideClick),
            Menu::new(),
            ActionSource::Menu,
            ActionContext::Global,
        );
        let palette_overlay = CommandPaletteOverlay::from_actions(
            OverlayEntry::new(
                OverlayId::from_raw(3),
                OverlayKind::CommandPalette,
                palette_rect,
            )
            .modal(true),
            &[],
            ActionContext::Global,
        );
        let dropdown_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.max_x() - 52.0, menu_rect.y + 8.0, 42.0, 20.0),
                size: Size::new(130.0, 42.0),
                placement: PopoverPlacement::Right,
                offset: 6.0,
                fit_viewport: true,
            },
            panel,
        );
        let tooltip_rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(menu_rect.x + 8.0, menu_rect.y, 80.0, 18.0),
                size: Size::new(120.0, 24.0),
                placement: PopoverPlacement::Above,
                offset: 4.0,
                fit_viewport: true,
            },
            panel,
        );
        let mut stack = OverlayStack::new();
        menu_overlay.open_in(&mut stack);
        let _ = stack.open_child(
            menu_overlay.entry.id,
            OverlayEntry::new(OverlayId::from_raw(2), OverlayKind::Popover, popover_rect)
                .dismiss_on(OverlayDismissal::OutsideClick),
        );
        stack.open(
            OverlayEntry::new(OverlayId::from_raw(4), OverlayKind::Dropdown, dropdown_rect)
                .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        );
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(5),
            OverlayKind::Tooltip,
            tooltip_rect,
        ));
        palette_overlay.open_in(&mut stack);
        stack
    }

    pub(in crate::app) fn systems_palette_panel(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        actions: &[ActionDescriptor],
    ) {
        panel_title(ui, panel, "Command Palette");
        let mut palette_overlay = CommandPaletteOverlay::from_actions(
            OverlayEntry::new(
                OverlayId::from_raw(201),
                OverlayKind::CommandPalette,
                Rect::new(
                    panel.x + 20.0,
                    panel.y + 42.0,
                    (panel.width - 40.0).max(160.0),
                    132.0,
                ),
            )
            .modal(true)
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
            actions,
            ActionContext::Global,
        );
        palette_overlay.palette.query = String::new();
        let x = panel.x + 20.0;
        let row_width = (panel.width - 40.0).max(160.0);
        let entries = palette_overlay
            .matches()
            .into_iter()
            .take(4)
            .map(|entry| (entry.label.clone(), !entry.enabled))
            .collect::<Vec<_>>();
        for (index, (label, disabled)) in entries.into_iter().enumerate() {
            let y = panel.y + 50.0 + index as f32 * 32.0;
            let response = ui.list_row(
                ("systems.palette", index),
                Rect::new(x, y, row_width, 28.0),
                &label,
                false,
                disabled,
            );
            if response.clicked {
                let mut queue = ActionQueue::new();
                palette_overlay.palette.selected = index;
                palette_overlay.invoke_selected(&mut queue);
                self.handle_action_queue(&mut queue);
            }
        }
    }

    pub(in crate::app) fn systems_stress_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Primitive Stress");
        let origin = Point::new(panel.x + 36.0, panel.y + 70.0);
        self.systems_stress_slider(ui, panel, origin);

        let wide = panel.width >= 820.0;
        let snapshot = Self::stress_snapshot_rect(panel, origin, wide);
        let tile_area = Self::stress_tile_area(panel, origin, snapshot, wide);
        Self::draw_stress_tiles(ui, tile_area, self.stress);
        self.draw_runtime_snapshot(ui, snapshot, wide);
    }

    pub(in crate::app) fn systems_stress_slider(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        origin: Point,
    ) {
        let before = self.stress;
        let mut stress_value = (self.stress as f32 - 32.0) / 768.0;
        ui.slider(
            "systems.stress",
            Rect::new(
                origin.x,
                origin.y,
                (panel.width - 72.0).clamp(180.0, 260.0),
                16.0,
            ),
            &mut stress_value,
            0.0..=1.0,
            false,
        );
        self.stress = (32.0 + stress_value * 768.0).round() as usize;
        if before != self.stress {
            self.status = format!("Generated tiles: {}", self.stress);
        }
        text(
            ui,
            origin.x,
            origin.y - 12.0,
            &format!("Generated tiles: {}", self.stress),
            11.0,
            rgb(220, 220, 224),
        );
    }

    pub(in crate::app) fn stress_snapshot_rect(panel: Rect, origin: Point, wide: bool) -> Rect {
        let snapshot_width = (panel.width - 72.0).clamp(220.0, 320.0);
        if wide {
            Rect::new(
                panel.max_x() - snapshot_width - 40.0,
                origin.y,
                snapshot_width,
                188.0,
            )
        } else {
            Rect::new(origin.x, panel.max_y() - 154.0, snapshot_width, 132.0)
        }
    }

    pub(in crate::app) fn stress_tile_area(
        panel: Rect,
        origin: Point,
        snapshot: Rect,
        wide: bool,
    ) -> Rect {
        if wide {
            Rect::new(
                origin.x,
                origin.y + 56.0,
                (snapshot.x - origin.x - 40.0).max(120.0),
                panel.height - 102.0,
            )
        } else {
            Rect::new(
                origin.x,
                origin.y + 56.0,
                (panel.width - 72.0).max(120.0),
                (snapshot.y - origin.y - 76.0).max(40.0),
            )
        }
    }

    pub(in crate::app) fn draw_stress_tiles(ui: &mut Ui<'_>, tile_area: Rect, stress: usize) {
        let cols = (tile_area.width / 21.0).floor().max(1.0) as usize;
        ui.clip_rect("systems.stress.tiles", tile_area, |ui| {
            for index in 0..stress {
                let col = index % cols;
                let row = index / cols;
                let tile_x = tile_area.x + col as f32 * 21.0;
                let tile_y = tile_area.y + row as f32 * 16.0;
                let shade = 30 + (index % 80) as u8;
                rect(
                    ui,
                    Rect::new(tile_x, tile_y, 16.0, 10.0),
                    rgb(shade, 48, 70),
                    Some(rgb(60, 70, 90)),
                );
            }
        });
    }

    pub(in crate::app) fn draw_runtime_snapshot(
        &self,
        ui: &mut Ui<'_>,
        snapshot: Rect,
        wide: bool,
    ) {
        rect(ui, snapshot, rgb(18, 18, 20), Some(rgb(58, 58, 62)));
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 28.0,
            "Runtime Snapshot",
            13.0,
            rgb(238, 238, 240),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 58.0,
            &format!("Primitive count: {}", self.output.primitives.len()),
            10.0,
            rgb(190, 190, 194),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 78.0,
            &format!("Stress tiles: {}", self.stress),
            10.0,
            rgb(190, 190, 194),
        );
        text(
            ui,
            snapshot.x + 20.0,
            snapshot.y + 98.0,
            &format!("Action invocations: {}", self.action_count),
            10.0,
            rgb(190, 190, 194),
        );
        for (row, primitive) in inspect_primitives(&self.output.primitives)
            .into_iter()
            .take(if wide { 4 } else { 1 })
            .enumerate()
        {
            text(
                ui,
                snapshot.x + 20.0,
                snapshot.y + 132.0 + row as f32 * 18.0,
                &format!("#{} {:?}", primitive.index, primitive.kind),
                10.0,
                rgb(144, 184, 255),
            );
        }
    }
}
