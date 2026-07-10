impl EditorShowcase {
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
            |ui, offset| {
                for row_rect in
                    layout.visible_row_rects_content(scroll, &rows, offset.y, 2)
                {
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
}
