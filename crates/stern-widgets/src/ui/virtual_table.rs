use std::hash::Hash;

use stern_core::{
    Brush, ClipId, ComponentState, Key, KeyState, Point, Primitive, Rect, RectPrimitive,
    RepaintRequest, Response, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    SemanticValue, Stroke, TextPrimitive, TextRole, Transform, Vec2, scrollable,
};

use super::Ui;
use crate::collections::{
    CollectionProjectedItem, CollectionProjection, ItemId, SortDirection, TableColumn,
    TableColumnResizeRequest, TableSort, VirtualTable, VirtualTableConfig, VirtualTableCursorMove,
    VirtualTableCursorTarget, VirtualTableHeaderResponse, VirtualTableMaterializedRow,
    VirtualTableOutput, VirtualTableRow, VirtualTableSelection, VirtualTableSelectionMode,
    VirtualTableSelectionResponse, VirtualTableTarget,
};

impl Ui<'_> {
    /// Prepares one fixed-height virtual-table frame before pointer arbitration.
    ///
    /// Returns `None` for invalid viewport/header/row geometry, empty or
    /// duplicate columns, and resizable columns narrower than one logical pixel.
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
    /// The callback runs once per prepared materialized body row. The caller
    /// retains selection and applies emitted sort or resize requests to future
    /// frames; current prepared geometry remains frozen.
    #[allow(clippy::too_many_lines)]
    pub fn virtual_table(
        &mut self,
        table: &VirtualTable<'_>,
        selection: &mut VirtualTableSelection,
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
            resize_requested: None,
            selection_changed: false,
            cursor_target: None,
            headers: Vec::with_capacity(table.headers().len()),
            selection_responses: Vec::new(),
            rows: Vec::with_capacity(table.rows().len()),
        };

        let old_target = selection.target();
        let old_projected_row = selection.last_projected_row();
        let old_column = selection.last_column();
        let old_focused = old_target
            .is_some_and(|target| self.memory().is_focused(table.target_widget_id(target)));
        output.cursor_target = selection.reconcile(
            table.projection(),
            &config.layout.columns,
            config.selection_mode,
        );
        let cursor_reconciled = old_target != selection.target()
            || old_projected_row != selection.last_projected_row()
            || old_column != selection.last_column();
        output.selection_changed |= old_target != selection.target();
        if old_focused && cursor_reconciled {
            if let Some(target) = output.cursor_target {
                self.focus_and_reveal_virtual_table_target(table, target);
            } else {
                self.runtime.memory_mut().clear_focus();
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        if !config.disabled
            && selection
                .target()
                .is_some_and(|target| self.memory().is_focused(table.target_widget_id(target)))
        {
            let events = self.input().keyboard.events.clone();
            let page_rows = virtual_table_page_rows(config);
            let mut final_focus_target = None;
            for event in events {
                if event.state != KeyState::Pressed || event.modifiers.alt {
                    continue;
                }
                let Some(movement) =
                    virtual_table_movement(config.selection_mode, event.key, page_rows)
                else {
                    continue;
                };
                let before = selection.target();
                if let Some(target) = selection.navigate(
                    table.projection(),
                    &config.layout.columns,
                    config.selection_mode,
                    movement,
                ) {
                    output.selection_changed |= before != selection.target();
                    output.cursor_target = Some(target);
                    final_focus_target = Some(target);
                }
            }
            if let Some(target) = final_focus_target {
                self.focus_and_reveal_virtual_table_target(table, target);
            }
        }

        if let Some(target) = selection.target()
            && !table.contains_materialized_target(target)
        {
            self.register_id(table.target_widget_id(target));
        }

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
            let resize_response = if config.resizable {
                let handle = &table.resize_handles()[header.cell.column];
                self.register_id(handle.id);
                let gesture = self.runtime.captured_domain_drag_gesture(
                    handle.id,
                    handle.rect,
                    config.disabled,
                );
                let resize_response = gesture.response;
                if resize_response.dragged
                    && let Some(delta) =
                        table.constrained_resize_delta(handle.column, resize_response.drag_delta.x)
                {
                    output
                        .resize_requested
                        .get_or_insert(TableColumnResizeRequest {
                            column: handle.column,
                            delta,
                        });
                }
                if resize_response.state.pressed || resize_response.dragged {
                    self.request_repaint(RepaintRequest::NextFrame);
                }
                self.paint_virtual_table_resize_handle(handle.rect, resize_response);
                Some(resize_response)
            } else {
                None
            };
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
                resize_response,
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
            let row_target = VirtualTableTarget::Row(projected.item.id);
            let mut row_response = None;
            if config.selection_mode == VirtualTableSelectionMode::Row {
                let (response, cursor_target, changed) = self.capture_virtual_table_target(
                    table,
                    selection,
                    row_target,
                    projected.id,
                    projected.rect,
                );
                row_response = Some(response);
                output.selection_changed |= changed;
                if cursor_target.is_some() {
                    output.cursor_target = cursor_target;
                }
            }

            let mut cell_responses = Vec::with_capacity(projected.cells.len());
            if config.selection_mode == VirtualTableSelectionMode::Cell {
                for cell in &projected.cells {
                    self.register_id(cell.id);
                    let target = VirtualTableTarget::Cell {
                        row: projected.item.id,
                        column: cell.cell.column_id,
                    };
                    let (response, cursor_target, changed) = self.capture_virtual_table_target(
                        table,
                        selection,
                        target,
                        cell.id,
                        cell.cell.rect,
                    );
                    output.selection_changed |= changed;
                    if cursor_target.is_some() {
                        output.cursor_target = cursor_target;
                    }
                    cell_responses.push((target, response));
                }
            }

            if let Some(response) = &mut row_response {
                refresh_virtual_table_response(
                    response,
                    selection.target() == Some(row_target),
                    self.memory().is_focused(projected.id),
                );
            }
            for (target, response) in &mut cell_responses {
                refresh_virtual_table_response(
                    response,
                    selection.target() == Some(*target),
                    self.memory().is_focused(table.target_widget_id(*target)),
                );
            }

            let visible_cells = projected
                .cells
                .iter()
                .filter(|cell| table.cell_is_visible(cell))
                .map(|cell| cell.id)
                .collect::<Vec<_>>();
            if table.row_is_visible(projected) {
                self.push_semantic_node(virtual_table_body_semantics(
                    projected.id,
                    SemanticRole::Row,
                    projected.rect,
                    &format!("Row {}", projected.item.id.raw()),
                    visible_cells,
                    row_response,
                    config.disabled,
                ));
            }

            for (cell_index, cell) in projected.cells.iter().enumerate() {
                self.register_id(cell.id);
                let label = presentation
                    .cells
                    .get(cell.cell.column)
                    .map_or("", String::as_str);
                let response = row_response.or_else(|| {
                    cell_responses
                        .get(cell_index)
                        .map(|(_, response)| *response)
                });
                let Some(response) = response else {
                    continue;
                };
                self.paint_virtual_table_cell(cell.cell.rect, label, response, config.disabled);
                if table.cell_is_visible(cell) {
                    let selection_response = (config.selection_mode
                        == VirtualTableSelectionMode::Cell)
                        .then(|| {
                            cell_responses
                                .get(cell_index)
                                .map(|(_, response)| *response)
                        })
                        .flatten();
                    self.push_semantic_node(virtual_table_body_semantics(
                        cell.id,
                        SemanticRole::Cell,
                        cell.cell.rect,
                        label,
                        Vec::new(),
                        selection_response,
                        config.disabled,
                    ));
                }
            }
            match config.selection_mode {
                VirtualTableSelectionMode::Row => {
                    if let Some(response) = row_response {
                        output
                            .selection_responses
                            .push(VirtualTableSelectionResponse {
                                target: row_target,
                                response,
                            });
                    }
                }
                VirtualTableSelectionMode::Cell => {
                    output
                        .selection_responses
                        .extend(cell_responses.into_iter().map(|(target, response)| {
                            VirtualTableSelectionResponse { target, response }
                        }));
                }
            }
            output.rows.push(VirtualTableMaterializedRow {
                id: projected.item.id,
                projected_index: projected.projected_index,
            });
        }

        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: body_clip_id });
        if output.selection_changed || output.resize_requested.is_some() {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn capture_virtual_table_target(
        &mut self,
        table: &VirtualTable<'_>,
        selection: &mut VirtualTableSelection,
        target: VirtualTableTarget,
        id: stern_core::WidgetId,
        rect: Rect,
    ) -> (Response, Option<VirtualTableCursorTarget>, bool) {
        let config = table.config();
        let gesture = self
            .runtime
            .captured_selection_gesture(id, rect, config.disabled);
        let mut response = gesture.response;
        let before = selection.target();
        let cursor_target = response
            .clicked
            .then(|| {
                selection.activate(
                    table.projection(),
                    &config.layout.columns,
                    target,
                    config.selection_mode,
                )
            })
            .flatten();
        if let Some(cursor_target) = cursor_target {
            self.focus_and_reveal_virtual_table_target(table, cursor_target);
        }
        refresh_virtual_table_response(
            &mut response,
            selection.target() == Some(target),
            self.memory().is_focused(id),
        );
        if response.clicked || response.double_clicked || response.state.pressed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        (response, cursor_target, before != selection.target())
    }

    fn focus_and_reveal_virtual_table_target(
        &mut self,
        table: &VirtualTable<'_>,
        target: VirtualTableCursorTarget,
    ) {
        let id = table.target_widget_id(target.target);
        if !table.contains_materialized_target(target.target) {
            self.register_id(id);
        }
        let reveal = table.revealed_offset(target);
        let focus_changed = !self.memory().is_focused(id);
        let reveal_changed = reveal.x.to_bits() != table.window().offset.x.to_bits()
            || reveal.y.to_bits() != table.window().offset.y.to_bits();
        let memory = self.runtime.memory_mut();
        if focus_changed {
            memory.focus(id);
        }
        if reveal_changed {
            memory.stage_scroll_offset(table.widget_id(), reveal);
        }
        if focus_changed || reveal_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
    }

    fn paint_virtual_table_surface(&mut self, rect: Rect) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.sunken)),
            stroke: Some(Stroke::new(
                self.theme.strokes.hairline,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_virtual_table_header(
        &mut self,
        rect: Rect,
        column: &TableColumn,
        response: stern_core::Response,
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

    fn paint_virtual_table_resize_handle(&mut self, rect: Rect, response: Response) {
        let width = self.theme.strokes.default;
        let line = Rect::new(
            rect.x + (rect.width - width) * 0.5,
            rect.y,
            width,
            rect.height,
        );
        let color = if response.state.pressed || response.dragged {
            self.theme.colors.accent.default
        } else {
            self.theme.colors.border.subtle
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: line,
            fill: Some(Brush::Solid(color)),
            stroke: None,
            radius: self.theme.radii.none,
        }));
    }

    fn paint_virtual_table_cell(
        &mut self,
        rect: Rect,
        label: &str,
        response: Response,
        disabled: bool,
    ) {
        let recipe = self.theme.row(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled,
            selected: response.state.selected,
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        self.paint_virtual_table_text(rect, label, recipe.foreground);
    }

    fn paint_virtual_table_text(&mut self, rect: Rect, label: &str, color: stern_core::Color) {
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

fn virtual_table_movement(
    mode: VirtualTableSelectionMode,
    key: Key,
    page_rows: usize,
) -> Option<VirtualTableCursorMove> {
    match (mode, key) {
        (_, Key::ArrowUp) => Some(VirtualTableCursorMove::PreviousRow),
        (_, Key::ArrowDown) => Some(VirtualTableCursorMove::NextRow),
        (_, Key::PageUp) => Some(VirtualTableCursorMove::PagePrevious { rows: page_rows }),
        (_, Key::PageDown) => Some(VirtualTableCursorMove::PageNext { rows: page_rows }),
        (VirtualTableSelectionMode::Row, Key::Home) => Some(VirtualTableCursorMove::FirstRow),
        (VirtualTableSelectionMode::Row, Key::End) => Some(VirtualTableCursorMove::LastRow),
        (VirtualTableSelectionMode::Cell, Key::ArrowLeft) => {
            Some(VirtualTableCursorMove::PreviousColumn)
        }
        (VirtualTableSelectionMode::Cell, Key::ArrowRight) => {
            Some(VirtualTableCursorMove::NextColumn)
        }
        (VirtualTableSelectionMode::Cell, Key::Home) => Some(VirtualTableCursorMove::FirstColumn),
        (VirtualTableSelectionMode::Cell, Key::End) => Some(VirtualTableCursorMove::LastColumn),
        _ => None,
    }
}

fn virtual_table_page_rows(config: &VirtualTableConfig) -> usize {
    let row_height = config
        .layout
        .effective_row_height()
        .expect("prepared virtual table has a valid row height");
    let body_height = config.bounds.height - config.layout.effective_header_height();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rows = (body_height / row_height).floor() as usize;
    rows.max(1)
}

fn refresh_virtual_table_response(response: &mut Response, selected: bool, focused: bool) {
    response.state.selected = selected;
    response.state.focused = focused;
}

fn virtual_table_body_semantics(
    id: stern_core::WidgetId,
    role: SemanticRole,
    rect: Rect,
    label: &str,
    children: Vec<stern_core::WidgetId>,
    response: Option<Response>,
    disabled: bool,
) -> SemanticNode {
    let selectable = response.is_some();
    let mut node = SemanticNode::new(id, role, rect)
        .with_label(label)
        .with_children(children)
        .focusable(selectable && !disabled);
    node.state.disabled = disabled;
    if let Some(response) = response {
        node.state.selected = response.state.selected;
        node.state.focused = response.state.focused;
        node.state.pressed = response.state.pressed;
    }
    if selectable && !disabled {
        node.actions
            .push(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    }
    node
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
    id: stern_core::WidgetId,
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
