use std::hash::Hash;

use kinetik_ui_core::{
    Brush, ClipId, ComponentState, Point, Primitive, Rect, RectPrimitive, RepaintRequest,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, Stroke,
    TextPrimitive, TextRole, Transform, Vec2, scrollable,
};

use super::Ui;
use crate::collections::{
    CollectionProjectedItem, CollectionProjection, ItemId, SortDirection, TableColumn, TableSort,
    VirtualTable, VirtualTableConfig, VirtualTableHeaderResponse, VirtualTableMaterializedRow,
    VirtualTableOutput, VirtualTableRow,
};

impl Ui<'_> {
    /// Prepares one fixed-height virtual-table frame before pointer arbitration.
    ///
    /// Returns `None` for invalid viewport/header/row geometry, empty or
    /// duplicate columns, and non-positive effective column widths.
    #[must_use]
    pub fn prepare_virtual_table<'table>(
        &self,
        key: impl Hash,
        config: VirtualTableConfig,
        projection: &'table CollectionProjection,
    ) -> Option<VirtualTable<'table>> {
        let root = self.make_id(key);
        VirtualTable::prepare(root, config, projection, self.memory().scroll_offset(root))
    }

    /// Paints and evaluates a prepared fixed-height virtual table.
    ///
    /// The callback runs once per prepared materialized body row. Header clicks
    /// emit sort requests; application data and projection order stay caller-owned.
    #[allow(clippy::too_many_lines)]
    pub fn virtual_table(
        &mut self,
        table: &VirtualTable<'_>,
        mut row: impl FnMut(CollectionProjectedItem) -> VirtualTableRow,
    ) -> VirtualTableOutput {
        let root = table.widget_id();
        let config = table.config();
        self.register_id(root);

        let scroll = {
            let (input, memory) = self.runtime.input_and_memory_mut();
            scrollable(
                root,
                config.bounds,
                table.content_size(),
                input,
                memory,
                config.disabled,
            )
        };
        if scroll.delta != Vec2::ZERO {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        let mut output = VirtualTableOutput {
            scroll,
            window: table.window().clone(),
            sort_requested: None,
            headers: Vec::with_capacity(table.headers().len()),
            rows: Vec::with_capacity(table.rows().len()),
        };

        self.paint_virtual_table_surface(config.bounds);
        let visible_rows = table
            .rows()
            .iter()
            .filter(|row| table.row_is_visible(row))
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let mut root_children = Vec::with_capacity(visible_rows.len().saturating_add(1));
        root_children.push(table.header_row_widget_id());
        root_children.extend(visible_rows);
        let mut root_semantics = SemanticNode::new(root, SemanticRole::Table, config.bounds)
            .with_label(&config.label)
            .with_children(root_children);
        root_semantics.state.disabled = config.disabled;
        self.push_semantic_node(root_semantics);

        let header_clip_id = ClipId::from_raw(root.child("virtual-table-header-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: header_clip_id,
            rect: table.header_clip(),
        });
        self.primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(-table.window().offset.x, 0.0),
        )));

        let header_row_id = table.header_row_widget_id();
        self.register_id(header_row_id);
        let visible_headers = table
            .headers()
            .iter()
            .filter(|header| table.header_is_visible(header))
            .map(|header| header.id)
            .collect::<Vec<_>>();
        let mut header_row_semantics = SemanticNode::new(
            header_row_id,
            SemanticRole::Row,
            Rect::new(
                config.bounds.x,
                config.bounds.y,
                table.total_width(),
                config.layout.effective_header_height(),
            ),
        )
        .with_label("Column headers")
        .with_children(visible_headers);
        header_row_semantics.state.disabled = config.disabled;
        self.push_semantic_node(header_row_semantics);

        for header in table.headers() {
            self.register_id(header.id);
            let response = self.pressable_with_id(header.id, header.cell.rect, config.disabled);
            let column = &config.layout.columns[header.cell.column];
            if response.clicked {
                output
                    .sort_requested
                    .get_or_insert_with(|| next_table_sort(config.layout.sort, column.id));
                self.request_repaint(RepaintRequest::NextFrame);
            }
            self.paint_virtual_table_header(header.cell.rect, column, response, config.layout.sort);
            if table.header_is_visible(header) {
                self.push_semantic_node(virtual_table_header_semantics(
                    header.id,
                    header.cell.rect,
                    column,
                    config.layout.sort,
                    config.disabled,
                ));
            }
            output.headers.push(VirtualTableHeaderResponse {
                column: column.id,
                response,
            });
        }

        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: header_clip_id });

        let body_clip_id = ClipId::from_raw(root.child("virtual-table-body-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: body_clip_id,
            rect: table.body_clip(),
        });
        self.primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(-table.window().offset.x, -table.window().offset.y),
        )));

        for projected in table.rows() {
            self.register_id(projected.id);
            let presentation = row(projected.item);
            let visible_cells = projected
                .cells
                .iter()
                .filter(|cell| table.cell_is_visible(cell))
                .map(|cell| cell.id)
                .collect::<Vec<_>>();
            if table.row_is_visible(projected) {
                let mut row_semantics =
                    SemanticNode::new(projected.id, SemanticRole::Row, projected.rect)
                        .with_label(format!("Row {}", projected.item.id.raw()))
                        .with_children(visible_cells);
                row_semantics.state.disabled = config.disabled;
                self.push_semantic_node(row_semantics);
            }

            for cell in &projected.cells {
                self.register_id(cell.id);
                let label = presentation
                    .cells
                    .get(cell.cell.column)
                    .map_or("", String::as_str);
                self.paint_virtual_table_cell(cell.cell.rect, label);
                if table.cell_is_visible(cell) {
                    let mut cell_semantics =
                        SemanticNode::new(cell.id, SemanticRole::Cell, cell.cell.rect)
                            .with_label(label);
                    cell_semantics.state.disabled = config.disabled;
                    self.push_semantic_node(cell_semantics);
                }
            }
            output.rows.push(VirtualTableMaterializedRow {
                id: projected.item.id,
                projected_index: projected.projected_index,
            });
        }

        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: body_clip_id });
        output
    }

    fn paint_virtual_table_surface(&mut self, rect: Rect) {
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

    fn paint_virtual_table_header(
        &mut self,
        rect: Rect,
        column: &TableColumn,
        response: kinetik_ui_core::Response,
        sort: Option<TableSort>,
    ) {
        let selected = sort.is_some_and(|sort| sort.column == column.id);
        let recipe = self.theme.row(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: response.state.disabled,
            selected,
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        let label = table_header_label(column, sort);
        self.paint_virtual_table_text(rect, &label, recipe.foreground);
    }

    fn paint_virtual_table_cell(&mut self, rect: Rect, label: &str) {
        let recipe = self.theme.row(ComponentState::default());
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        self.paint_virtual_table_text(rect, label, recipe.foreground);
    }

    fn paint_virtual_table_text(&mut self, rect: Rect, label: &str, color: kinetik_ui_core::Color) {
        let font = self.theme.font(TextRole::Label);
        let extra = (rect.height - font.line_height).max(0.0) * 0.5;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                rect.x + self.theme.controls.padding_x,
                rect.y + extra + font.size,
            ),
            text: label.to_owned(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(color),
        }));
    }
}

