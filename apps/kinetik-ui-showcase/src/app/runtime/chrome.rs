use super::super::{
    Axis, Color, Rect, RepaintRequest, ShowcaseApp, ShowcasePage, Size, Ui, editor_nav_bounds,
    editor_nav_items, nav_items, page_rect, rect, rect_from_size, rgb, split_leading, text,
};

impl ShowcaseApp {
    pub(in crate::app) fn app_background(ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let (_, body) = split_leading(viewport, Axis::Vertical, 52.0);
        rect(ui, viewport, rgb(11, 12, 13), None);
        rect(
            ui,
            Rect::new(0.0, 52.0, viewport.width, 1.0),
            rgb(65, 72, 84),
            None,
        );
        let footer_height = body.height.min(140.0);
        rect(
            ui,
            Rect::new(
                0.0,
                viewport.max_y() - footer_height,
                viewport.width,
                footer_height,
            ),
            rgb(13, 16, 17),
            None,
        );
    }

    pub(in crate::app) fn page_content(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let scroll_bounds = Rect::new(0.0, 52.0, viewport.width, (viewport.height - 52.0).max(1.0));
        let content_size = Size::new(viewport.width, self.page_content_height(viewport));
        ui.scroll_area(
            ("showcase.page-scroll", self.page as u8),
            scroll_bounds,
            content_size,
            false,
            |ui, _| match self.page {
                ShowcasePage::Editor => {
                    let _ = ui;
                }
                ShowcasePage::Components => self.components_page(ui),
                ShowcasePage::Layout => self.layout_page(ui),
                ShowcasePage::Viewport => self.viewport_page(ui),
                ShowcasePage::Systems => self.systems_page(ui),
            },
        );
    }

    pub(in crate::app) fn page_content_height(&self, viewport: Rect) -> f32 {
        let page = page_rect(viewport);
        let height: f32 = match self.page {
            ShowcasePage::Editor => viewport.height,
            ShowcasePage::Components | ShowcasePage::Layout if page.width >= 1160.0 => 840.0,
            ShowcasePage::Components => 1320.0,
            ShowcasePage::Layout => 1180.0,
            ShowcasePage::Viewport if page.width >= 1160.0 => 780.0,
            ShowcasePage::Viewport => 1160.0,
            ShowcasePage::Systems if page.width >= 1220.0 => 780.0,
            ShowcasePage::Systems if page.width >= 820.0 => 1120.0,
            ShowcasePage::Systems => 1340.0,
        };
        height.max(viewport.height)
    }

    pub(in crate::app) fn chrome_nav(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        rect(
            ui,
            Rect::new(0.0, 0.0, viewport.width, 52.0),
            rgb(19, 21, 23),
            Some(rgb(58, 64, 72)),
        );
        rect(ui, Rect::new(0.0, 0.0, 6.0, 52.0), rgb(82, 150, 132), None);
        text(
            ui,
            20.0,
            24.0,
            "Kinetik UI Showcase",
            15.0,
            rgb(238, 238, 238),
        );
        text(ui, 20.0, 40.0, "Workbench", 10.0, rgb(150, 160, 164));
        self.page_selector(ui, nav_items(viewport.width));
    }

    pub(in crate::app) fn editor_nav(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        rect(
            ui,
            editor_nav_bounds(viewport),
            rgb(19, 21, 23),
            Some(rgb(58, 64, 72)),
        );
        self.page_selector(ui, editor_nav_items(viewport));
    }

    fn page_selector<const N: usize>(&mut self, ui: &mut Ui<'_>, items: [(ShowcasePage, Rect); N]) {
        for (page, item) in items {
            let response = ui.tab_button_value(
                ("nav", page as u8),
                item,
                page.label(),
                &mut self.page,
                page,
                false,
            );
            if response.clicked {
                self.status = format!("Page: {}", page.label());
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    pub(in crate::app) fn chrome_status(&self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        if viewport.width >= 1200.0 {
            Self::status_badge(
                ui,
                Rect::new(viewport.width - 434.0, 12.0, 128.0, 28.0),
                "Primitives",
                &self.output.primitives.len().to_string(),
                rgb(82, 150, 132),
            );
            Self::status_badge(
                ui,
                Rect::new(viewport.width - 294.0, 12.0, 108.0, 28.0),
                "Actions",
                &self.action_count.to_string(),
                rgb(144, 184, 255),
            );
            text(
                ui,
                viewport.width - 170.0,
                31.0,
                &self.status,
                10.0,
                rgb(178, 182, 188),
            );
        }
    }

    pub(in crate::app) fn status_badge(
        ui: &mut Ui<'_>,
        rect_value: Rect,
        label: &str,
        value: &str,
        accent: Color,
    ) {
        rect(ui, rect_value, rgb(26, 28, 31), Some(rgb(62, 68, 76)));
        rect(
            ui,
            Rect::new(rect_value.x, rect_value.y, 3.0, rect_value.height),
            accent,
            None,
        );
        text(
            ui,
            rect_value.x + 10.0,
            rect_value.y + 11.0,
            label,
            8.0,
            rgb(142, 148, 156),
        );
        text(
            ui,
            rect_value.x + 76.0,
            rect_value.y + 19.0,
            value,
            11.0,
            rgb(232, 234, 238),
        );
    }
}
