use std::collections::BTreeSet;
use std::hash::Hash;

use stern_core::{
    ActionContext, ActionDescriptor, ActionSource, Brush, ClipId, ComponentState,
    DomainDragGesturePhase, InteractionState, Key, KeyState, LinePrimitive, Modifiers, Point,
    Primitive, Rect, RectPrimitive, RepaintRequest, Response, Stroke, TextPrimitive, TextRole,
    Vec2, context_menu_trigger, drop_target, scrollable,
};

use super::Ui;
use crate::outliner::{
    OutlinerConfig, OutlinerContextState, OutlinerDragState, OutlinerDropTarget,
    OutlinerDropZoneKind, OutlinerModel, OutlinerOutput, OutlinerRequest, OutlinerRowResponse,
    OutlinerRowZones, OutlinerScene, OutlinerSelectionMode, OutlinerState, background_widget_id,
    context_overlay_id, disclosure_widget_id, drop_widget_id, lock_widget_id, outliner_semantics,
    visibility_widget_id,
};
use crate::{
    CollectionContextActionRequest, CollectionContextTarget, CollectionCursorMove,
    CollectionCursorTarget, InlineEditCancelReason, InlineEditCommitReason,
    InlineEditFocusLossPolicy, InlineEditRequest, ItemId, Menu, MenuOverlay, OverlayDismissal,
    OverlayKind, OverlayScene, OverlaySceneIntent, OverlaySceneSurface, PopoverPlacement,
    Selection, TextFieldAccess, collection_context_actions,
};