fn next_table_sort(current: Option<TableSort>, column: ItemId) -> TableSort {
    let direction = match current {
        Some(TableSort {
            column: active,
            direction: SortDirection::Ascending,
        }) if active == column => SortDirection::Descending,
        Some(TableSort {
            column: active,
            direction: SortDirection::Descending,
        }) if active == column => SortDirection::Ascending,
        _ => SortDirection::Ascending,
    };
    TableSort { column, direction }
}

fn table_header_label(column: &TableColumn, sort: Option<TableSort>) -> String {
    match sort.filter(|sort| sort.column == column.id) {
        Some(TableSort {
            direction: SortDirection::Ascending,
            ..
        }) => format!("{} ↑", column.header),
        Some(TableSort {
            direction: SortDirection::Descending,
            ..
        }) => format!("{} ↓", column.header),
        None => column.header.clone(),
    }
}

fn virtual_table_header_semantics(
    id: kinetik_ui_core::WidgetId,
    rect: Rect,
    column: &TableColumn,
    sort: Option<TableSort>,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::Cell, rect)
        .with_label(table_header_label(column, sort));
    node.state.disabled = disabled;
    if let Some(sort) = sort.filter(|sort| sort.column == column.id) {
        node.state.value = Some(SemanticValue::Text(match sort.direction {
            SortDirection::Ascending => "Sorted ascending".to_owned(),
            SortDirection::Descending => "Sorted descending".to_owned(),
        }));
    }
    if !disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            format!("Sort by {}", column.header),
        ));
    }
    node
}
