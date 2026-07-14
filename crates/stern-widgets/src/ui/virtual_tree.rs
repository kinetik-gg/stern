use std::hash::Hash;

use stern_core::{
    Brush, ClipId, ComponentState, Key, KeyState, LinePrimitive, Modifiers, Point, Primitive, Rect,
    RectPrimitive, RepaintRequest, Response, SelectionGesturePhase, SemanticAction,
    SemanticActionKind, SemanticNode, SemanticRole, Stroke, TextPrimitive, TextRole, Transform,
    Vec2, scrollable,
};

use super::Ui;
use crate::collections::{
    CollectionCursor, CollectionCursorMove, CollectionCursorTarget, CollectionProjection, ItemId,
    Selection, TreeExpansion, TreeRow, VirtualTree, VirtualTreeConfig, VirtualTreeItemResponse,
    VirtualTreeOutput, VirtualTreeRow, VirtualTreeSelectionMode,
};

impl Ui<'_> {
    /// Prepares one fixed-height virtual-tree frame before pointer arbitration.
    ///
    /// Returns `None` for malformed models, empty/non-finite viewport geometry,
    /// or an invalid row height. The returned snapshot must be shared by
    /// pointer declaration and [`Self::virtual_tree`].
    #[must_use]
    pub fn prepare_virtual_tree<'tree>(
        &self,
        key: impl Hash,
        config: VirtualTreeConfig,
        model: &'tree crate::collections::TreeModel,
        expansion: &TreeExpansion,
    ) -> Option<VirtualTree<'tree>> {
        let root = self.make_id(key);
        let retained_scroll = self.memory().scroll_offset(root).y;
        VirtualTree::prepare(root, config, model, expansion, retained_scroll)
    }

    /// Paints and evaluates a prepared fixed-height virtual tree.
    ///
    /// The row callback is invoked only for the prepared materialized range.
    /// Call [`VirtualTree::declare_pointer_targets`] inside the frame's single
    /// [`Self::resolve_pointer_targets`] pass before evaluating the tree.
    #[allow(clippy::too_many_lines)]
    pub fn virtual_tree(
        &mut self,
        tree: &VirtualTree<'_>,
        cursor: &mut CollectionCursor,
        selection: &mut Selection,
        expansion: &mut TreeExpansion,
        mut row: impl FnMut(TreeRow) -> VirtualTreeRow,
    ) -> VirtualTreeOutput {
        let root = tree.widget_id();
        let config = tree.config();
        self.register_id(root);

        let scroll = {
            let (input, memory) = self.runtime.input_and_memory_mut();
            scrollable(
                root,
                config.bounds,
                tree.content_size(),
                input,
                memory,
                config.disabled,
            )
        };
        if scroll.delta != Vec2::ZERO {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        let mut output = VirtualTreeOutput {
            scroll,
            window: tree.window().clone(),
            activated: None,
            selection_changed: false,
            expansion_changed: false,
            toggled: None,
            cursor_target: None,
            responses: Vec::with_capacity(tree.rows().len()),
        };

        let old_active = cursor.active();
        let old_projected_index = cursor.last_projected_index();
        let old_focused =
            old_active.is_some_and(|id| self.memory().is_focused(tree.row_widget_id(id)));
        output.cursor_target = cursor.reconcile(tree.projection());
        let cursor_reconciled =
            old_active != cursor.active() || old_projected_index != cursor.last_projected_index();
        if old_focused && cursor_reconciled {
            if let Some(target) = output.cursor_target {
                self.focus_and_reveal_virtual_tree_target(tree, target);
            } else {
                self.runtime.memory_mut().clear_focus();
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        let mut keyboard_activated = None;
        if !config.disabled
            && cursor
                .active()
                .is_some_and(|id| self.memory().is_focused(tree.row_widget_id(id)))
        {
            let events = self.input().keyboard.events.clone();
            let page_rows = tree_page_rows(config);
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
                    if let Some(target) = cursor.navigate(tree.projection(), movement) {
                        output.selection_changed |= apply_tree_selection(
                            selection,
                            tree.projection(),
                            target.id,
                            event.modifiers,
                            config.selection_mode,
                        );
                        output.cursor_target = Some(target);
                        final_focus_target = Some(target);
                    }
                    continue;
                }

                if matches!(event.key, Key::ArrowLeft | Key::ArrowRight) {
                    let horizontal = navigate_tree_horizontally(
                        tree,
                        cursor,
                        expansion,
                        event.key == Key::ArrowRight,
                    );
                    if let Some(id) = horizontal.toggled {
                        output.expansion_changed = true;
                        output.toggled = Some(id);
                    }
                    if let Some(target) = horizontal.target {
                        output.selection_changed |= apply_tree_selection(
                            selection,
                            tree.projection(),
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
                self.focus_and_reveal_virtual_tree_target(tree, target);
            }
        }

        if let Some(active) = cursor.active()
            && !tree.contains_materialized(active)
        {
            self.register_id(tree.row_widget_id(active));
        }

        self.paint_virtual_tree_surface(config.bounds);
        let children = tree
            .rows()
            .iter()
            .filter(|row| tree.row_is_visible(row))
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let mut root_semantics = SemanticNode::new(root, SemanticRole::List, config.bounds)
            .with_label(&config.label)
            .with_children(children);
        root_semantics.state.disabled = config.disabled;
        self.push_semantic_node(root_semantics);

        let clip = ClipId::from_raw(root.child("virtual-tree-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: config.bounds,
        });
        self.primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(0.0, -tree.window().clamped_scroll_offset),
        )));

        for projected in tree.rows() {
            self.register_id(projected.id);
            if projected.row.has_children {
                self.register_id(projected.disclosure_id);
            }
            let presentation = row(projected.row);
            let gesture = self.runtime.captured_selection_gesture(
                projected.id,
                projected.rect,
                config.disabled,
            );
            let mut response = gesture.response;
            response.keyboard_activated = keyboard_activated == Some(projected.row.id);
            let disclosure_response = projected.row.has_children.then(|| {
                self.pressable_with_id(
                    projected.disclosure_id,
                    projected.disclosure_rect,
                    config.disabled,
                )
            });

            if disclosure_response.is_some_and(|response| response.clicked) {
                expansion.toggle(projected.row.id);
                output.expansion_changed = true;
                output.toggled = Some(projected.row.id);
            }

            if response.clicked {
                let modifiers = gesture
                    .actions
                    .iter()
                    .rev()
                    .find(|action| action.phase == SelectionGesturePhase::Release)
                    .map_or(self.input().keyboard.modifiers, |action| action.modifiers);
                if let Some(target) = cursor.activate(tree.projection(), projected.row.id) {
                    output.cursor_target = Some(target);
                    output.selection_changed |= apply_tree_selection(
                        selection,
                        tree.projection(),
                        target.id,
                        modifiers,
                        config.selection_mode,
                    );
                    self.focus_and_reveal_virtual_tree_target(tree, target);
                }
            }
            if response.double_clicked {
                output.activated.get_or_insert(projected.row.id);
            }

            response.state.selected = selection.contains(projected.row.id);
            response.state.focused = self.memory().is_focused(projected.id);
            if response.clicked
                || response.double_clicked
                || response.keyboard_activated
                || response.state.pressed
                || disclosure_response
                    .is_some_and(|response| response.clicked || response.state.pressed)
            {
                self.request_repaint(RepaintRequest::NextFrame);
            }
            self.paint_virtual_tree_row(
                projected.rect,
                projected.disclosure_rect,
                projected.row,
                &presentation.label,
                response,
                disclosure_response,
                config.disabled,
            );
            if tree.row_is_visible(projected) {
                self.push_semantic_node(virtual_tree_row_semantics(
                    projected.id,
                    projected.rect,
                    projected.row,
                    &presentation.label,
                    response,
                    config.disabled,
                ));
            }
            output.responses.push(VirtualTreeItemResponse {
                row: projected.row,
                response,
                disclosure_response,
            });
        }

        self.primitive(Primitive::TransformEnd);
        self.primitive(Primitive::ClipEnd { id: clip });

        if output.selection_changed || output.expansion_changed || output.activated.is_some() {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn focus_and_reveal_virtual_tree_target(
        &mut self,
        tree: &VirtualTree<'_>,
        target: CollectionCursorTarget,
    ) {
        let row_id = tree.row_widget_id(target.id);
        if !tree.contains_materialized(target.id) {
            self.register_id(row_id);
        }
        let reveal = revealed_tree_scroll_offset(tree, target);
        let focus_changed = !self.memory().is_focused(row_id);
        let reveal_changed = reveal.to_bits() != tree.window().clamped_scroll_offset.to_bits();
        let memory = self.runtime.memory_mut();
        if focus_changed {
            memory.focus(row_id);
        }
        if reveal_changed {
            memory.stage_scroll_offset(tree.widget_id(), Vec2::new(0.0, reveal));
        }
        if focus_changed || reveal_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
    }

    fn paint_virtual_tree_surface(&mut self, rect: Rect) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.sunken)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_virtual_tree_row(
        &mut self,
        rect: Rect,
        disclosure_rect: Rect,
        row: TreeRow,
        label: &str,
        response: Response,
        disclosure_response: Option<Response>,
        disabled: bool,
    ) {
        let recipe = self.theme.row(ComponentState {
            hovered: response.state.hovered
                || disclosure_response.is_some_and(|response| response.state.hovered),
            pressed: response.state.pressed
                || disclosure_response.is_some_and(|response| response.state.pressed),
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

        if row.has_children {
            self.paint_virtual_tree_disclosure(disclosure_rect, row.expanded, recipe.foreground);
        }

        let font = self.theme.font(TextRole::Label);
        let extra = (rect.height - font.line_height).max(0.0) * 0.5;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                disclosure_rect.max_x() + self.theme.controls.padding_x,
                rect.y + extra + font.size,
            ),
            text: label.to_owned(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));
    }

    fn paint_virtual_tree_disclosure(
        &mut self,
        rect: Rect,
        expanded: bool,
        color: stern_core::Color,
    ) {
        let center = rect.center();
        let half = rect.width.min(rect.height) * 0.16;
        let stroke = Stroke::new(
            self.theme.controls.border_width.max(1.0),
            Brush::Solid(color),
        );
        let (first, middle, last) = if expanded {
            (
                Point::new(center.x - half, center.y - half * 0.5),
                Point::new(center.x, center.y + half * 0.5),
                Point::new(center.x + half, center.y - half * 0.5),
            )
        } else {
            (
                Point::new(center.x - half * 0.5, center.y - half),
                Point::new(center.x + half * 0.5, center.y),
                Point::new(center.x - half * 0.5, center.y + half),
            )
        };
        self.primitive(Primitive::Line(LinePrimitive {
            from: first,
            to: middle,
            stroke,
        }));
        self.primitive(Primitive::Line(LinePrimitive {
            from: middle,
            to: last,
            stroke,
        }));
    }
}

fn apply_tree_selection(
    selection: &mut Selection,
    projection: &CollectionProjection,
    id: ItemId,
    modifiers: Modifiers,
    mode: VirtualTreeSelectionMode,
) -> bool {
    let before = selection.clone();
    match mode {
        VirtualTreeSelectionMode::Multiple if modifiers.shift => {
            let visible = projection.visible_ids();
            if !selection.select_range(&visible, id) {
                selection.replace(id);
            }
        }
        VirtualTreeSelectionMode::Multiple if modifiers.ctrl || modifiers.super_key => {
            selection.toggle(id);
        }
        VirtualTreeSelectionMode::Single | VirtualTreeSelectionMode::Multiple => {
            selection.replace(id);
        }
    }
    *selection != before
}

fn tree_page_rows(config: &VirtualTreeConfig) -> usize {
    let row_height = config
        .layout
        .effective_row_height()
        .expect("prepared virtual tree has a valid row height");
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let rows = (config.bounds.height / row_height).floor() as usize;
    rows.max(1)
}

#[allow(clippy::cast_precision_loss)]
fn revealed_tree_scroll_offset(tree: &VirtualTree<'_>, target: CollectionCursorTarget) -> f32 {
    let config = tree.config();
    let current = tree.window().clamped_scroll_offset;
    let row_height = config
        .layout
        .effective_row_height()
        .expect("prepared virtual tree has a valid row height");
    let top = target.projected_index as f32 * row_height;
    let bottom = top + row_height;
    let desired = if top < current {
        top
    } else if bottom > current + config.bounds.height {
        bottom - config.bounds.height
    } else {
        current
    };
    config
        .layout
        .clamp_scroll_offset(tree.projection().len(), config.bounds.height, desired)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HorizontalTreeNavigation {
    target: Option<CollectionCursorTarget>,
    toggled: Option<ItemId>,
}

fn navigate_tree_horizontally(
    tree: &VirtualTree<'_>,
    cursor: &mut CollectionCursor,
    expansion: &mut TreeExpansion,
    right: bool,
) -> HorizontalTreeNavigation {
    let Some(active) = cursor.active() else {
        return HorizontalTreeNavigation::default();
    };
    let Some(item) = tree.model().items().iter().find(|item| item.id == active) else {
        return HorizontalTreeNavigation::default();
    };
    let children = tree.model().child_ids(Some(active));
    let has_children = item.has_children || !children.is_empty();

    if right {
        if has_children && !expansion.is_expanded(active) {
            expansion.expand(active);
            return HorizontalTreeNavigation {
                target: None,
                toggled: Some(active),
            };
        }
        let target = children
            .first()
            .and_then(|child| cursor.activate(tree.projection(), *child));
        return HorizontalTreeNavigation {
            target,
            toggled: None,
        };
    }

    if has_children && expansion.collapse(active) {
        return HorizontalTreeNavigation {
            target: None,
            toggled: Some(active),
        };
    }
    let target = item
        .parent
        .and_then(|parent| cursor.activate(tree.projection(), parent));
    HorizontalTreeNavigation {
        target,
        toggled: None,
    }
}

fn virtual_tree_row_semantics(
    id: stern_core::WidgetId,
    rect: Rect,
    row: TreeRow,
    label: &str,
    response: Response,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::ListItem, rect)
        .with_label(label)
        .focusable(!disabled);
    node.state.disabled = disabled;
    node.state.selected = response.state.selected;
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    node.state.expanded = row.has_children.then_some(row.expanded);
    if !disabled {
        node.actions
            .push(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
        if row.has_children {
            node.actions.push(SemanticAction::new(
                if row.expanded {
                    SemanticActionKind::Close
                } else {
                    SemanticActionKind::Open
                },
                if row.expanded { "Collapse" } else { "Expand" },
            ));
        }
    }
    node
}
