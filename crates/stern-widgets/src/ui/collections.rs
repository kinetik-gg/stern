use std::hash::Hash;

use stern_core::{
    Brush, ClipId, ComponentState, Key, KeyState, Modifiers, Point, Primitive, Rect, RectPrimitive,
    RepaintRequest, SelectionGesturePhase, SemanticAction, SemanticActionKind, SemanticNode,
    SemanticRole, Stroke, TextPrimitive, TextRole, Transform, Vec2, scrollable,
};

use super::Ui;
use crate::collections::{
    CollectionCursor, CollectionCursorMove, CollectionCursorTarget, CollectionProjectedItem,
    CollectionProjection, Selection, VirtualList, VirtualListConfig, VirtualListItemResponse,
    VirtualListOutput, VirtualListRow, VirtualListSelectionMode,
};

impl Ui<'_> {
    /// Prepares one fixed-height virtual-list frame before pointer arbitration.
    ///
    /// Returns `None` for empty/non-finite viewport geometry or an invalid row
    /// height. The returned snapshot must be shared by pointer declaration and
    /// [`Self::virtual_list`] so hit, paint, and semantic geometry stay frozen.
    #[must_use]
    pub fn prepare_virtual_list<'list>(
        &self,
        key: impl Hash,
        config: VirtualListConfig,
        projection: &'list CollectionProjection,
    ) -> Option<VirtualList<'list>> {
        let root = self.make_id(key);
        let retained_scroll = self.memory().scroll_offset(root).y;
        VirtualList::prepare(root, config, projection, retained_scroll)
    }

    /// Paints and evaluates a prepared fixed-height virtual list.
    ///
    /// The row callback is invoked only for the prepared materialized range.
    /// Call [`VirtualList::declare_pointer_targets`] inside the frame's single
    /// [`Self::resolve_pointer_targets`] pass before evaluating the list.
    #[allow(clippy::too_many_lines)]
    pub fn virtual_list(
        &mut self,
        list: &VirtualList<'_>,
        cursor: &mut CollectionCursor,
        selection: &mut Selection,
        mut row: impl FnMut(CollectionProjectedItem) -> VirtualListRow,
    ) -> VirtualListOutput {
        let root = list.widget_id();
        let config = list.config();
        self.register_id(root);

        let scroll = {
            let (input, memory) = self.runtime.input_and_memory_mut();
            scrollable(
                root,
                config.bounds,
                list.content_size(),
                input,
                memory,
                config.disabled,
            )
        };
        if scroll.delta != Vec2::ZERO {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        let mut output = VirtualListOutput {
            scroll,
            window: list.window().clone(),
            activated: None,
            selection_changed: false,
            cursor_target: None,
            responses: Vec::with_capacity(list.rows().len()),
        };

        let old_active = cursor.active();
        let old_projected_index = cursor.last_projected_index();
        let old_focused =
            old_active.is_some_and(|id| self.memory().is_focused(list.row_widget_id(id)));
        output.cursor_target = cursor.reconcile(list.projection());
        let cursor_reconciled =
            old_active != cursor.active() || old_projected_index != cursor.last_projected_index();
        if old_focused && cursor_reconciled {
            if let Some(target) = output.cursor_target {
                self.focus_and_reveal_virtual_list_target(list, target);
            } else {
                self.runtime.memory_mut().clear_focus();
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        let mut keyboard_activated = None;
        if !config.disabled
            && cursor
                .active()
                .is_some_and(|id| self.memory().is_focused(list.row_widget_id(id)))
        {
            let events = self.input().keyboard.events.clone();
            let page_rows = page_rows(config);
            let mut final_focus_target = None;
            for event in events {
                if event.state != KeyState::Pressed || event.modifiers.alt {
                    continue;
                }
                let movement = match event.key {
                    Key::ArrowUp => Some(CollectionCursorMove::Previous),
                    Key::ArrowDown => Some(CollectionCursorMove::Next),
                    Key::Home => Some(CollectionCursorMove::First),
                    Key::End => Some(CollectionCursorMove::Last),
                    Key::PageUp => Some(CollectionCursorMove::PagePrevious { rows: page_rows }),
                    Key::PageDown => Some(CollectionCursorMove::PageNext { rows: page_rows }),
                    _ => None,
                };
                if let Some(movement) = movement {
                    if let Some(target) = cursor.navigate(list.projection(), movement) {
                        output.selection_changed |= apply_list_selection(
                            selection,
                            list.projection(),
                            target.id,
                            event.modifiers,
                            config.selection_mode,
                        );
                        output.cursor_target = Some(target);
                        final_focus_target = Some(target);
                    }
                    continue;
                }

                if !event.repeat
                    && event.modifiers.is_empty()
                    && matches!(event.key, Key::Enter | Key::Space)
                    && let Some(id) = cursor.active()
                {
                    keyboard_activated.get_or_insert(id);
                    output.activated.get_or_insert(id);
                }
            }
            if let Some(target) = final_focus_target {
                self.focus_and_reveal_virtual_list_target(list, target);
            }
        }

        if let Some(active) = cursor.active()
            && !list.contains_materialized(active)
        {
            self.register_id(list.row_widget_id(active));
        }

        self.paint_virtual_list_surface(config.bounds);
        let children = list
            .rows()
            .iter()
            .filter(|row| list.row_is_visible(row))
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let mut root_semantics = SemanticNode::new(root, SemanticRole::List, config.bounds)
            .with_label(&config.label)
            .with_children(children);
        root_semantics.state.disabled = config.disabled;
        self.push_semantic_node(root_semantics);

        let clip = ClipId::from_raw(root.child("virtual-list-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: config.bounds,
        });
        self.primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(0.0, -list.window().clamped_scroll_offset),
        )));

        for projected in list.rows() {
            self.register_id(projected.id);
            let presentation = row(projected.item);
            let gesture = self.runtime.captured_selection_gesture(
                projected.id,
                projected.rect,
                config.disabled,
            );
            let mut response = gesture.response;
            response.keyboard_activated = keyboard_activated == Some(projected.item.id);

            if response.clicked {
                let modifiers = gesture
                    .actions
                    .iter()
                    .rev()
                    .find(|action| action.phase == SelectionGesturePhase::Release)
                    .map_or(self.input().keyboard.modifiers, |action| action.modifiers);
                if let Some(target) = cursor.activate(list.projection(), projected.item.id) {
                    output.cursor_target = Some(target);
                    output.selection_changed |= apply_list_selection(
                        selection,
                        list.projection(),
                        target.id,
                        modifiers,
                        config.selection_mode,
                    );
                    self.focus_and_reveal_virtual_list_target(list, target);
                }
            }
            if response.double_clicked {
                output.activated.get_or_insert(projected.item.id);
            }

            response.state.selected = selection.contains(projected.item.id);
            response.state.focused = self.memory().is_focused(projected.id);
            if response.clicked
                || response.double_clicked
                || response.keyboard_activated
                || response.state.pressed
            {
                self.request_repaint(RepaintRequest::NextFrame);
            }
            self.paint_virtual_list_row(
                projected.rect,
                &presentation.label,
                response,
                config.disabled,
            );
            if list.row_is_visible(projected) {
                self.push_semantic_node(virtual_list_row_semantics(
                    projected.id,
                    projected.rect,
                    &presentation.label,
                    response,
                    config.disabled,
                ));
            }
            output.responses.push(VirtualListItemResponse {
                id: projected.item.id,
                response,
            });
        }

        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: clip });

        if output.selection_changed || output.activated.is_some() {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn focus_and_reveal_virtual_list_target(
        &mut self,
        list: &VirtualList<'_>,
        target: CollectionCursorTarget,
    ) {
        let row_id = list.row_widget_id(target.id);
        if !list.contains_materialized(target.id) {
            self.register_id(row_id);
        }
        let reveal = revealed_scroll_offset(list, target);
        let focus_changed = !self.memory().is_focused(row_id);
        let reveal_changed = reveal.to_bits() != list.window().clamped_scroll_offset.to_bits();
        let memory = self.runtime.memory_mut();
        if focus_changed {
            memory.focus(row_id);
        }
        if reveal_changed {
            memory.stage_scroll_offset(list.widget_id(), Vec2::new(0.0, reveal));
        }
        if focus_changed || reveal_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
    }

    fn paint_virtual_list_surface(&mut self, rect: Rect) {
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

    fn paint_virtual_list_row(
        &mut self,
        rect: Rect,
        label: &str,
        response: stern_core::Response,
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
            brush: Brush::Solid(recipe.foreground),
        }));
    }
}

