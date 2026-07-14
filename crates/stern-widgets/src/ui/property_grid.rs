use std::hash::Hash;

use stern_core::{
    Brush, ClipId, Color, InteractionState, Point, Primitive, Rect, RectPrimitive, RepaintRequest,
    Response, ScrollResponse, SemanticNode, SemanticRole, SemanticValue, Size, Stroke,
    TextPrimitive, TextRole, Vec2, WidgetId, scrollable,
};

use super::Ui;
use crate::inspector::{
    PropertyGridAccess, PropertyGridCell, PropertyGridConfig, PropertyGridError,
    PropertyGridIntent, PropertyGridLayout, PropertyGridOutput, PropertyGridRow,
    PropertyGridRowKind, PropertyGridRowRect, PropertyGridStatusSeverity, PropertyGridValueOutput,
    property_grid_row_affordance_rects, property_grid_row_status_semantics,
    property_grid_row_widget_id, property_grid_value_widget_id,
};

impl Ui<'_> {
    /// Paints one live, scrollable property grid.
    ///
    /// The callback composes the current application-owned value control for
    /// each visible property row. Reset and keyframe operations are returned
    /// as intents; this component never mutates domain state.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyGridError::DuplicateRowId`] when row identities are
    /// not unique.
    pub fn property_grid<'rows, T>(
        &mut self,
        key: impl Hash,
        bounds: Rect,
        rows: &'rows [PropertyGridRow],
        config: PropertyGridConfig,
        mut value: impl FnMut(&mut Self, PropertyGridCell<'rows>) -> T,
    ) -> Result<PropertyGridOutput<T>, PropertyGridError> {
        PropertyGridLayout::validate_rows(rows)?;
        let root = self.runtime.push_id_scope(key);
        let output = self.property_grid_in_scope(root, bounds, rows, config, &mut value);
        self.runtime.pop_id_scope();
        Ok(output)
    }

    #[allow(clippy::too_many_lines)]
    fn property_grid_in_scope<'rows, T>(
        &mut self,
        root: WidgetId,
        bounds: Rect,
        rows: &'rows [PropertyGridRow],
        config: PropertyGridConfig,
        value: &mut impl FnMut(&mut Self, PropertyGridCell<'rows>) -> T,
    ) -> PropertyGridOutput<T> {
        let Some(bounds) = sanitize_property_grid_bounds(bounds) else {
            return PropertyGridOutput {
                root,
                scroll: empty_property_grid_scroll(self.make_id("property-grid-scroll")),
                visible_rows: Vec::new(),
                values: Vec::new(),
                intents: Vec::new(),
            };
        };
        let scroll_id = self.id("property-grid-scroll");
        let content_size = Size::new(
            bounds.width.max(0.0),
            config
                .layout
                .content_height(rows)
                .max(bounds.height.max(0.0)),
        );
        let frame_offset = stern_core::clamp_scroll_offset(
            self.memory().scroll_offset(scroll_id),
            bounds.size(),
            content_size,
        );
        let freeze_offset =
            self.memory().pointer_wheel_route() != stern_core::PointerRoute::Unplanned;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let scroll = scrollable(
            scroll_id,
            bounds,
            content_size,
            input,
            memory,
            config.disabled,
        );
        if scroll.delta != Vec2::ZERO {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        let content_offset = if freeze_offset {
            frame_offset
        } else {
            scroll.offset
        };

        let mut root_node =
            SemanticNode::new(root, SemanticRole::Grid, bounds).with_label("Property grid");
        root_node.state.disabled = config.disabled;
        self.push_semantic_node(root_node);

        let visible_rows = config
            .layout
            .visible_row_rects(bounds, rows, content_offset.y, config.overscan)
            .into_iter()
            .filter(|geometry| geometry.rect.intersection(bounds).is_some())
            .collect::<Vec<_>>();
        let clip = ClipId::from_raw(root.child("property-grid-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: bounds,
        });
        let mut values = Vec::new();
        let mut intents = Vec::new();
        for geometry in &visible_rows {
            let row = &rows[geometry.index];
            let row_id = self
                .runtime
                .push_id_scope(("property-grid-row", row.id.raw()));
            debug_assert_eq!(row_id, property_grid_row_widget_id(root, row.id));
            let access = effective_access(row, config.disabled);
            let effective_row = if config.disabled && !row.state.disabled {
                row.clone().with_disabled(true)
            } else {
                row.clone()
            };

            self.paint_property_grid_row(&effective_row, *geometry, access);
            self.push_property_grid_row_semantics(row_id, &effective_row, *geometry, access);

            if matches!(row.kind, PropertyGridRowKind::Section) {
                self.runtime.pop_id_scope();
                continue;
            }
            let affordance_rects = property_grid_row_affordance_rects(
                &effective_row,
                geometry
                    .value_rect
                    .intersection(bounds)
                    .unwrap_or(Rect::ZERO)
                    .inset(2.0)
                    .max_zero(),
                config.affordances,
            );
            let cell =
                PropertyGridCell::new(root, row, *geometry, affordance_rects.value_rect, access);
            let value_id = self.runtime.push_id_scope("value");
            debug_assert_eq!(value_id, property_grid_value_widget_id(root, row.id));
            values.push(PropertyGridValueOutput {
                row: row.id,
                value: value(self, cell),
            });
            self.runtime.pop_id_scope();

            let affordance = self.property_grid_row_affordance_controls(
                "affordances",
                &effective_row,
                affordance_rects,
            );
            if affordance.reset_requested {
                intents.push(PropertyGridIntent::Reset { row: row.id });
            }
            if affordance.keyframe_toggle_requested {
                intents.push(PropertyGridIntent::SetKeyed {
                    row: row.id,
                    keyed: affordance.requested_keyed,
                });
            }
            self.runtime.pop_id_scope();
        }
        self.primitive(Primitive::ClipEnd { id: clip });

        PropertyGridOutput {
            root,
            scroll,
            visible_rows,
            values,
            intents,
        }
    }

    fn paint_property_grid_row(
        &mut self,
        row: &PropertyGridRow,
        geometry: PropertyGridRowRect,
        access: PropertyGridAccess,
    ) {
        let section = matches!(row.kind, PropertyGridRowKind::Section);
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: geometry.rect,
            fill: Some(Brush::Solid(if section {
                self.theme.colors.surface.raised
            } else {
                self.theme.colors.surface.sunken
            })),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));

        let presentation = row.state.status.presentation();
        let status_color = property_status_color(self.theme, presentation.severity);
        if presentation.accented {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: Rect::new(geometry.rect.x, geometry.rect.y, 3.0, geometry.rect.height),
                fill: Some(Brush::Solid(status_color)),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }

        let mut label = row.label.clone();
        if row.state.required && !section {
            label.push_str(" *");
        }
        let label_color = if access.disabled() {
            self.theme.colors.content.disabled
        } else if presentation.accented {
            status_color
        } else if section {
            self.theme.colors.content.primary
        } else {
            self.theme.colors.content.muted
        };
        self.paint_property_grid_text(
            Point::new(
                geometry.label_rect.x + if section { 8.0 } else { 6.0 },
                text_baseline(geometry.label_rect, self.theme.font(TextRole::Label).size),
            ),
            label,
            label_color,
        );
        if row.state.help_text.is_some() {
            self.paint_property_grid_text(
                Point::new(
                    geometry.label_rect.max_x() - 22.0,
                    text_baseline(geometry.label_rect, self.theme.font(TextRole::Label).size),
                ),
                "?".to_owned(),
                self.theme.colors.content.muted,
            );
        }
        if presentation.accented {
            let glyph = match presentation.severity {
                PropertyGridStatusSeverity::Info => "i",
                PropertyGridStatusSeverity::Warning => "!",
                PropertyGridStatusSeverity::Error => "x",
                PropertyGridStatusSeverity::None => "",
            };
            self.paint_property_grid_text(
                Point::new(
                    geometry.label_rect.max_x() - 10.0,
                    text_baseline(geometry.label_rect, self.theme.font(TextRole::Label).size),
                ),
                glyph.to_owned(),
                status_color,
            );
        }
    }

    fn paint_property_grid_text(&mut self, origin: Point, text: String, color: Color) {
        let font = self.theme.font(TextRole::Label);
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin,
            text,
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(color),
        }));
    }

    fn push_property_grid_row_semantics(
        &mut self,
        row_id: stern_core::WidgetId,
        row: &PropertyGridRow,
        geometry: PropertyGridRowRect,
        access: PropertyGridAccess,
    ) {
        let role = if matches!(row.kind, PropertyGridRowKind::Section) {
            SemanticRole::Label
        } else {
            SemanticRole::Row
        };
        let mut node = SemanticNode::new(row_id, role, geometry.rect).with_label(&row.label);
        node.state.disabled = access.disabled();
        let mut descriptions = Vec::new();
        if let Some(help) = row.state.help_text.as_ref().filter(|help| !help.is_empty()) {
            descriptions.push(format!("Help: {help}"));
            let help_id = row_id.child("help");
            let mut help_node =
                SemanticNode::new(help_id, SemanticRole::Label, geometry.label_rect)
                    .with_label(format!("{} help", row.label));
            help_node.description = Some(help.clone());
            help_node.state.value = Some(SemanticValue::Text(help.clone()));
            help_node.state.disabled = access.disabled();
            self.push_semantic_node(help_node);
        }
        if let Some(status) = row.state.status.semantic_text() {
            descriptions.push(status);
        }
        if access.read_only() {
            descriptions.push("Read only".to_owned());
        }
        if !descriptions.is_empty() {
            node.description = Some(descriptions.join(". "));
        }
        self.push_semantic_node(node);
        if let Some(status) = property_grid_row_status_semantics(row_id, row, geometry) {
            self.push_semantic_node(status);
        }
    }
}

