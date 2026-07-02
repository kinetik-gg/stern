use super::super::{
    ClipId, Crosshair, Guide, PanZoom, Point, Primitive, Rect, SemanticNode, SemanticRole,
    ShowcaseApp, Size, TextureId, TexturePrimitive, Ui, ViewportComposition, ViewportSurface,
    page_rect, panel_title, rect_from_size, rgb, section_title, text,
};

impl ShowcaseApp {
    pub(in crate::app) fn viewport_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Viewport, Texture, and Overlay Surface");

        if page.width >= 1120.0 {
            let main = Rect::new(page.x, page.y, 960.0, 620.0);
            let side_x = main.max_x() + 40.0;
            self.viewport_controls_panel(ui, Rect::new(side_x, page.y, 300.0, 250.0));
            self.viewport_surface_panel(ui, main);
            Self::video_boundary_panel(ui, Rect::new(side_x, page.y + 286.0, 300.0, 230.0));
        } else {
            let width = page.width.min(980.0);
            let surface_height = ((width - 80.0).max(220.0) * 9.0 / 16.0).clamp(180.0, 300.0);
            let main_height = surface_height + 132.0;
            let main = Rect::new(page.x, page.y, width, main_height);
            self.viewport_controls_panel(ui, Rect::new(page.x, main.max_y() + 24.0, width, 150.0));
            self.viewport_surface_panel(ui, main);
            Self::video_boundary_panel(ui, Rect::new(page.x, main.max_y() + 198.0, width, 190.0));
        }
    }

    pub(in crate::app) fn viewport_surface_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Pan/Zoom Texture Surface");
        let max_surface_width = (panel.width - 80.0).max(160.0);
        let max_surface_height = (panel.height - 132.0).max(120.0);
        let surface_height = (max_surface_width * 9.0 / 16.0).min(max_surface_height);
        let surface_width = (surface_height * 16.0 / 9.0).min(max_surface_width);
        let surface = Rect::new(
            panel.x + (panel.width - surface_width) * 0.5,
            panel.y + 86.0,
            surface_width,
            surface_height,
        );
        let viewport_semantic_id = ui.id("viewport.surface.semantic");
        ui.push_semantic_node(
            SemanticNode::new(viewport_semantic_id, SemanticRole::Viewport, surface)
                .with_label("Pan/Zoom Texture Surface")
                .focusable(true),
        );

        let mut pan_zoom = PanZoom::default();
        pan_zoom.set_zoom(0.25 + self.zoom * 3.75);
        let composition = ViewportComposition {
            surface: ViewportSurface {
                texture: TextureId::from_raw(99),
                source_size: Size::new(384.0, 216.0),
                bounds: surface,
                pan_zoom,
            },
            guides: vec![
                Guide::Horizontal(108.0),
                Guide::Vertical(192.0),
                Guide::Horizontal(72.0),
            ],
            crosshair: Some(Crosshair {
                visible: true,
                position: Point::new(192.0, 108.0),
                label: Some("192, 108".to_owned()),
                color: rgb(240, 240, 240),
            }),
            clip: ClipId::from_raw(99),
        };
        ui.extend(composition.primitives_at(ui.viewport().scale_factor));
        text(
            ui,
            surface.x,
            surface.max_y() + 36.0,
            "Surface: 384x216 | Guides: 3 | Crosshair: 192,108",
            11.0,
            rgb(190, 190, 194),
        );
    }

    pub(in crate::app) fn viewport_controls_panel(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Viewport Controls");
        let x = panel.x + 40.0;
        let y = panel.y + 84.0;
        let slider_width = (panel.width - 80.0).clamp(160.0, 220.0);
        let before = self.zoom;
        ui.slider(
            "viewport.zoom",
            Rect::new(x, y, slider_width, 16.0),
            &mut self.zoom,
            0.0..=1.0,
            false,
        );
        if (before - self.zoom).abs() > f32::EPSILON {
            self.status = format!("Viewport zoom {:.0}%", 25.0 + self.zoom * 375.0);
        }
        let fit = ui.button(
            "viewport.fit",
            Rect::new(x, y + 44.0, 90.0, 28.0),
            "Fit",
            false,
        );
        if fit.clicked {
            self.zoom = 0.0;
            "Viewport fit".clone_into(&mut self.status);
        }
        let actual = ui.button(
            "viewport.actual",
            Rect::new(x + 104.0, y + 44.0, 116.0, 28.0),
            "Actual Size",
            false,
        );
        if actual.clicked {
            self.zoom = 0.2;
            "Viewport actual size".clone_into(&mut self.status);
        }
        text(
            ui,
            x,
            y - 10.0,
            &format!("Zoom: {:.0}%", 25.0 + self.zoom * 375.0),
            11.0,
            rgb(220, 220, 224),
        );
    }

    pub(in crate::app) fn video_boundary_panel(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "3D/Video Boundary");
        let texture_width = (panel.width - 80.0).clamp(180.0, 260.0);
        let texture_height = texture_width * 9.0 / 16.0;
        let texture = Rect::new(
            panel.x + 40.0,
            panel.y + 54.0,
            texture_width,
            texture_height,
        );
        ui.primitive(Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(101),
            rect: texture,
            source_size: Size::new(256.0, 144.0),
        }));
        text(
            ui,
            texture.x,
            texture.max_y() + 34.0,
            "Frame 256x144",
            11.0,
            rgb(220, 220, 224),
        );
    }
}
