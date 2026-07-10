use super::super::{
    ACTION_COMPONENTS_RUN, ActionSource, GridColumns, GridLayout, IconId, ImageId, ListLayout,
    Point, Rect, ShowcaseApp, Size, Ui, line, page_rect, panel_title, rect, rect_from_size, rgb,
    section_title, text,
};

impl ShowcaseApp {
    pub(in crate::app) fn components_page(&mut self, ui: &mut Ui<'_>) {
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        section_title(ui, page.x, 86.0, "Component Gallery");

        if page.width >= 1160.0 {
            self.component_controls(ui, Rect::new(page.x, page.y, 620.0, 218.0));
            self.component_text_inputs(ui, Rect::new(page.x + 660.0, page.y, 500.0, 218.0));
            self.collection_preview(ui, Rect::new(page.x, page.y + 246.0, 560.0, 190.0));
            self.tabs_preview(ui, Rect::new(page.x + 600.0, page.y + 246.0, 560.0, 190.0));
            Self::primitive_preview(ui, Rect::new(page.x, page.y + 466.0, 1160.0, 230.0));
        } else {
            let width = page.width.min(900.0);
            self.component_controls(ui, Rect::new(page.x, page.y, width, 218.0));
            self.component_text_inputs(ui, Rect::new(page.x, page.y + 242.0, width, 218.0));
            self.collection_preview(ui, Rect::new(page.x, page.y + 484.0, width, 190.0));
            self.tabs_preview(ui, Rect::new(page.x, page.y + 698.0, width, 190.0));
            Self::primitive_preview(ui, Rect::new(page.x, page.y + 912.0, width, 230.0));
        }
    }

    pub(in crate::app) fn component_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Controls");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;
        let compact = panel.width < 560.0;
        let slider_x = if compact { x } else { x + 300.0 };
        let slider_y = if compact { y + 88.0 } else { y + 8.0 };
        let slider_width = if compact {
            (panel.width - 40.0).max(120.0)
        } else {
            (panel.max_x() - slider_x - 60.0).clamp(160.0, 240.0)
        };

