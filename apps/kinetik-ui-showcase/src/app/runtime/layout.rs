use super::super::{
    Axis, Dock, DockDropTarget, DockNode, DockPlacement, DockSplitDemoState, Frame, FrameId,
    GridColumns, GridLayout, Insets, ItemId, LayoutItem, Measurement, Panel, PanelId, Rect,
    SemanticNode, SemanticRole, ShowcaseApp, Size, SizeRule, TableColumn, TableLayout, Ui,
    column_layout, frame_tabs, page_rect, panel_title, panel_title_body, rect, rect_from_size, rgb,
    row_layout, section_title, solve_dock_layout, solve_dock_splitters, text,
};
use kinetik_ui::widgets::FrameLayout;

const DOCK_PREVIEW_TAB_MAX_WIDTH: f32 = 74.0;
const DOCK_PREVIEW_TAB_HEIGHT: f32 = 22.0;
const DOCK_PREVIEW_TAB_GAP: f32 = 4.0;
const DOCK_PREVIEW_TAB_INSET: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct DockPreviewTabLayout {
    pub(in crate::app) frame: FrameId,
    pub(in crate::app) panel: PanelId,
    pub(in crate::app) rect: Rect,
    title: String,
    active: bool,
}

impl ShowcaseApp {
    pub(in crate::app) fn layout_page(&mut self, ui: &mut Ui<'_>) {
        section_title(ui, 40.0, 86.0, "Layout, Docking, and Data Surfaces");
        let viewport = rect_from_size(ui.viewport().logical_size);
        let page = page_rect(viewport);
        if page.width >= 1160.0 {
            Self::layout_solver_preview(ui, Rect::new(page.x, page.y, 560.0, 320.0));
            self.dock_preview(ui, Rect::new(page.x + 600.0, page.y, 560.0, 320.0));
            Self::table_preview(ui, Rect::new(page.x, page.y + 356.0, 1160.0, 300.0));
        } else {
            let width = page.width.min(760.0);
            Self::layout_solver_preview(ui, Rect::new(page.x, page.y, width, 320.0));
            self.dock_preview(ui, Rect::new(page.x, page.y + 356.0, width, 320.0));
            Self::table_preview(ui, Rect::new(page.x, page.y + 712.0, width, 300.0));
        }
    }

    pub(in crate::app) fn layout_solver_preview(ui: &mut Ui<'_>, panel: Rect) {
        let body = panel_title_body(
            ui,
            panel,
            "Measurement-Aware Layout",
            Insets::new(20.0, 20.0, 46.0, 16.0),
        );

        ui.clip_rect("layout.measurement.body", body, |ui| {
            let row_bounds = Rect::new(body.x + 4.0, body.y, (body.width - 8.0).max(0.0), 42.0);
            let items = [
                LayoutItem::new(
                    SizeRule::Fixed(140.0),
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(140.0, 42.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(180.0, 42.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fit,
                    SizeRule::Fixed(42.0),
                    Measurement::new(Size::new(96.0, 42.0)),
                ),
            ];
            for (index, rect_value) in row_layout(row_bounds, &items, 8.0).into_iter().enumerate() {
                rect(ui, rect_value, rgb(36, 42, 50), Some(rgb(90, 110, 140)));
                text(
                    ui,
                    rect_value.x + 12.0,
                    rect_value.y + 26.0,
                    &format!("Row {index}"),
                    11.0,
                    rgb(236, 236, 236),
                );
            }

            let column_items = [
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(34.0),
                    Measurement::new(Size::new(80.0, 34.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(54.0),
                    Measurement::new(Size::new(80.0, 54.0)),
                ),
                LayoutItem::new(
                    SizeRule::Fill,
                    SizeRule::Fixed(34.0),
                    Measurement::new(Size::new(80.0, 34.0)),
                ),
            ];
            let column_bounds = Rect::new(body.x + 4.0, body.y + 70.0, 220.0, 122.0);
            for rect_value in column_layout(column_bounds, &column_items, 8.0) {
                rect(ui, rect_value, rgb(44, 38, 52), Some(rgb(120, 94, 150)));
            }

            let grid_x = (body.x + 280.0).min(body.max_x() - 220.0).max(body.x + 4.0);
            let adaptive = GridLayout {
                columns: GridColumns::Adaptive { min_width: 64.0 },
                item_size: Size::new(58.0, 32.0),
                gap: 8.0,
            };
            for item in
                adaptive.item_rects(Rect::new(grid_x, body.y + 70.0, 220.0, 120.0), 12, 0..12)
            {
                rect(ui, item.rect, rgb(38, 45, 44), Some(rgb(84, 122, 110)));
            }
        });
    }

    pub(in crate::app) fn dock_preview(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Interactive Dock Model");
        let dock_semantic_id = ui.id("layout.dock-preview.semantic");
        ui.push_semantic_node(
            SemanticNode::new(dock_semantic_id, SemanticRole::Dock, panel)
                .with_label("Interactive Dock Model"),
        );
        self.dock_preview_controls(ui, panel);
        let area = self.dock_model_preview();
        Self::draw_dock_preview(ui, &area, panel);
    }

    pub(in crate::app) fn dock_model_preview(&self) -> Dock {
        let mut area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: self.dock_ratio,
            min_first: 140.0,
            min_second: 220.0,
            first: Box::new(DockNode::Frame(Frame::new(
                FrameId::from_raw(1),
                vec![
                    Panel::new(PanelId::from_raw(1), "Inspector"),
                    Panel::new(PanelId::from_raw(2), "Assets"),
                ],
            ))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.62,
                min_first: 120.0,
                min_second: 80.0,
                first: Box::new(DockNode::Frame(Frame::new(
                    FrameId::from_raw(2),
                    vec![Panel::new(PanelId::from_raw(3), "Viewport")],
                ))),
                second: Box::new(DockNode::Frame(Frame::new(
                    FrameId::from_raw(3),
                    vec![
                        Panel::new(PanelId::from_raw(4), "Console"),
                        Panel::new(PanelId::from_raw(5), "Jobs"),
                    ],
                ))),
            }),
        });

        if self.dock_split_demo == DockSplitDemoState::Inserted {
            let drag = area
                .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
                .expect("demo panel exists");
            area.drop_tab(
                drag,
                DockDropTarget::split(
                    FrameId::from_raw(1),
                    DockPlacement::Bottom,
                    FrameId::from_raw(9),
                ),
            );
        }

        area
    }