fn apply_list_selection(
    selection: &mut Selection,
    projection: &CollectionProjection,
    id: crate::collections::ItemId,
    modifiers: Modifiers,
    mode: VirtualListSelectionMode,
) -> bool {
    let before = selection.clone();
    match mode {
        VirtualListSelectionMode::Multiple if modifiers.shift => {
            let visible = projection.visible_ids();
            if !selection.select_range(&visible, id) {
                selection.replace(id);
            }
        }
        VirtualListSelectionMode::Multiple if modifiers.ctrl || modifiers.super_key => {
            selection.toggle(id);
        }
        VirtualListSelectionMode::Single | VirtualListSelectionMode::Multiple => {
            selection.replace(id);
        }
    }
    *selection != before
}

fn page_rows(config: &VirtualListConfig) -> usize {
    let row_height = config
        .layout
        .effective_row_height()
        .expect("prepared virtual list has a valid row height");
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rows = (config.bounds.height / row_height).floor() as usize;
    rows.max(1)
}

fn revealed_scroll_offset(list: &VirtualList<'_>, target: CollectionCursorTarget) -> f32 {
    let config = list.config();
    let current = list.window().clamped_scroll_offset;
    let Some(rect) = config.layout.row_rect(
        Rect::new(config.bounds.x, 0.0, config.bounds.width, 0.0),
        target.projected_index,
    ) else {
        return current;
    };
    let desired = if rect.y < current {
        rect.y
    } else if rect.max_y() > current + config.bounds.height {
        rect.max_y() - config.bounds.height
    } else {
        current
    };
    config
        .layout
        .clamp_scroll_offset(list.projection().len(), config.bounds.height, desired)
}

fn virtual_list_row_semantics(
    id: stern_core::WidgetId,
    rect: Rect,
    label: &str,
    response: stern_core::Response,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::ListItem, rect)
        .with_label(label)
        .focusable(!disabled);
    node.state.disabled = disabled;
    node.state.selected = response.state.selected;
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    if !disabled {
        node.actions
            .push(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    }
    node
}
