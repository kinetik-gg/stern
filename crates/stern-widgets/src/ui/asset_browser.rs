use std::hash::Hash;

use stern_core::{
    ActionContext, ActionDescriptor, ActionSource, Brush, ClipId, ComponentState,
    DomainDragGesturePhase, ImagePrimitive, InteractionState, Key, KeyState, Modifiers, Point,
    Primitive, Rect, RectPrimitive, RepaintRequest, Response, SemanticValue, Stroke, TextPrimitive,
    TextRole, Vec2, context_menu_trigger, drop_target, scrollable,
};

use super::Ui;
use crate::asset_browser::{
    AssetBrowserConfig, AssetBrowserContextState, AssetBrowserDragState, AssetBrowserDropTarget,
    AssetBrowserDropTargetKind, AssetBrowserItemRect, AssetBrowserItemResponse,
    AssetBrowserLayoutResult, AssetBrowserModel, AssetBrowserOutput, AssetBrowserRenameConflict,
    AssetBrowserRequest, AssetBrowserScene, AssetBrowserSelectionMode, AssetBrowserState,
    AssetBrowserViewMode, asset_browser_semantics, background_drop_widget_id, background_widget_id,
    context_overlay_id, drop_widget_id,
};
use crate::{
    CollectionContextActionRequest, CollectionContextTarget, CollectionCursorTarget,
    InlineEditCancelReason, InlineEditCommitReason, InlineEditFocusLossPolicy, InlineEditRequest,
    ItemId, Menu, MenuOverlay, OverlayDismissal, OverlayKind, OverlayScene, OverlaySceneIntent,
    OverlaySceneSurface, PopoverPlacement, Selection, TextFieldAccess, collection_context_actions,
};