        self.component_button_controls(ui, x, y);
        self.component_selection_controls(ui, x, y);
        self.component_slider_controls(ui, panel, slider_x, slider_y, slider_width);
        Self::state_strip(
            ui,
            Rect::new(
                x,
                panel.max_y() - 46.0,
                (panel.width - 40.0).max(120.0),
                24.0,
            ),
            &format!(
                "checkbox={} toggle={} radio={} selected_row={} action_counter={}",
                self.checkbox,
                self.toggle,
                self.radio + 1,
                self.selected_row + 1,
                self.component_action_count
            ),
        );
    }

    pub(in crate::app) fn component_button_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let run = ui.button(
            "components.run-action",
            Rect::new(x, y, 128.0, 30.0),
            "Increment Counter",
            false,
        );
        if run.clicked {
            self.invoke_action(ACTION_COMPONENTS_RUN, ActionSource::Button);
        }

        let disabled = ui.button(
            "components.disabled",
            Rect::new(x + 144.0, y, 128.0, 30.0),
            "Disabled",
            true,
        );
        if disabled.clicked {
            "Disabled button should not invoke".clone_into(&mut self.status);
        }
    }

    pub(in crate::app) fn component_selection_controls(&mut self, ui: &mut Ui<'_>, x: f32, y: f32) {
        let checkbox = ui.checkbox_value(
            "components.checkbox",
            Rect::new(x, y + 48.0, 22.0, 22.0),
            &mut self.checkbox,
            false,
        );
        if checkbox.clicked {
            self.status = format!("Checkbox: {}", self.checkbox);
        }
        ui.label(Rect::new(x + 32.0, y + 46.0, 90.0, 20.0), "Checkbox");

        let toggle = ui.toggle_value(
            "components.toggle",
            Rect::new(x + 144.0, y + 48.0, 54.0, 24.0),
            &mut self.toggle,
            false,
        );
        if toggle.clicked {
            self.status = format!("Toggle: {}", self.toggle);
        }
        ui.label(Rect::new(x + 210.0, y + 46.0, 70.0, 20.0), "Toggle");

        for (index, radio_x, label) in [(0, x, "Radio A"), (1, x + 100.0, "Radio B")] {
            let response = ui.radio_button_value(
                ("components.radio", index),
                Rect::new(radio_x, y + 94.0, 20.0, 20.0),
                &mut self.radio,
                index,
                false,
            );
            if response.clicked {
                self.status = format!("Radio: {label}");
            }
            ui.label(Rect::new(radio_x + 30.0, y + 92.0, 70.0, 20.0), label);
        }
    }

    pub(in crate::app) fn component_slider_controls(
        &mut self,
        ui: &mut Ui<'_>,
        panel: Rect,
        slider_x: f32,
        slider_y: f32,
        slider_width: f32,
    ) {
        let before = self.strength;
        ui.slider(
            "components.slider",
            Rect::new(slider_x, slider_y, slider_width, 16.0),
            &mut self.strength,
            0.0..=1.0,
            false,
        );
        text(
            ui,
            slider_x,
            slider_y - 10.0,
            &format!("Slider: {:.2}", self.strength),
            10.0,
            rgb(210, 210, 214),
        );
        if (before - self.strength).abs() > f32::EPSILON {
            self.status = format!("Slider: {:.2}", self.strength);
        }

        let icon_button_size = ui.theme().controls.control_height;
        let icon = ui.icon_button_with_label(
            "components.icon",
            Rect::new(
                slider_x,
                slider_y + 44.0,
                icon_button_size,
                icon_button_size,
            ),
            IconId::from_raw(1),
            "Icon button",
            false,
        );
        if icon.clicked {
            "Icon button".clone_into(&mut self.status);
        }
        ui.label(
            Rect::new(slider_x + 44.0, slider_y + 50.0, 90.0, 20.0),
            "Icon button",
        );
        if panel.width >= 560.0 {
            ui.image(
                Rect::new(slider_x + 152.0, slider_y + 36.0, 64.0, 48.0),
                ImageId::from_raw(7),
            );
            ui.label(
                Rect::new(slider_x + 152.0, slider_y + 96.0, 120.0, 20.0),
                "Thumbnail",
            );
        }
    }

    pub(in crate::app) fn component_text_inputs(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Text Input");
        let x = panel.x + 20.0;
        let y = panel.y + 46.0;
        let compact = panel.width < 460.0;
        let primary_width = if compact {
            (panel.width - 40.0).max(140.0)
        } else {
            (panel.width * 0.46).clamp(190.0, 260.0)
        };
        let secondary_x = if compact { x } else { x + primary_width + 50.0 };
        let secondary_y = y + 64.0;
        let secondary_width = if compact {
            primary_width.min(160.0)
        } else {
            (panel.max_x() - secondary_x - 20.0).clamp(100.0, 160.0)
        };

        text(ui, x, y - 8.0, "Search", 10.0, rgb(190, 190, 194));
        let search = ui.search_field(
            "components.search",
            Rect::new(x, y, primary_width, 30.0),
            &mut self.search,
            false,
        );
        if search.field.changed {
            self.status = format!("Search: {}", search.query);
        }

        text(ui, x, y + 56.0, "Text field", 10.0, rgb(190, 190, 194));
        let name = ui.text_field(
            "components.name",
            Rect::new(x, y + 64.0, primary_width.min(220.0), 30.0),
            &mut self.name,
            false,
        );
        if name.changed {
            self.status = format!("Name: {}", self.name.text);
        }

        text(
            ui,
            secondary_x,
            secondary_y - 8.0,
            "Numeric",
            10.0,
            rgb(190, 190, 194),
        );
        let number = ui.numeric_input(
            "components.number",
            Rect::new(secondary_x, secondary_y, secondary_width, 30.0),
            &mut self.number,
            false,
        );
        if number.field.changed {
            self.status = if number.valid {
                format!("Number: {}", self.number.text)
            } else {
                "Number field is invalid".to_owned()
            };
        }

        let notes_y = if compact { y + 118.0 } else { y + 120.0 };
        text(ui, x, notes_y - 8.0, "Multi-line", 10.0, rgb(190, 190, 194));
        let notes = ui.multi_line_text_field(
            "components.notes",
            Rect::new(x, notes_y, (panel.width - 80.0).max(160.0), 38.0),
            &mut self.notes,
            false,
        );
        if notes.changed {
            self.status = format!("Notes: {} lines", self.notes.text.lines().count());
        }

        text(
            ui,
            x + (panel.width - 160.0).max(0.0),
            notes_y + 30.0,
            "Undo stack",
            10.0,
            rgb(160, 160, 164),
        );
    }

    pub(in crate::app) fn collection_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Lists, Grids, Tables");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;
        let list_width = (panel.width * 0.45).clamp(220.0, 260.0);
        let grid_x = if panel.width >= 520.0 {
            x + list_width + 50.0
        } else {
            x
        };
        let grid_y = if panel.width >= 520.0 { y } else { y + 124.0 };

        let list = ListLayout::new(28.0);
        let labels = [
            "Row: primary surface",
            "Row: selected state",
            "Row: cached resource",
            "Row: async result",
        ];
        for row in list.row_rects(Rect::new(x, y, list_width, 112.0), labels.len(), 0..4) {
            let response = ui.list_row_value(
                ("components.list-row", row.index),
                row.rect,
                labels[row.index],
                &mut self.selected_row,
                row.index,
                false,
            );
            if response.clicked {
                self.status = format!("Selected row {}", row.index + 1);
            }
        }

        let grid = GridLayout {
            columns: GridColumns::Fixed(4),
            item_size: Size::new(42.0, 30.0),
            gap: 12.0,
        };
        for item in grid.item_rects(
            Rect::new(
                grid_x,
                grid_y,
                (panel.max_x() - grid_x - 20.0).max(180.0),
                120.0,
            ),
            12,
            0..12,
        ) {
            rect(ui, item.rect, rgb(36, 38, 42), Some(rgb(70, 70, 74)));
        }
    }

    pub(in crate::app) fn tabs_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Reusable Panel States");
        let x = panel.x + 20.0;
        let y = panel.y + 40.0;

        for (index, tab_x, label) in [
            (0, x, "Theme"),
            (1, x + 120.0, "State"),
            (2, x + 240.0, "Actions"),
        ] {
            let response = ui.tab_button_value(
                ("components.tab", index),
                Rect::new(tab_x, y, 108.0, 30.0),
                label,
                &mut self.selected_tab,
                index,
                false,
            );
            if response.clicked {
                self.status = format!("Tab: {label}");
            }
        }

        let body = match self.selected_tab {
            0 => "Palette: graphite, cyan, steel, signal blue.",
            1 => "State: focus, hover, active, selected, disabled.",
            _ => "Actions: toolbar, menu, palette, shortcut.",
        };
        rect(
            ui,
            Rect::new(x, y + 40.0, (panel.width - 40.0).max(140.0), 82.0),
            rgb(22, 22, 25),
            Some(rgb(62, 62, 66)),
        );
        text(ui, x + 20.0, y + 72.0, body, 11.0, rgb(224, 224, 226));
        text(
            ui,
            x + 20.0,
            y + 102.0,
            &format!("Actions: {}", self.action_count),
            12.0,
            rgb(144, 184, 255),
        );
    }

    pub(in crate::app) fn primitive_preview(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Primitive Stream");
        let x = panel.x + 24.0;
        let y = panel.y + 48.0;
        rect(
            ui,
            Rect::new(x, y, 120.0, 72.0),
            rgb(46, 48, 54),
            Some(rgb(120, 120, 126)),
        );
        line(
            ui,
            Point::new(x + 146.0, y),
            Point::new(x + 266.0, y + 72.0),
            rgb(230, 230, 230),
            2.0,
        );
        ui.image(Rect::new(x + 296.0, y, 96.0, 72.0), ImageId::from_raw(11));
        text(ui, x + 436.0, y + 42.0, "Label", 13.0, rgb(238, 238, 238));
        rect(
            ui,
            Rect::new((x + 636.0).min(panel.max_x() - 160.0), y, 140.0, 72.0),
            rgb(12, 12, 13),
            Some(rgb(92, 132, 240)),
        );
        if panel.width >= 900.0 {
            ui.separator(Rect::new(x + 816.0, y + 32.0, 220.0, 12.0));
        }
    }

    pub(in crate::app) fn state_strip(ui: &mut Ui<'_>, bounds: Rect, value: &str) {
        rect(ui, bounds, rgb(22, 22, 25), Some(rgb(58, 58, 62)));
        text(
            ui,
            bounds.x + 10.0,
            bounds.y + 16.0,
            value,
            10.0,
            rgb(190, 190, 194),
        );
    }
}