    pub(in crate::app) fn dock_preview_controls(&mut self, ui: &mut Ui<'_>, panel: Rect) {
        let before = self.dock_ratio;
        ui.slider(
            "layout.dock-ratio",
            Rect::new(panel.x + 236.0, panel.y + 54.0, 170.0, 14.0),
            &mut self.dock_ratio,
            0.25..=0.75,
            false,
        );
        if (before - self.dock_ratio).abs() > f32::EPSILON {
            self.status = format!("Dock split: {:.0}%", self.dock_ratio * 100.0);
        }
        text(
            ui,
            panel.x + 420.0,
            panel.y + 64.0,
            &format!("{:.0}%", self.dock_ratio * 100.0),
            10.0,
            rgb(190, 190, 194),
        );

        let split = ui.button(
            "layout.split-demo",
            Rect::new(panel.x + 32.0, panel.y + 46.0, 132.0, 28.0),
            if self.dock_split_demo == DockSplitDemoState::Inserted {
                "Reset Dock"
            } else {
                "Split Tab"
            },
            false,
        );
        if split.clicked {
            self.dock_split_demo = match self.dock_split_demo {
                DockSplitDemoState::Base => DockSplitDemoState::Inserted,
                DockSplitDemoState::Inserted => DockSplitDemoState::Base,
            };
            self.status = match self.dock_split_demo {
                DockSplitDemoState::Base => "Dock split reset".to_owned(),
                DockSplitDemoState::Inserted => "Dock tab split inserted".to_owned(),
            };
        }
    }

    pub(in crate::app) fn draw_dock_preview(ui: &mut Ui<'_>, area: &Dock, panel: Rect) {
        let dock_bounds = Rect::new(
            panel.x + 20.0,
            panel.y + 86.0,
            (panel.width - 60.0).max(0.0),
            (panel.height - 116.0).max(0.0),
        );
        let frame_layouts = solve_dock_layout(area, dock_bounds);
        for frame in &frame_layouts {
            rect(ui, frame.rect, rgb(22, 22, 25), Some(rgb(70, 70, 76)));
            text(
                ui,
                frame.rect.x + 10.0,
                frame.rect.y + 24.0,
                &format!("Frame {}", frame.frame.raw()),
                10.0,
                rgb(180, 180, 184),
            );
        }
        for splitter in solve_dock_splitters(area, dock_bounds, 6.0) {
            rect(
                ui,
                splitter.rect,
                rgb(82, 94, 118),
                Some(rgb(116, 132, 160)),
            );
        }
        for tab in Self::dock_preview_tab_layouts(area, &frame_layouts) {
            rect(
                ui,
                tab.rect,
                if tab.active {
                    rgb(42, 96, 224)
                } else {
                    rgb(30, 30, 33)
                },
                Some(rgb(72, 72, 76)),
            );
            text(
                ui,
                tab.rect.x + 8.0,
                tab.rect.y + 15.0,
                &tab.title,
                9.0,
                rgb(236, 236, 238),
            );
        }
        text(
            ui,
            panel.x + 32.0,
            panel.max_y() - 10.0,
            &format!("Frames: {} | Snapshot: valid", area.frames().len()),
            10.0,
            rgb(160, 160, 164),
        );
    }