impl Ui<'_> {
    /// Prepares one fixed-size virtualized asset-browser frame before pointer arbitration.
    ///
    /// Returns `None` for malformed models, empty or non-finite viewport
    /// geometry, or invalid metrics for the selected view mode. The returned
    /// snapshot must be shared by pointer declaration and [`Self::asset_browser`].
    #[must_use]
    pub fn prepare_asset_browser<'model>(
        &self,
        key: impl Hash,
        config: AssetBrowserConfig,
        model: &'model AssetBrowserModel,
        state: &AssetBrowserState,
    ) -> Option<AssetBrowserScene<'model>> {
        let root = self.make_id(key);
        let retained_scroll = self.memory().scroll_offset(root).y;
        AssetBrowserScene::prepare(root, config, model, state, retained_scroll)
    }

    /// Evaluates and paints one prepared reusable asset browser.
    ///
    /// Filtering, sorting, and view mode are application inputs. Selection and
    /// cursor state remain in [`AssetBrowserState`]. Preview, rename, drop, and
    /// context actions are emitted as typed requests. The rename validator
    /// returns a caller-owned conflict message and never embeds file rules.
    #[allow(clippy::too_many_lines)]
    pub fn asset_browser(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        state: &mut AssetBrowserState,
        mut rename_conflict: impl FnMut(ItemId, &str) -> Option<String>,
        mut context_actions: impl FnMut(&CollectionContextTarget) -> Vec<ActionDescriptor>,
    ) -> AssetBrowserOutput {
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

        let mut output = AssetBrowserOutput {
            scroll,
            view_mode: scene.layout().view_mode,
            visible_range: scene.layout().visible_range.clone(),
            materialized_range: scene.layout().materialized_range.clone(),
            selection_changed: false,
            context_opened: None,
            drag_payload: None,
            drop_preview: None,
            rename_conflict: state.rename_conflict().cloned(),
            requests: Vec::new(),
            responses: Vec::with_capacity(scene.layout().items.len()),
        };

        self.reconcile_asset_browser_retained_state(
            scene,
            state,
            &mut output,
            &mut rename_conflict,
        );
        let prepared_rename = state.rename_target();
        let context_was_prepared = scene.has_prepared_context() && state.context.is_some();
        let view_changed =
            state.view_mode.replace(scene.layout().view_mode) != Some(scene.layout().view_mode);

        self.reconcile_asset_browser_cursor(scene, state, view_changed);
        let mut keyboard_preview = None;
        self.handle_asset_browser_keyboard(scene, state, &mut output, &mut keyboard_preview);

        self.paint_asset_browser_surface(config.bounds);
        let strict_items = scene.strict_items().cloned().collect::<Vec<_>>();
        let mut prepared_semantics = asset_browser_semantics(
            root,
            config.bounds,
            &strict_asset_layout(scene, &strict_items, &state.selection),
            &config.label,
        );
        let mut root_semantics = prepared_semantics.remove(0);
        root_semantics.state.disabled = config.disabled;
        root_semantics.state.value = Some(SemanticValue::Text(format!(
            "{} items",
            scene.projection().len()
        )));
        if let Some(target) = prepared_rename
            && strict_items.iter().any(|item| item.item.id == target)
            && let Some(child) = root_semantics
                .children
                .iter_mut()
                .find(|child| **child == scene.item_widget_id(target))
        {
            *child = scene.rename_widget_id(target);
        }
        self.push_semantic_node(root_semantics);

        let clip = ClipId::from_raw(root.child("asset-browser-clip").raw());
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

        for item_rect in &scene.layout().items {
            let item = item_rect.item.id;
            let item_id = scene.item_widget_id(item);
            let editing = prepared_rename == Some(item) && state.rename_target() == Some(item);
            self.register_id(item_id);

            let mut response = if editing {
                Response::new(item_id, item_rect.rect, InteractionState::default())
            } else {
                let gesture = self.runtime.captured_domain_drag_gesture(
                    item_id,
                    item_rect.rect,
                    config.disabled || item_rect.item.state.disabled,
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

                let source_visible = self.memory().drag_source() == Some(item_id)
                    || self.memory().released_drag_source() == Some(item_id)
                    || gesture.response.dragged;
                if source_visible
                    && state.drag.is_none()
                    && let Some(source) = item_rect.item.drag_source(&state.selection)
                {
                    state.drag = Some(AssetBrowserDragState {
                        widget: item_id,
                        source,
                    });
                }

                let double_click_position = gesture
                    .actions
                    .iter()
                    .rev()
                    .find(|action| {
                        action.phase == DomainDragGesturePhase::Release
                            && action.release_clicked
                            && action.click_count >= 2
                    })
                    .and_then(|action| action.position)
                    .or(self.input().pointer.position);

                let mut response = gesture.response;
                response.keyboard_activated = keyboard_preview == Some(item);
                if response.clicked && !item_rect.item.state.disabled {
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
                        output.selection_changed |= apply_asset_browser_selection(
                            &mut state.selection,
                            scene,
                            target.id,
                            modifiers,
                            config.selection_mode,
                        );
                        self.focus_and_reveal_asset_browser_target(scene, target);
                    }
                }

                if response.double_clicked {
                    let rename_hit = double_click_position
                        .is_some_and(|point| item_rect.name_rect.contains_point(point));
                    if rename_hit
                        && let Some(begin) = item_rect.item.inline_rename_begin_request(root)
                    {
                        let text_widget = begin.text_widget_id;
                        self.register_id(text_widget);
                        output.requests.push(AssetBrowserRequest::Rename(
                            state.begin_rename(begin, config),
                        ));
                        self.runtime.memory_mut().focus(text_widget);
                        self.request_repaint(RepaintRequest::NextFrame);
                    } else if !item_rect.item.state.disabled {
                        output.requests.push(AssetBrowserRequest::Preview(item));
                    }
                }

                let pointer_context_requested = response.secondary_clicked;
                let context_response = {
                    let (input, memory) = self.runtime.input_and_memory_mut();
                    context_menu_trigger(
                        item_id,
                        item_rect.rect,
                        input,
                        memory,
                        config.disabled || item_rect.item.state.disabled,
                    )
                };
                if (pointer_context_requested || context_response.context_requested)
                    && let Some(target) =
                        asset_browser_context_target(scene, item_rect, &state.selection)
                {
                    pending_context.get_or_insert_with(|| {
                        (
                            target,
                            item_id,
                            asset_browser_context_anchor(
                                self.input().pointer.position,
                                item_rect.rect,
                            ),
                        )
                    });
                }
                response
            };

            response.state.disabled = config.disabled || item_rect.item.state.disabled;
            response.state.selected = state.selection.contains(item);
            response.state.focused = self.memory().is_focused(item_id);
            self.paint_asset_browser_item(item_rect, response, !editing);

            if editing {
                rename_evaluated = true;
                let rename_id = self.register_id(scene.rename_widget_id(item));
                if let Some(edit) = state.edit.as_mut() {
                    let (field, ordered) = self.text_field_with_access_id(
                        rename_id,
                        item_rect.name_rect,
                        &mut edit.text,
                        TextFieldAccess::Editable,
                    );
                    if field.changed {
                        edit.conflict = None;
                        output.rename_conflict = None;
                        let draft = edit.session.set_draft(edit.text.text.clone());
                        output.requests.push(AssetBrowserRequest::Rename(
                            InlineEditRequest::DraftEdit(draft),
                        ));
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

            if response.clicked
                || response.double_clicked
                || response.keyboard_activated
                || response.state.pressed
            {
                self.request_repaint(RepaintRequest::NextFrame);
            }
            output
                .responses
                .push(AssetBrowserItemResponse { item, response });
        }

        let final_semantics = asset_browser_semantics(
            root,
            config.bounds,
            &strict_asset_layout(scene, &strict_items, &state.selection),
            &config.label,
        );
        for (item_rect, mut semantic) in
            strict_items.iter().zip(final_semantics.into_iter().skip(1))
        {
            if prepared_rename == Some(item_rect.item.id) {
                continue;
            }
            if let Some(item_response) = output
                .responses
                .iter()
                .find(|response| response.item == item_rect.item.id)
            {
                semantic.state.focused = item_response.response.state.focused;
                semantic.state.pressed = item_response.response.state.pressed;
            }
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
                    asset_browser_context_anchor(self.input().pointer.position, config.bounds),
                )
            });
        }

        if let Some((target, trigger, anchor)) = pending_context {
            let descriptors = collection_context_actions(&target, context_actions(&target))
                .into_iter()
                .map(|action| action.descriptor)
                .collect::<Vec<_>>();
            if !descriptors.is_empty() {
                let viewport = asset_browser_context_viewport(self, config.bounds);
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
                overlay.push(OverlaySceneSurface::menu("Asset actions", menu));
                self.runtime.memory_mut().focus(trigger);
                state.context = Some(AssetBrowserContextState {
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
            match validate_asset_browser_rename(request, &mut rename_conflict) {
                Ok(request) => {
                    output.requests.push(AssetBrowserRequest::Rename(request));
                    state.clear_rename();
                    output.rename_conflict = None;
                    self.runtime
                        .memory_mut()
                        .focus(scene.item_widget_id(target));
                    self.request_repaint(RepaintRequest::NextFrame);
                }
                Err(conflict) => {
                    if let Some(edit) = state.edit.as_mut() {
                        edit.conflict = Some(conflict.clone());
                    }
                    output.rename_conflict = Some(conflict);
                    self.runtime
                        .memory_mut()
                        .focus(scene.rename_widget_id(target));
                    self.request_repaint(RepaintRequest::NextFrame);
                }
            }
        }

        self.evaluate_asset_browser_drop(
            scene,
            state,
            drag_position,
            drag_released,
            drag_cancelled,
            &mut output,
        );
        output.drag_payload = state.drag_source().cloned();
        if let Some(preview) = output.drop_preview.as_ref() {
            self.paint_asset_browser_drop_preview(scene, preview);
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
                        output.requests.push(AssetBrowserRequest::Context(
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

        if output.selection_changed || !output.requests.is_empty() {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    fn reconcile_asset_browser_retained_state(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        state: &mut AssetBrowserState,
        output: &mut AssetBrowserOutput,
        rename_conflict: &mut impl FnMut(ItemId, &str) -> Option<String>,
    ) {
        let config = scene.config();
        let context_valid = state
            .context
            .as_ref()
            .is_none_or(|context| scene.context_target_valid(&context.target));
        if (config.disabled || !context_valid) && state.context.take().is_some() {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        if let Some(drag) = state.drag.as_ref() {
            let owner_retained = self.memory().drag_source() == Some(drag.widget)
                || self.memory().released_drag_source() == Some(drag.widget);
            let source_visible = scene.strict_items().any(|item| {
                item.item.id == drag.source.source
                    && !item.item.state.disabled
                    && !item.item.state.read_only
            });
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
                .is_some_and(|item| !item.disabled && !item.read_only && item.renamable);
        let projected_index = scene.projection().projected_index(target);
        if !eligible || projected_index.is_none() {
            let text_widget = state.edit.as_ref().map(|edit| edit.session.text_widget_id);
            if let Some(edit) = state.edit.as_ref() {
                output
                    .requests
                    .push(AssetBrowserRequest::Rename(InlineEditRequest::Cancel(
                        edit.session
                            .cancel_request(InlineEditCancelReason::Explicit),
                    )));
            }
            state.clear_rename();
            output.rename_conflict = None;
            if text_widget.is_some_and(|text_widget| self.memory().is_focused(text_widget)) {
                self.runtime.memory_mut().clear_focus();
            }
            self.request_repaint(RepaintRequest::NextFrame);
            return;
        }

        if scene.strict_items().any(|item| item.item.id == target) {
            return;
        }
        let cursor_target = CollectionCursorTarget {
            id: target,
            projected_index: projected_index.unwrap_or_default(),
        };
        let focus_loss_policy = state
            .edit
            .as_ref()
            .map(|edit| edit.session.focus_loss_policy);
        if focus_loss_policy == Some(InlineEditFocusLossPolicy::KeepEditing) {
            self.retain_and_reveal_asset_browser_rename(scene, cursor_target);
            return;
        }

        let request = state
            .edit
            .as_ref()
            .and_then(|edit| edit.session.focus_loss_request());
        if let Some(request) = request {
            match validate_asset_browser_rename(request, rename_conflict) {
                Ok(request) => {
                    output.requests.push(AssetBrowserRequest::Rename(request));
                    state.clear_rename();
                    output.rename_conflict = None;
                    self.focus_and_reveal_asset_browser_target(scene, cursor_target);
                }
                Err(conflict) => {
                    if let Some(edit) = state.edit.as_mut() {
                        edit.conflict = Some(conflict.clone());
                    }
                    output.rename_conflict = Some(conflict);
                    self.retain_and_reveal_asset_browser_rename(scene, cursor_target);
                }
            }
        } else {
            self.retain_and_reveal_asset_browser_rename(scene, cursor_target);
        }
    }

    fn retain_and_reveal_asset_browser_rename(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        target: CollectionCursorTarget,
    ) {
        self.register_id(scene.rename_widget_id(target.id));
        let reveal = scene.reveal_scroll_offset(target);
        self.runtime
            .memory_mut()
            .stage_scroll_offset(scene.widget_id(), Vec2::new(0.0, reveal));
        self.request_repaint(RepaintRequest::NextFrame);
    }

    fn reconcile_asset_browser_cursor(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        state: &mut AssetBrowserState,
        view_changed: bool,
    ) {
        let old_active = state.cursor.active();
        let old_index = state.cursor.last_projected_index();
        let old_focused =
            old_active.is_some_and(|item| self.memory().is_focused(scene.item_widget_id(item)));
        let repaired = state
            .cursor
            .reconcile(scene.projection())
            .and_then(|target| {
                repair_asset_browser_cursor_target(scene, &mut state.cursor, target)
            });
        let changed = old_active != state.cursor.active()
            || old_index != state.cursor.last_projected_index()
            || view_changed;
        if old_focused && changed {
            if let Some(target) = repaired {
                self.focus_and_reveal_asset_browser_target(scene, target);
            } else {
                self.runtime.memory_mut().clear_focus();
                self.request_repaint(RepaintRequest::NextFrame);
            }
        }
    }

    fn handle_asset_browser_keyboard(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        state: &mut AssetBrowserState,
        output: &mut AssetBrowserOutput,
        keyboard_preview: &mut Option<ItemId>,
    ) {
        let config = scene.config();
        if config.disabled || state.edit.is_some() || state.context.is_some() {
            return;
        }
        let Some(active) = state.cursor.active() else {
            return;
        };
        if !self.memory().is_focused(scene.item_widget_id(active)) {
            return;
        }

        let events = self.input().keyboard.events.clone();
        let mut final_focus = None;
        for event in events {
            if event.state != KeyState::Pressed || event.modifiers.alt {
                continue;
            }

            if let Some(target) =
                navigate_asset_browser_target(scene, &mut state.cursor, &event.key)
            {
                output.selection_changed |= apply_asset_browser_selection(
                    &mut state.selection,
                    scene,
                    target.id,
                    event.modifiers,
                    config.selection_mode,
                );
                final_focus = Some(target);
                continue;
            }

            if event.repeat || !event.modifiers.is_empty() {
                continue;
            }
            match event.key {
                Key::Enter | Key::Space => {
                    *keyboard_preview = state.cursor.active();
                    if let Some(item) = state.cursor.active() {
                        output.requests.push(AssetBrowserRequest::Preview(item));
                    }
                }
                Key::Function(2) => {
                    if let Some(active) = state.cursor.active()
                        && state.selection.contains(active)
                        && let Some(begin) = scene
                            .model()
                            .item_by_id(active)
                            .and_then(|item| item.inline_rename_begin_request(scene.widget_id()))
                    {
                        let text_widget = begin.text_widget_id;
                        self.register_id(text_widget);
                        output.requests.push(AssetBrowserRequest::Rename(
                            state.begin_rename(begin, config),
                        ));
                        self.runtime.memory_mut().focus(text_widget);
                        self.request_repaint(RepaintRequest::NextFrame);
                        return;
                    }
                }
                _ => {}
            }
        }
        if let Some(target) = final_focus {
            self.focus_and_reveal_asset_browser_target(scene, target);
        }
    }

    fn focus_and_reveal_asset_browser_target(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        target: CollectionCursorTarget,
    ) {
        let item_id = scene.item_widget_id(target.id);
        if scene.item(target.id).is_none() {
            self.register_id(item_id);
        }
        let reveal = scene.reveal_scroll_offset(target);
        let focus_changed = !self.memory().is_focused(item_id);
        let reveal_changed = reveal.to_bits() != scene.layout().scroll_offset.to_bits();
        let memory = self.runtime.memory_mut();
        if focus_changed {
            memory.focus(item_id);
        }
        if reveal_changed {
            memory.stage_scroll_offset(scene.widget_id(), Vec2::new(0.0, reveal));
        }
        if focus_changed || reveal_changed {
            self.request_repaint(RepaintRequest::NextFrame);
        }
    }

    fn evaluate_asset_browser_drop(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        state: &mut AssetBrowserState,
        position: Option<Point>,
        gesture_released: bool,
        gesture_cancelled: bool,
        output: &mut AssetBrowserOutput,
    ) {
        let Some(drag) = state.drag.clone() else {
            return;
        };
        let released =
            gesture_released || self.memory().released_drag_source() == Some(drag.widget);
        let mut preview = None;
        let mut accepted = None;
        if !gesture_cancelled && !scene.config().disabled {
            for item in &scene.layout().items {
                let drop_id = drop_widget_id(scene.item_widget_id(item.item.id));
                let (input, memory) = self.runtime.input_and_memory_mut();
                let response =
                    drop_target(drop_id, item.rect, input, memory, item.item.state.disabled);
                if response.source == Some(drag.widget)
                    && response.response.state.hovered
                    && let Some(point) = position
                    && let Some(target) = scene.resolve_drop(point, &drag.source)
                {
                    preview = Some(target.clone());
                    if response.dropped {
                        accepted = Some(target);
                    }
                }
            }

            if preview.is_none() && accepted.is_none() {
                let (input, memory) = self.runtime.input_and_memory_mut();
                let response = drop_target(
                    background_drop_widget_id(scene.widget_id()),
                    scene.config().bounds,
                    input,
                    memory,
                    false,
                );
                if response.source == Some(drag.widget)
                    && response.response.state.hovered
                    && let Some(point) = position
                    && let Some(target) = scene.resolve_drop(point, &drag.source)
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
            output.requests.push(AssetBrowserRequest::Drop(target));
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

    fn paint_asset_browser_surface(&mut self, rect: Rect) {
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

    fn paint_asset_browser_item(
        &mut self,
        item: &AssetBrowserItemRect,
        response: Response,
        paint_name: bool,
    ) {
        let recipe = self.theme.row(ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: response.state.disabled,
            selected: response.state.selected,
        });
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: item.rect,
            fill: Some(recipe.background),
            stroke: Some(recipe.border),
            radius: recipe.radius,
        }));
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: item.preview_rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.raised)),
            stroke: Some(Stroke::new(
                self.theme.controls.border_width,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.sm,
        }));
        if let Some(image) = item.item.thumbnail {
            self.primitive(Primitive::Image(ImagePrimitive {
                image,
                rect: item.preview_rect,
                tint: response
                    .state
                    .disabled
                    .then_some(self.theme.colors.content.disabled),
            }));
        } else {
            paint_asset_text(
                self,
                item.preview_rect,
                &item.item.fallback.label,
                TextRole::Label,
                if response.state.disabled {
                    self.theme.colors.content.disabled
                } else {
                    self.theme.colors.content.muted
                },
            );
        }
        if paint_name {
            paint_asset_text(
                self,
                item.name_rect,
                &item.item.name,
                TextRole::Label,
                recipe.foreground,
            );
        }
        paint_asset_text(
            self,
            item.kind_rect,
            &item.item.kind,
            TextRole::Body,
            if response.state.disabled {
                self.theme.colors.content.disabled
            } else {
                self.theme.colors.content.muted
            },
        );
    }

    fn paint_asset_browser_drop_preview(
        &mut self,
        scene: &AssetBrowserScene<'_>,
        preview: &AssetBrowserDropTarget,
    ) {
        let stroke = Stroke::new(
            self.theme.controls.border_width.max(1.0),
            Brush::Solid(self.theme.colors.accent.default),
        );
        let rect = match preview.kind {
            AssetBrowserDropTargetKind::Item { target } => {
                let Some(item) = scene.item(target) else {
                    return;
                };
                item.rect
            }
            AssetBrowserDropTargetKind::EmptySpace { .. } => scene.config().bounds,
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(
                self.theme.colors.accent.default.with_alpha(0.12),
            )),
            stroke: Some(stroke),
            radius: self.theme.radii.sm,
        }));
    }
}

fn apply_asset_browser_selection(
    selection: &mut Selection,
    scene: &AssetBrowserScene<'_>,
    id: ItemId,
    modifiers: Modifiers,
    mode: AssetBrowserSelectionMode,
) -> bool {
    let before = selection.clone();
    match mode {
        AssetBrowserSelectionMode::Multiple if modifiers.shift => {
            let visible = scene
                .projection()
                .items()
                .iter()
                .map(|item| item.id)
                .filter(|item| asset_browser_item_selectable(scene, *item))
                .collect::<Vec<_>>();
            if !selection.select_range(&visible, id) {
                selection.replace(id);
            }
        }
        AssetBrowserSelectionMode::Multiple if modifiers.ctrl || modifiers.super_key => {
            selection.toggle(id);
        }
        AssetBrowserSelectionMode::Single | AssetBrowserSelectionMode::Multiple => {
            selection.replace(id);
        }
    }
    *selection != before
}

fn navigate_asset_browser_target(
    scene: &AssetBrowserScene<'_>,
    cursor: &mut crate::CollectionCursor,
    key: &Key,
) -> Option<CollectionCursorTarget> {
    let projection = scene.projection();
    if projection.is_empty() {
        cursor.clear();
        return None;
    }
    let current = cursor.reconcile(projection)?.projected_index;
    let last = projection.len() - 1;
    let columns = scene.layout().columns.max(1);
    let page = scene.page_items();
    let (candidate, direction, step) = match key {
        Key::Home => (0, 1_i8, 1),
        Key::End => (last, -1, 1),
        Key::ArrowLeft if scene.layout().view_mode == AssetBrowserViewMode::Grid => {
            (current.saturating_sub(1), -1, 1)
        }
        Key::ArrowRight if scene.layout().view_mode == AssetBrowserViewMode::Grid => {
            (current.saturating_add(1).min(last), 1, 1)
        }
        Key::ArrowUp => {
            let step = if scene.layout().view_mode == AssetBrowserViewMode::Grid {
                columns
            } else {
                1
            };
            (current.saturating_sub(step), -1, step)
        }
        Key::ArrowDown => {
            let step = if scene.layout().view_mode == AssetBrowserViewMode::Grid {
                columns
            } else {
                1
            };
            (current.saturating_add(step).min(last), 1, step)
        }
        Key::PageUp => (current.saturating_sub(page), -1, page),
        Key::PageDown => (current.saturating_add(page).min(last), 1, page),
        _ => return None,
    };

    let mut index = candidate;
    loop {
        let item = projection.get(index)?;
        if asset_browser_item_selectable(scene, item.id) {
            return cursor.activate(projection, item.id);
        }
        let next = if direction > 0 {
            index.saturating_add(step).min(last)
        } else {
            index.saturating_sub(step)
        };
        if next == index {
            break;
        }
        index = next;
    }
    cursor
        .active()
        .and_then(|active| cursor.activate(projection, active))
}

fn repair_asset_browser_cursor_target(
    scene: &AssetBrowserScene<'_>,
    cursor: &mut crate::CollectionCursor,
    target: CollectionCursorTarget,
) -> Option<CollectionCursorTarget> {
    if asset_browser_item_selectable(scene, target.id) {
        return Some(target);
    }
    let following = target.projected_index..scene.projection().len();
    let preceding = (0..target.projected_index).rev();
    for index in following.chain(preceding) {
        let item = scene.projection().get(index)?;
        if asset_browser_item_selectable(scene, item.id) {
            return cursor.activate(scene.projection(), item.id);
        }
    }
    cursor.clear();
    None
}

fn asset_browser_item_selectable(scene: &AssetBrowserScene<'_>, item: ItemId) -> bool {
    scene
        .model()
        .item_by_id(item)
        .is_some_and(|item| !item.disabled)
}

fn asset_browser_context_target(
    scene: &AssetBrowserScene<'_>,
    item: &AssetBrowserItemRect,
    selection: &Selection,
) -> Option<CollectionContextTarget> {
    if item.item.state.disabled {
        return None;
    }
    if selection.contains(item.item.id) {
        let visible_selection = scene
            .projection()
            .items()
            .iter()
            .map(|projected| projected.id)
            .filter(|id| selection.contains(*id));
        CollectionContextTarget::selection(visible_selection)
    } else {
        Some(CollectionContextTarget::item(item.item.id))
    }
}

fn validate_asset_browser_rename(
    request: InlineEditRequest,
    validator: &mut impl FnMut(ItemId, &str) -> Option<String>,
) -> Result<InlineEditRequest, AssetBrowserRenameConflict> {
    if let InlineEditRequest::Commit(commit) = &request
        && let Some(message) = validator(commit.target, &commit.draft_text)
    {
        return Err(AssetBrowserRenameConflict {
            target: commit.target,
            draft_text: commit.draft_text.clone(),
            message,
        });
    }
    Ok(request)
}

fn strict_asset_layout(
    scene: &AssetBrowserScene<'_>,
    items: &[AssetBrowserItemRect],
    selection: &Selection,
) -> AssetBrowserLayoutResult {
    let mut layout = scene.layout().clone();
    layout.items = items.to_vec();
    for item in &mut layout.items {
        item.item.state.selected = selection.contains(item.item.id);
    }
    layout
}

fn paint_asset_text(
    ui: &mut Ui<'_>,
    rect: Rect,
    text: &str,
    role: TextRole,
    color: stern_core::Color,
) {
    let font = ui.theme.font(role);
    let extra = (rect.height - font.line_height).max(0.0) * 0.5;
    ui.primitive(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(
            rect.x + ui.theme.controls.padding_x.min(rect.width * 0.25),
            rect.y + extra + font.size,
        ),
        text: text.to_owned(),
        family: font.family.to_owned(),
        size: font.size,
        line_height: font.line_height,
        brush: Brush::Solid(color),
    }));
}

fn asset_browser_context_anchor(position: Option<Point>, fallback: Rect) -> Rect {
    let point = position
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .unwrap_or_else(|| fallback.center());
    Rect::new(point.x, point.y, 1.0, 1.0)
}

fn asset_browser_context_viewport(ui: &Ui<'_>, fallback: Rect) -> Rect {
    let size = ui.viewport().logical_size;
    if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
        Rect::new(0.0, 0.0, size.width, size.height)
    } else {
        fallback
    }
}