impl Ui<'_> {
    /// Prepares one fixed-height outliner frame before pointer arbitration.
    ///
    /// Returns `None` for malformed models, empty or non-finite viewport
    /// geometry, or an invalid row height. The returned snapshot must be
    /// shared by pointer declaration and [`Self::outliner`].
    #[must_use]
    pub fn prepare_outliner<'model>(
        &self,
        key: impl Hash,
        config: OutlinerConfig,
        model: &'model OutlinerModel,
        state: &OutlinerState,
    ) -> Option<OutlinerScene<'model>> {
        let root = self.make_id(key);
        let retained_scroll = self.memory().scroll_offset(root).y;
        OutlinerScene::prepare(root, config, model, state, retained_scroll)
    }

    /// Evaluates and paints one prepared reusable outliner.
    ///
    /// Selection and expansion remain in [`OutlinerState`]. Rename, visibility,
    /// lock, hierarchy-drop, and context-action mutations are emitted as typed
    /// application requests. Context descriptors are resolved only when a real
    /// menu is requested.
    #[allow(clippy::too_many_lines)]
    pub fn outliner(
        &mut self,
        scene: &OutlinerScene<'_>,
        state: &mut OutlinerState,
        mut context_actions: impl FnMut(&CollectionContextTarget) -> Vec<ActionDescriptor>,
    ) -> OutlinerOutput {
        let root = scene.widget_id();
        let config = scene.config();
        self.register_id(root);

        let scroll = {
            let (input, memory) = self.runtime.input_and_memory_mut();
            scrollable(
                root,
                config.bounds,
                scene.content_size(),
                input,
                memory,
                config.disabled,
            )
        };
        if scroll.delta != Vec2::ZERO {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        let mut output = OutlinerOutput {
            scroll,
            window: scene.window().clone(),
            activated: None,
            selection_changed: false,
            expansion_changed: false,
            context_opened: None,
            drop_preview: None,
            requests: Vec::new(),
            responses: Vec::with_capacity(scene.rows().len()),
        };

        self.reconcile_outliner_retained_state(scene, state, &mut output);
        let prepared_rename = state.rename_target();
        let context_was_prepared = scene.has_prepared_context() && state.context.is_some();

        self.reconcile_outliner_cursor(scene, state);
        let mut keyboard_activated = None;
        self.handle_outliner_keyboard(scene, state, &mut output, &mut keyboard_activated);

        self.paint_outliner_surface(config.bounds);
        let strict_rows = scene.strict_rows().cloned().collect::<Vec<_>>();
        let strict_ids = strict_rows
            .iter()
            .map(|row| row.row.id)
            .collect::<BTreeSet<_>>();
        let mut root_semantics = outliner_semantics(
            root,
            config.bounds,
            &strict_rows,
            &state.selection,
            &config.label,
        )
        .remove(0);
        root_semantics.state.disabled = config.disabled;
        if let Some(target) = prepared_rename
            && strict_ids.contains(&target)
            && let Some(child) = root_semantics
                .children
                .iter_mut()
                .find(|child| **child == scene.row_widget_id(target))
        {
            *child = scene.rename_widget_id(target);
        }
        self.push_semantic_node(root_semantics);

        let clip = ClipId::from_raw(root.child("outliner-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: config.bounds,
        });

        let mut drag_position = self.input().pointer.position;
        let mut drag_released = false;
        let mut drag_cancelled = false;
        let mut pending_context = None;
        let mut rename_terminal = None;
        let mut rename_evaluated = false;

        for zones in scene.rows() {
            let item = zones.row.id;
            let row_id = scene.row_widget_id(item);
            let editing = prepared_rename == Some(item) && state.rename_target() == Some(item);
            self.register_id(row_id);

            let mut row_response = if editing {
                Response::new(row_id, zones.rect, InteractionState::default())
            } else {
                let gesture = self.runtime.captured_domain_drag_gesture(
                    row_id,
                    zones.rect,
                    config.disabled || zones.row.flags.disabled,
                );
                for action in &gesture.actions {
                    match action.phase {
                        DomainDragGesturePhase::Move => {
                            drag_position = action.position.or(drag_position);
                        }
                        DomainDragGesturePhase::Release => {
                            drag_position = action.position.or(drag_position);
                            drag_released = true;
                        }
                        DomainDragGesturePhase::Cancel => drag_cancelled = true,
                        DomainDragGesturePhase::Press => {}
                    }
                }

                let source_visible = self.memory().drag_source() == Some(row_id)
                    || self.memory().released_drag_source() == Some(row_id)
                    || gesture.response.dragged;
                if source_visible
                    && state.drag.is_none()
                    && let Some(source) = zones.row.drag_source(&state.selection)
                {
                    state.drag = Some(OutlinerDragState {
                        widget: row_id,
                        source,
                    });
                }

                let mut response = gesture.response;
                response.keyboard_activated = keyboard_activated == Some(item);
                if response.clicked && zones.row.flags.can_request_selection() {
                    let modifiers = gesture
                        .actions
                        .iter()
                        .rev()
                        .find(|action| {
                            action.phase == DomainDragGesturePhase::Release
                                && action.release_clicked
                        })
                        .map_or(self.input().keyboard.modifiers, |action| action.modifiers);
                    if let Some(target) = state.cursor.activate(scene.projection(), item) {
                        output.selection_changed |= apply_outliner_selection(
                            &mut state.selection,
                            scene,
                            target.id,
                            modifiers,
                            config.selection_mode,
                        );
                        self.focus_and_reveal_outliner_target(scene, target);
                    }
                }

                if response.double_clicked {
                    if let Some(begin) = zones.row.inline_rename_begin_request(root) {
                        let text_widget = begin.text_widget_id;
                        self.register_id(text_widget);
                        output
                            .requests
                            .push(OutlinerRequest::Rename(state.begin_rename(begin, config)));
                        self.runtime.memory_mut().focus(text_widget);
                        self.request_repaint(RepaintRequest::NextFrame);
                    } else {
                        output.activated.get_or_insert(item);
                    }
                }

                let pointer_context_requested = response.secondary_clicked;
                let context_response = {
                    let (input, memory) = self.runtime.input_and_memory_mut();
                    context_menu_trigger(
                        row_id,
                        zones.context_rect,
                        input,
                        memory,
                        config.disabled || zones.row.flags.disabled,
                    )
                };
                if (pointer_context_requested || context_response.context_requested)
                    && let Some(target) = zones.row.context_target(&state.selection)
                {
                    pending_context.get_or_insert_with(|| {
                        (
                            target,
                            row_id,
                            outliner_context_anchor(self.input().pointer.position, zones.rect),
                        )
                    });
                }
                response
            };

            row_response.state.disabled = config.disabled || zones.row.flags.disabled;
            row_response.state.selected = state.selection.contains(item);
            row_response.state.focused = self.memory().is_focused(row_id);

            let disclosure = zones.row.has_children.then(|| {
                let id = self.register_id(disclosure_widget_id(row_id));
                self.pressable_with_id(
                    id,
                    zones.disclosure_rect,
                    config.disabled || zones.row.flags.disabled,
                )
            });
            if disclosure.is_some_and(|response| response.clicked) {
                state.expansion.toggle(item);
                output.expansion_changed = true;
            }

            let visibility = zones.row.flags.can_request_visibility_toggle().then(|| {
                let id = self.register_id(visibility_widget_id(row_id));
                self.pressable_with_id(id, zones.visibility_toggle_rect, config.disabled)
            });
            if visibility.is_some_and(|response| response.clicked)
                && let Some(request) = zones.row.visibility_toggle_request()
            {
                output.requests.push(OutlinerRequest::Visibility(request));
            }

            let lock = zones.row.flags.can_request_lock_toggle().then(|| {
                let id = self.register_id(lock_widget_id(row_id));
                self.pressable_with_id(id, zones.lock_toggle_rect, config.disabled)
            });
            if lock.is_some_and(|response| response.clicked)
                && let Some(request) = zones.row.lock_toggle_request()
            {
                output.requests.push(OutlinerRequest::Lock(request));
            }

            self.paint_outliner_row(zones, row_response, disclosure, visibility, lock, !editing);

            if editing {
                rename_evaluated = true;
                let rename_id = self.register_id(scene.rename_widget_id(item));
                if let Some(edit) = state.edit.as_mut() {
                    let (field, ordered) = self.text_field_with_access_id(
                        rename_id,
                        zones.label_rect,
                        &mut edit.text,
                        TextFieldAccess::Editable,
                    );
                    if field.changed {
                        let draft = edit.session.set_draft(edit.text.text.clone());
                        output
                            .requests
                            .push(OutlinerRequest::Rename(InlineEditRequest::DraftEdit(draft)));
                    }
                    if ordered.commit_requested {
                        rename_terminal = edit
                            .session
                            .resolve_commit(InlineEditCommitReason::Enter)
                            .request;
                    } else if ordered.revert_requested {
                        rename_terminal = Some(InlineEditRequest::Cancel(
                            edit.session.cancel_request(InlineEditCancelReason::Escape),
                        ));
                    }
                }
            }

            if row_response.clicked
                || row_response.double_clicked
                || row_response.keyboard_activated
                || row_response.state.pressed
                || disclosure.is_some_and(|response| response.clicked || response.state.pressed)
                || visibility.is_some_and(|response| response.clicked || response.state.pressed)
                || lock.is_some_and(|response| response.clicked || response.state.pressed)
            {
                self.request_repaint(RepaintRequest::NextFrame);
            }
            output.responses.push(OutlinerRowResponse {
                item,
                row: row_response,
                disclosure,
                visibility,
                lock,
            });
        }

        let final_semantics = outliner_semantics(
            root,
            config.bounds,
            &strict_rows,
            &state.selection,
            &config.label,
        );
        for (zones, mut semantic) in strict_rows.iter().zip(final_semantics.into_iter().skip(1)) {
            if prepared_rename == Some(zones.row.id) {
                continue;
            }
            if let Some(response) = output
                .responses
                .iter()
                .find(|response| response.item == zones.row.id)
            {
                semantic.state.focused = response.row.state.focused;
                semantic.state.pressed = response.row.state.pressed;
            }
            semantic.state.expanded = zones
                .row
                .has_children
                .then_some(state.expansion.is_expanded(zones.row.id));
            self.push_semantic_node(semantic);
        }

        let background_id = self.register_id(background_widget_id(root));
        let background_context = {
            let (input, memory) = self.runtime.input_and_memory_mut();
            context_menu_trigger(background_id, config.bounds, input, memory, config.disabled)
        };
        if background_context.context_requested {
            pending_context.get_or_insert_with(|| {
                (
                    CollectionContextTarget::background(),
                    background_id,
                    outliner_context_anchor(self.input().pointer.position, config.bounds),
                )
            });
        }

        if let Some((target, trigger, anchor)) = pending_context {
            let descriptors = collection_context_actions(&target, context_actions(&target))
                .into_iter()
                .map(|action| action.descriptor)
                .collect::<Vec<_>>();
            if !descriptors.is_empty() {
                let viewport = outliner_context_viewport(self, config.bounds);
                let menu = MenuOverlay::anchored(
                    context_overlay_id(root),
                    OverlayKind::ContextMenu,
                    Menu::from_actions(descriptors),
                    anchor,
                    config.context_menu.size,
                    PopoverPlacement::Below,
                    config.context_menu.offset,
                    true,
                    viewport,
                    OverlayDismissal::OutsideClickOrEscape,
                    ActionSource::Menu,
                    ActionContext::Widget(root),
                );
                let mut overlay = OverlayScene::new();
                overlay.push(OverlaySceneSurface::menu("Outliner actions", menu));
                self.runtime.memory_mut().focus(trigger);
                state.context = Some(OutlinerContextState {
                    target: target.clone(),
                    trigger,
                    scene: overlay,
                });
                output.context_opened = Some(target);
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        if rename_terminal.is_none()
            && rename_evaluated
            && let Some(edit) = state.edit.as_ref()
            && !self.memory().is_focused(edit.session.text_widget_id)
        {
            rename_terminal = edit.session.focus_loss_request();
        }
        if let Some(request) = rename_terminal
            && let Some(target) = state.edit.as_ref().map(|edit| edit.session.target)
        {
            output.requests.push(OutlinerRequest::Rename(request));
            state.clear_rename();
            self.runtime.memory_mut().focus(scene.row_widget_id(target));
            self.request_repaint(RepaintRequest::NextFrame);
        }

        self.evaluate_outliner_drop(
            scene,
            state,
            drag_position,
            drag_released,
            drag_cancelled,
            &mut output,
        );
        if let Some(preview) = output.drop_preview.as_ref() {
            self.paint_outliner_drop_preview(scene, preview);
        }
        self.primitive(Primitive::ClipEnd { id: clip });

        if context_was_prepared
            && output.context_opened.is_none()
            && let Some(context) = state.context.as_mut()
        {
            let overlay_output = self.overlay_scene(&mut context.scene);
            let target = context.target.clone();
            let trigger = context.trigger;
            let mut close = false;
            for intent in overlay_output.intents {
                match intent {
                    OverlaySceneIntent::Action(invocation) => {
                        output.requests.push(OutlinerRequest::Context(
                            CollectionContextActionRequest::new(invocation.action_id, &target),
                        ));
                        close = true;
                    }
                    OverlaySceneIntent::Dismiss(_) => close = true,
                    OverlaySceneIntent::OpenSubmenu(_) | OverlaySceneIntent::SelectDropdown(_) => {}
                }
            }
            if close {
                state.context = None;
                self.runtime.memory_mut().focus(trigger);
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        if output.selection_changed
            || output.expansion_changed
            || output.activated.is_some()
            || !output.requests.is_empty()
        {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn reconcile_outliner_retained_state(
        &mut self,
        scene: &OutlinerScene<'_>,
        state: &mut OutlinerState,
        output: &mut OutlinerOutput,
    ) {
        let config = scene.config();
        if config.disabled && state.context.take().is_some() {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        if let Some(drag) = state.drag.as_ref() {
            let owner_retained = self.memory().drag_source() == Some(drag.widget)
                || self.memory().released_drag_source() == Some(drag.widget);
            let source_visible = scene
                .strict_rows()
                .any(|row| row.row.id == drag.source.source);
            if config.disabled || !owner_retained || !source_visible {
                if owner_retained {
                    self.runtime.memory_mut().clear_drag();
                }
                state.drag = None;
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }

        let Some(target) = state.rename_target() else {
            return;
        };
        let eligible = !config.disabled
            && scene
                .model()
                .item_by_id(target)
                .is_some_and(|item| item.flags.can_request_rename());
        let projected_index = scene.projection().projected_index(target);
        if !eligible || projected_index.is_none() {
            let text_widget = state.edit.as_ref().map(|edit| edit.session.text_widget_id);
            if let Some(edit) = state.edit.as_ref() {
                output
                    .requests
                    .push(OutlinerRequest::Rename(InlineEditRequest::Cancel(
                        edit.session
                            .cancel_request(InlineEditCancelReason::Explicit),
                    )));
            }
            state.clear_rename();
            if text_widget.is_some_and(|text_widget| self.memory().is_focused(text_widget)) {
                self.runtime.memory_mut().clear_focus();
            }
            self.request_repaint(RepaintRequest::NextFrame);
            return;
        }

        if scene.strict_rows().any(|row| row.row.id == target) {
            return;
        }
        let projected_index = projected_index.unwrap_or_default();
        let cursor_target = CollectionCursorTarget {
            id: target,
            projected_index,
        };
        let focus_loss_policy = state
            .edit
            .as_ref()
            .map(|edit| edit.session.focus_loss_policy);
        if focus_loss_policy == Some(InlineEditFocusLossPolicy::KeepEditing) {
            self.retain_and_reveal_outliner_rename(scene, cursor_target);
            return;
        }

        let request = state
            .edit
            .as_ref()
            .and_then(|edit| edit.session.focus_loss_request());
        if let Some(request) = request {
            output.requests.push(OutlinerRequest::Rename(request));
            state.clear_rename();
            self.focus_and_reveal_outliner_target(scene, cursor_target);
        } else {
            self.retain_and_reveal_outliner_rename(scene, cursor_target);
        }
    }

    fn retain_and_reveal_outliner_rename(
        &mut self,
        scene: &OutlinerScene<'_>,
        target: CollectionCursorTarget,
    ) {
        self.register_id(scene.rename_widget_id(target.id));
        let reveal = scene.reveal_scroll_offset(target);
        self.runtime
            .memory_mut()
            .stage_scroll_offset(scene.widget_id(), Vec2::new(0.0, reveal));
        self.request_repaint(RepaintRequest::NextFrame);
    }

    fn reconcile_outliner_cursor(&mut self, scene: &OutlinerScene<'_>, state: &mut OutlinerState) {
        let old_active = state.cursor.active();
        let old_index = state.cursor.last_projected_index();
        let old_focused =
            old_active.is_some_and(|item| self.memory().is_focused(scene.row_widget_id(item)));
        let repaired = state
            .cursor
            .reconcile(scene.projection())
            .and_then(|target| repair_outliner_cursor_target(scene, &mut state.cursor, target));
        let changed =
            old_active != state.cursor.active() || old_index != state.cursor.last_projected_index();
        if old_focused && changed {
            if let Some(target) = repaired {
                self.focus_and_reveal_outliner_target(scene, target);
            } else {
                self.runtime.memory_mut().clear_focus();
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    fn handle_outliner_keyboard(
        &mut self,
        scene: &OutlinerScene<'_>,
        state: &mut OutlinerState,
        output: &mut OutlinerOutput,
        keyboard_activated: &mut Option<ItemId>,
    ) {
        let config = scene.config();
        if config.disabled || state.edit.is_some() || state.context.is_some() {
            return;
        }
        let Some(active) = state.cursor.active() else {
            return;
        };
        if !self.memory().is_focused(scene.row_widget_id(active)) {
            return;
        }

        let events = self.input().keyboard.events.clone();
        let mut final_focus = None;
        for event in events {
            if event.state != KeyState::Pressed || event.modifiers.alt {
                continue;
            }

            let movement = match event.key {
                Key::ArrowUp => Some(CollectionCursorMove::Previous),
                Key::ArrowDown => Some(CollectionCursorMove::Next),
                Key::Home => Some(CollectionCursorMove::First),
                Key::End => Some(CollectionCursorMove::Last),
                Key::PageUp => Some(CollectionCursorMove::PagePrevious {
                    rows: scene.page_rows(),
                }),
                Key::PageDown => Some(CollectionCursorMove::PageNext {
                    rows: scene.page_rows(),
                }),
                _ => None,
            };
            if let Some(movement) = movement {
                if let Some(target) =
                    navigate_selectable_outliner_target(scene, &mut state.cursor, movement)
                {
                    output.selection_changed |= apply_outliner_selection(
                        &mut state.selection,
                        scene,
                        target.id,
                        event.modifiers,
                        config.selection_mode,
                    );
                    final_focus = Some(target);
                }
                continue;
            }

            if matches!(event.key, Key::ArrowLeft | Key::ArrowRight) {
                let horizontal = navigate_outliner_horizontally(
                    scene,
                    &mut state.cursor,
                    &mut state.expansion,
                    event.key == Key::ArrowRight,
                );
                output.expansion_changed |= horizontal.toggled;
                if let Some(target) = horizontal.target
                    && scene
                        .model()
                        .item_by_id(target.id)
                        .is_some_and(|item| item.flags.can_request_selection())
                {
                    output.selection_changed |= apply_outliner_selection(
                        &mut state.selection,
                        scene,
                        target.id,
                        event.modifiers,
                        config.selection_mode,
                    );
                    final_focus = Some(target);
                }
                continue;
            }

            if event.repeat || !event.modifiers.is_empty() {
                continue;
            }
            match event.key {
                Key::Enter | Key::Space => {
                    *keyboard_activated = state.cursor.active();
                    output.activated = state.cursor.active();
                }
                Key::Function(2) => {
                    if let Some(begin) = scene
                        .model()
                        .inline_rename_begin_from_selection(&state.selection, scene.widget_id())
                    {
                        let text_widget = begin.text_widget_id;
                        self.register_id(text_widget);
                        output
                            .requests
                            .push(OutlinerRequest::Rename(state.begin_rename(begin, config)));
                        self.runtime.memory_mut().focus(text_widget);
                        self.request_repaint(RepaintRequest::NextFrame);
                        return;
                    }
                }
                _ => {}
            }
        }
        if let Some(target) = final_focus {
            self.focus_and_reveal_outliner_target(scene, target);
        }
    }

    fn focus_and_reveal_outliner_target(
        &mut self,
        scene: &OutlinerScene<'_>,
        target: CollectionCursorTarget,
    ) {
        let row_id = scene.row_widget_id(target.id);
        if scene.row(target.id).is_none() {
            self.register_id(row_id);
        }
        let reveal = scene.reveal_scroll_offset(target);
        let focus_changed = !self.memory().is_focused(row_id);
        let reveal_changed = reveal.to_bits() != scene.window().clamped_scroll_offset.to_bits();
        let memory = self.runtime.memory_mut();
        if focus_changed {
            memory.focus(row_id);
        }
        if reveal_changed {
            memory.stage_scroll_offset(scene.widget_id(), Vec2::new(0.0, reveal));
        }
        if focus_changed || reveal_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
    }

    fn evaluate_outliner_drop(
        &mut self,
        scene: &OutlinerScene<'_>,
        state: &mut OutlinerState,
        position: Option<Point>,
        gesture_released: bool,
        gesture_cancelled: bool,
        output: &mut OutlinerOutput,
    ) {
        let Some(drag) = state.drag.clone() else {
            return;
        };
        let released =
            gesture_released || self.memory().released_drag_source() == Some(drag.widget);
        let mut preview = None;
        let mut accepted = None;
        if !gesture_cancelled && !scene.config().disabled {
            for zones in scene.rows() {
                let drop_id = drop_widget_id(scene.row_widget_id(zones.row.id));
                let (input, memory) = self.runtime.input_and_memory_mut();
                let response =
                    drop_target(drop_id, zones.rect, input, memory, zones.row.flags.disabled);
                if response.source == Some(drag.widget)
                    && response.response.state.hovered
                    && let Some(point) = position
                    && let Some(target) = scene.resolve_drop(zones, point, &drag.source)
                {
                    preview = Some(target.clone());
                    if response.dropped {
                        accepted = Some(target);
                    }
                }
            }
        }

        let accepted_drop = accepted.is_some();
        if let Some(target) = accepted {
            output.requests.push(OutlinerRequest::Drop(target));
        }
        if released || gesture_cancelled || accepted_drop || scene.config().disabled {
            if self.memory().drag_source() == Some(drag.widget)
                || self.memory().released_drag_source() == Some(drag.widget)
            {
                self.runtime.memory_mut().clear_drag();
            }
            state.drag = None;
        } else {
            output.drop_preview = preview;
        }
    }

    fn paint_outliner_surface(&mut self, rect: Rect) {
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

    #[allow(clippy::too_many_arguments)]
    fn paint_outliner_row(
        &mut self,
        zones: &OutlinerRowZones,
        row: Response,
        disclosure: Option<Response>,
        visibility: Option<Response>,
        lock: Option<Response>,
        paint_label: bool,
    ) {
        let hovered = row.state.hovered
            || disclosure.is_some_and(|response| response.state.hovered)
            || visibility.is_some_and(|response| response.state.hovered)
            || lock.is_some_and(|response| response.state.hovered);
        let pressed = row.state.pressed
            || disclosure.is_some_and(|response| response.state.pressed)
            || visibility.is_some_and(|response| response.state.pressed)
            || lock.is_some_and(|response| response.state.pressed);
        let recipe = self.theme.row(ComponentState {
            hovered,
            pressed,
            focused: row.state.focused,
            disabled: row.state.disabled,
            selected: row.state.selected,
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: zones.rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));

        if zones.row.has_children {
            self.paint_outliner_disclosure(
                zones.disclosure_rect,
                zones.row.expanded,
                recipe.foreground,
            );
        }
        if zones.row.flags.visibility_toggle_available {
            self.paint_outliner_visibility(
                zones.visibility_toggle_rect,
                zones.row.flags.visible,
                recipe.foreground,
            );
        }
        if zones.row.flags.lock_toggle_available {
            self.paint_outliner_lock(
                zones.lock_toggle_rect,
                zones.row.flags.locked,
                recipe.foreground,
            );
        }
        if paint_label {
            let font = self.theme.font(TextRole::Label);
            let extra = (zones.label_rect.height - font.line_height).max(0.0) * 0.5;
            self.primitive(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(
                    zones.label_rect.x + self.theme.controls.padding_x,
                    zones.label_rect.y + extra + font.size,
                ),
                text: zones.row.label.clone(),
                family: font.family.to_owned(),
                size: font.size,
                line_height: font.line_height,
                brush: Brush::Solid(recipe.foreground),
            }));
        }
    }

    fn paint_outliner_disclosure(&mut self, rect: Rect, expanded: bool, color: stern_core::Color) {
        let center = rect.center();
        let half = rect.width.min(rect.height) * 0.16;
        let stroke = Stroke::new(self.theme.strokes.default, Brush::Solid(color));
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

    fn paint_outliner_visibility(&mut self, rect: Rect, visible: bool, color: stern_core::Color) {
        let stroke = Stroke::new(
            self.theme.strokes.default,
            Brush::Solid(if visible {
                color
            } else {
                color.with_alpha(0.5)
            }),
        );
        let inset = rect.width.min(rect.height) * 0.25;
        let icon = Rect::new(
            rect.x + inset,
            rect.y + inset,
            (rect.width - inset * 2.0).max(0.0),
            (rect.height - inset * 2.0).max(0.0),
        );
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: icon,
            fill: None,
            stroke: Some(stroke),
            radius: self.theme.radii.full,
        }));
        if !visible {
            self.primitive(Primitive::Line(LinePrimitive {
                from: Point::new(icon.x, icon.y),
                to: Point::new(icon.max_x(), icon.max_y()),
                stroke,
            }));
        }
    }

    fn paint_outliner_lock(&mut self, rect: Rect, locked: bool, color: stern_core::Color) {
        let stroke = Stroke::new(
            self.theme.strokes.default,
            Brush::Solid(if locked {
                color
            } else {
                color.with_alpha(0.55)
            }),
        );
        let width = rect.width * 0.42;
        let height = rect.height * 0.34;
        let body = Rect::new(
            rect.center().x - width * 0.5,
            rect.center().y,
            width,
            height,
        );
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: body,
            fill: if locked {
                Some(Brush::Solid(color))
            } else {
                None
            },
            stroke: Some(stroke),
            radius: self.theme.radii.sm,
        }));
        let shackle_y = body.y - height * 0.55;
        let shackle_x = if locked {
            body.x
        } else {
            body.x + width * 0.22
        };
        self.primitive(Primitive::Line(LinePrimitive {
            from: Point::new(shackle_x, body.y),
            to: Point::new(shackle_x, shackle_y),
            stroke,
        }));
        self.primitive(Primitive::Line(LinePrimitive {
            from: Point::new(shackle_x, shackle_y),
            to: Point::new(body.max_x(), shackle_y),
            stroke,
        }));
        self.primitive(Primitive::Line(LinePrimitive {
            from: Point::new(body.max_x(), shackle_y),
            to: Point::new(body.max_x(), body.y),
            stroke,
        }));
    }

    fn paint_outliner_drop_preview(
        &mut self,
        scene: &OutlinerScene<'_>,
        preview: &OutlinerDropTarget,
    ) {
        let Some(zones) = scene.row(preview.target) else {
            return;
        };
        let stroke = Stroke::new(
            self.theme.strokes.default,
            Brush::Solid(self.theme.colors.accent.default),
        );
        match preview.zone {
            OutlinerDropZoneKind::Inside => {
                self.primitive(Primitive::Rect(RectPrimitive {
                    rect: zones.rect,
                    fill: Some(Brush::Solid(
                        self.theme.colors.accent.default.with_alpha(0.16),
                    )),
                    stroke: Some(stroke),
                    radius: self.theme.radii.sm,
                }));
            }
            OutlinerDropZoneKind::Before | OutlinerDropZoneKind::After => {
                let y = if preview.zone == OutlinerDropZoneKind::Before {
                    zones.rect.y
                } else {
                    zones.rect.max_y()
                };
                self.primitive(Primitive::Line(LinePrimitive {
                    from: Point::new(zones.rect.x, y),
                    to: Point::new(zones.rect.max_x(), y),
                    stroke,
                }));
            }
        }
    }
}

fn apply_outliner_selection(
    selection: &mut Selection,
    scene: &OutlinerScene<'_>,
    id: ItemId,
    modifiers: Modifiers,
    mode: OutlinerSelectionMode,
) -> bool {
    let before = selection.clone();
    match mode {
        OutlinerSelectionMode::Multiple if modifiers.shift => {
            let visible = scene
                .projection()
                .items()
                .iter()
                .map(|item| item.id)
                .filter(|item| outliner_item_selectable(scene, *item))
                .collect::<Vec<_>>();
            if !selection.select_range(&visible, id) {
                selection.replace(id);
            }
        }
        OutlinerSelectionMode::Multiple if modifiers.ctrl || modifiers.super_key => {
            selection.toggle(id);
        }
        OutlinerSelectionMode::Single | OutlinerSelectionMode::Multiple => {
            selection.replace(id);
        }
    }
    *selection != before
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HorizontalOutlinerNavigation {
    target: Option<CollectionCursorTarget>,
    toggled: bool,
}

fn navigate_outliner_horizontally(
    scene: &OutlinerScene<'_>,
    cursor: &mut crate::CollectionCursor,
    expansion: &mut crate::TreeExpansion,
    right: bool,
) -> HorizontalOutlinerNavigation {
    let Some(active) = cursor.active() else {
        return HorizontalOutlinerNavigation::default();
    };
    let Some(item) = scene.model().item_by_id(active) else {
        return HorizontalOutlinerNavigation::default();
    };
    let tree = scene.model().tree_model();
    let children = tree.child_ids(Some(active));
    let has_children = item.has_children || !children.is_empty();

    if right {
        if has_children && !expansion.is_expanded(active) {
            expansion.expand(active);
            return HorizontalOutlinerNavigation {
                target: None,
                toggled: true,
            };
        }
        return HorizontalOutlinerNavigation {
            target: children
                .first()
                .filter(|child| outliner_item_selectable(scene, **child))
                .and_then(|child| cursor.activate(scene.projection(), *child)),
            toggled: false,
        };
    }

    if has_children && expansion.collapse(active) {
        return HorizontalOutlinerNavigation {
            target: None,
            toggled: true,
        };
    }
    HorizontalOutlinerNavigation {
        target: item
            .parent
            .filter(|parent| outliner_item_selectable(scene, *parent))
            .and_then(|parent| cursor.activate(scene.projection(), parent)),
        toggled: false,
    }
}

fn navigate_selectable_outliner_target(
    scene: &OutlinerScene<'_>,
    cursor: &mut crate::CollectionCursor,
    movement: CollectionCursorMove,
) -> Option<CollectionCursorTarget> {
    let old_active = cursor.active();
    let candidate = cursor.navigate(scene.projection(), movement)?;
    let indices: Box<dyn Iterator<Item = usize>> = match movement {
        CollectionCursorMove::First
        | CollectionCursorMove::Next
        | CollectionCursorMove::PageNext { .. } => {
            Box::new(candidate.projected_index..scene.projection().len())
        }
        CollectionCursorMove::Last
        | CollectionCursorMove::Previous
        | CollectionCursorMove::PagePrevious { .. } => {
            Box::new((0..=candidate.projected_index).rev())
        }
    };
    for index in indices {
        let Some(item) = scene.projection().get(index) else {
            continue;
        };
        if outliner_item_selectable(scene, item.id) {
            return cursor.activate(scene.projection(), item.id);
        }
    }
    if let Some(old_active) = old_active {
        cursor.activate(scene.projection(), old_active)
    } else {
        cursor.clear();
        None
    }
}

fn repair_outliner_cursor_target(
    scene: &OutlinerScene<'_>,
    cursor: &mut crate::CollectionCursor,
    target: CollectionCursorTarget,
) -> Option<CollectionCursorTarget> {
    if outliner_item_selectable(scene, target.id) {
        return Some(target);
    }
    let following = target.projected_index..scene.projection().len();
    let preceding = (0..target.projected_index).rev();
    for index in following.chain(preceding) {
        let item = scene.projection().get(index)?;
        if outliner_item_selectable(scene, item.id) {
            return cursor.activate(scene.projection(), item.id);
        }
    }
    cursor.clear();
    None
}

fn outliner_item_selectable(scene: &OutlinerScene<'_>, item: ItemId) -> bool {
    scene
        .model()
        .item_by_id(item)
        .is_some_and(|item| item.flags.can_request_selection())
}

fn outliner_context_anchor(position: Option<Point>, fallback: Rect) -> Rect {
    let point = position
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .unwrap_or_else(|| fallback.center());
    Rect::new(point.x, point.y, 1.0, 1.0)
}

fn outliner_context_viewport(ui: &Ui<'_>, fallback: Rect) -> Rect {
    let size = ui.viewport().logical_size;
    if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
        Rect::new(0.0, 0.0, size.width, size.height)
    } else {
        fallback
    }
}