    pub(in crate::app) fn dock_preview_tab_layouts(
        area: &Dock,
        frame_layouts: &[FrameLayout],
    ) -> Vec<DockPreviewTabLayout> {
        let frames = area.frames();
        let mut tab_layouts = Vec::new();

        for frame_layout in frame_layouts {
            let Some(frame) = frames.iter().find(|frame| frame.id == frame_layout.frame) else {
                continue;
            };
            let tabs = frame_tabs(frame);
            let tab_count = tabs.len();
            for (index, tab) in tabs.into_iter().enumerate() {
                tab_layouts.push(DockPreviewTabLayout {
                    frame: frame_layout.frame,
                    panel: tab.panel,
                    rect: dock_preview_tab_rect(frame_layout.rect, index, tab_count),
                    title: tab.title,
                    active: tab.active,
                });
            }
        }

        tab_layouts
    }

    pub(in crate::app) fn table_preview(ui: &mut Ui<'_>, panel: Rect) {
        panel_title(ui, panel, "Virtualized Table Model");
        let table_semantic_id = ui.id("layout.table-preview.semantic");
        ui.push_semantic_node(
            SemanticNode::new(table_semantic_id, SemanticRole::Table, panel)
                .with_label("Virtualized Table Model"),
        );
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 220.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "State".to_owned(),
                    width: 160.0,
                },
                TableColumn {
                    id: ItemId::from_raw(3),
                    header: "Latency".to_owned(),
                    width: 120.0,
                },
                TableColumn {
                    id: ItemId::from_raw(4),
                    header: "Owner".to_owned(),
                    width: 180.0,
                },
            ],
            header_height: 30.0,
            row_height: 28.0,
            sort: None,
        };
        let max_table_width = (panel.width - 48.0).max(0.0);
        let preferred_table_width = (panel.width * 0.62).clamp(0.0, 680.0);
        let table_width = if max_table_width < 420.0 {
            max_table_width
        } else {
            preferred_table_width.clamp(420.0, max_table_width)
        };
        let bounds = Rect::new(
            panel.x + 24.0,
            panel.y + 50.0,
            table_width,
            (panel.height - 90.0).max(120.0),
        );
        for header in table.header_rects(bounds) {
            rect(ui, header.rect, rgb(34, 34, 38), Some(rgb(72, 72, 76)));
            let column = &table.columns[header.index];
            text(
                ui,
                header.rect.x + 10.0,
                header.rect.y + 20.0,
                &column.header,
                10.0,
                rgb(236, 236, 238),
            );
        }
        for cell in table.cell_rects(bounds, 7, 0..7) {
            rect(ui, cell.rect, rgb(22, 22, 25), Some(rgb(52, 52, 58)));
            let row = cell.index / table.columns.len();
            let column = cell.index % table.columns.len();
            let value = match column {
                0 => format!("Item {row:02}"),
                1 => {
                    if row.is_multiple_of(2) {
                        "Ready".to_owned()
                    } else {
                        "Queued".to_owned()
                    }
                }
                2 => format!("{} ms", 12 + row * 7),
                _ => format!("Team {}", row % 3 + 1),
            };
            text(
                ui,
                cell.rect.x + 10.0,
                cell.rect.y + 18.0,
                &value,
                9.0,
                rgb(210, 210, 214),
            );
        }

        text(
            ui,
            bounds.max_x() + 56.0,
            bounds.y + 30.0,
            "Rows: 7 | Columns: 4 | Overscan: 0",
            11.0,
            rgb(190, 190, 194),
        );
    }
}

fn dock_preview_tab_rect(frame: Rect, index: usize, tab_count: usize) -> Rect {
    if tab_count == 0 {
        return Rect::new(frame.x, frame.y, 0.0, 0.0);
    }

    let frame_width = frame.width.max(0.0);
    let frame_height = frame.height.max(0.0);
    let horizontal_inset = DOCK_PREVIEW_TAB_INSET.min(frame_width * 0.5);
    let vertical_inset = DOCK_PREVIEW_TAB_INSET.min(frame_height * 0.5);
    let inner_width = (frame_width - horizontal_inset * 2.0).max(0.0);
    let gap_count = tab_count.saturating_sub(1);
    let gap = if gap_count == 0 {
        0.0
    } else {
        DOCK_PREVIEW_TAB_GAP.min(inner_width / gap_count as f32)
    };
    let available_width = (inner_width - gap * gap_count as f32).max(0.0);
    let tab_width = (available_width / tab_count as f32).min(DOCK_PREVIEW_TAB_MAX_WIDTH);
    let tab_height = (frame_height - vertical_inset * 2.0).clamp(0.0, DOCK_PREVIEW_TAB_HEIGHT);

    Rect::new(
        frame.x + horizontal_inset + index as f32 * (tab_width + gap),
        frame.y + frame_height - vertical_inset - tab_height,
        tab_width,
        tab_height,
    )
}
