use kinetik_ui_core::{
    Brush, ClipId, ComponentState, Point, Primitive, Rect, RectPrimitive, SemanticNode,
    SemanticRole, Stroke, TextPrimitive, TextRole,
};

use super::Ui;
use crate::dock::{DockScene, DockSceneFrame, DockScenePanel, DockScenePreviewKind, DockSceneTab};

impl Ui<'_> {
    /// Paints one prepared public Dock → Frame → Panel scene.
    ///
    /// The callback runs exactly once for each active panel body with positive
    /// area, in deterministic dock-tree order, under that panel's exact clip.
    /// This presentation packet does not mutate the caller-owned [`crate::dock::Dock`].
    pub fn dock_scene<T>(
        &mut self,
        scene: &DockScene,
        mut panel_content: impl FnMut(&mut Self, &DockScenePanel) -> T,
    ) -> Vec<T> {
        let layout = scene.layout();
        if layout.bounds == Rect::ZERO {
            return Vec::new();
        }

        self.register_id(scene.root_widget_id());
        self.paint_dock_root(layout.bounds);
        self.push_semantic_node(
            SemanticNode::new(scene.root_widget_id(), SemanticRole::Dock, layout.bounds)
                .with_label("Editor dock")
                .with_children(layout.frames.iter().map(|frame| frame.id)),
        );

        let mut output = Vec::with_capacity(layout.frames.len());
        for frame in &layout.frames {
            self.paint_dock_frame(frame, scene.config().disabled);
            if let Some(panel) = &frame.panel {
                self.register_id(panel.id);
                self.paint_dock_panel(panel.rect);
                self.push_semantic_node(
                    SemanticNode::new(panel.id, SemanticRole::Panel, panel.rect)
                        .with_label(&panel.title),
                );

                let clip = ClipId::from_raw(panel.id.child("clip").raw());
                self.primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: panel.rect,
                });
                self.runtime
                    .push_id_scope(("dock-panel-content", panel.id.raw()));
                output.push(panel_content(self, panel));
                self.runtime.pop_id_scope();
                self.primitive(Primitive::ClipEnd { id: clip });
            }
        }

        for splitter in &layout.splitters {
            self.register_id(splitter.id);
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: splitter.rect,
                fill: Some(Brush::Solid(self.theme.colors.border)),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }

        if let Some(preview) = layout.preview {
            self.register_id(preview.id);
            let (alpha, radius) = match preview.kind {
                DockScenePreviewKind::Merge => (0.20, self.theme.radii.sm),
                DockScenePreviewKind::Split(_) => (0.32, self.theme.radii.none),
            };
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: preview.rect,
                fill: Some(Brush::Solid(self.theme.colors.accent.with_alpha(alpha))),
                stroke: Some(Stroke::new(
                    self.theme.controls.border_width.max(1.0),
                    Brush::Solid(self.theme.colors.accent),
                )),
                radius,
            }));
        }

        output
    }

    fn paint_dock_root(&mut self, rect: Rect) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(self.theme.colors.surface_sunken)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border_subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_dock_frame(&mut self, frame: &DockSceneFrame, disabled: bool) {
        self.register_id(frame.id);
        let fill = if frame.active {
            self.theme.colors.surface_raised
        } else {
            self.theme.colors.surface_sunken
        };
        let border = if frame.active {
            self.theme.colors.focus_ring
        } else {
            self.theme.colors.border_subtle
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: frame.rect,
            fill: Some(Brush::Solid(fill)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(border),
            )),
            radius: self.theme.radii.none,
        }));

        self.register_id(frame.tab_list_id);
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: frame.tab_list_rect,
            fill: Some(Brush::Solid(self.theme.colors.surface_sunken)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border_subtle),
            )),
            radius: self.theme.radii.none,
        }));

        let mut frame_node = SemanticNode::new(frame.id, SemanticRole::Frame, frame.rect)
            .with_label(
                frame
                    .panel
                    .as_ref()
                    .map_or("Empty frame", |panel| panel.title.as_str()),
            )
            .with_children(
                core::iter::once(frame.tab_list_id).chain(frame.panel.iter().map(|panel| panel.id)),
            );
        frame_node.state.selected = frame.active;
        frame_node.state.disabled = disabled;
        self.push_semantic_node(frame_node);

        self.push_semantic_node(
            SemanticNode::new(
                frame.tab_list_id,
                SemanticRole::TabList,
                frame.tab_list_rect,
            )
            .with_label("Frame tabs")
            .with_children(frame.tabs.iter().map(|tab| tab.id)),
        );

        let strip_clip = ClipId::from_raw(frame.tab_list_id.child("clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: strip_clip,
            rect: frame.tab_list_rect,
        });
        for tab in &frame.tabs {
            self.paint_dock_tab(tab, disabled);
        }
        self.primitive(Primitive::ClipEnd { id: strip_clip });
    }

    fn paint_dock_tab(&mut self, tab: &DockSceneTab, disabled: bool) {
        self.register_id(tab.id);
        let recipe = self.theme.tab(ComponentState {
            disabled,
            selected: tab.selected,
            ..ComponentState::default()
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: tab.rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        if let Some(indicator) = recipe.indicator {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: Rect::new(
                    tab.rect.x,
                    tab.rect.max_y() - recipe.indicator_thickness,
                    tab.rect.width,
                    recipe.indicator_thickness,
                ),
                fill: Some(indicator),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }

        let tab_clip = ClipId::from_raw(tab.id.child("clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: tab_clip,
            rect: tab.rect,
        });
        let font = self.theme.font(TextRole::Label);
        let extra = (tab.rect.height - font.line_height).max(0.0) * 0.5;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                tab.rect.x + self.theme.controls.padding_x,
                tab.rect.y + extra + font.size,
            ),
            text: tab.title.clone(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));
        if let Some(close_rect) = tab.close_rect {
            self.register_id(tab.close_id);
            self.primitive(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(
                    close_rect.x + self.theme.controls.padding_x * 0.5,
                    close_rect.y + extra + font.size,
                ),
                text: "×".to_owned(),
                family: font.family.to_owned(),
                size: font.size,
                line_height: font.line_height,
                brush: Brush::Solid(recipe.foreground),
            }));
        }
        self.primitive(Primitive::ClipEnd { id: tab_clip });

        let mut node =
            SemanticNode::new(tab.id, SemanticRole::Tab, tab.rect).with_label(&tab.title);
        node.state.selected = tab.selected;
        node.state.disabled = disabled;
        self.push_semantic_node(node);
    }

    fn paint_dock_panel(&mut self, rect: Rect) {
        let recipe = self.theme.panel();
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
    }
}