fn effective_access(row: &PropertyGridRow, grid_disabled: bool) -> PropertyGridAccess {
    if grid_disabled || row.state.disabled {
        PropertyGridAccess::Disabled
    } else if row.state.read_only {
        PropertyGridAccess::ReadOnly
    } else {
        PropertyGridAccess::Editable
    }
}

fn property_status_color(theme: &stern_core::Theme, severity: PropertyGridStatusSeverity) -> Color {
    match severity {
        PropertyGridStatusSeverity::None => theme.colors.content.muted,
        PropertyGridStatusSeverity::Info => theme.colors.status.info.strong,
        PropertyGridStatusSeverity::Warning => theme.colors.status.warning.strong,
        PropertyGridStatusSeverity::Error => theme.colors.status.danger.strong,
    }
}

fn text_baseline(rect: Rect, size: f32) -> f32 {
    rect.y + (rect.height - size).max(0.0) * 0.5 + size
}

fn sanitize_property_grid_bounds(bounds: Rect) -> Option<Rect> {
    if bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0
    {
        Some(bounds)
    } else {
        None
    }
}

fn empty_property_grid_scroll(id: WidgetId) -> ScrollResponse {
    ScrollResponse {
        response: Response::new(
            id,
            Rect::ZERO,
            InteractionState {
                disabled: true,
                ..InteractionState::default()
            },
        ),
        offset: Vec2::ZERO,
        delta: Vec2::ZERO,
        max_offset: Vec2::ZERO,
    }
}
