use std::hash::Hash;

use crate::input::{InputStreamConflict, UiInput, UiInputEvent};
use crate::interaction::{
    captured_domain_drag_gesture_with_ordinals, captured_selection_gesture_with_ordinals,
};
use crate::memory::{TextInputOwnerMode, UiMemory};
use crate::render::Primitive;
use crate::{
    ActionContext, ActionId, ActionInvocation, ActionSource, CapturedDomainDragGesture,
    CapturedSelectionGesture, IdStack, LivenessRemovalStatus, LivenessTargetId, LivenessToken,
    LivenessUpdateStatus, Modifiers, MouseButton, OrderedTextInputEvent, Rect, SemanticActionKind,
    SemanticNode, WidgetId,
};

use super::focus::{
    apply_escape_text_blur, apply_keyboard_focus_traversal, apply_pointer_text_owner_blur,
    apply_window_focus_text_blur,
};
use super::output::FrameOutput;
use super::pointer::{
    PointerDropProbe, PointerPlanError, PointerPressProbe, PointerTargetPlan, RetainedDragProbe,
};
use super::primitive_stack::validate_primitive_stack;
use super::spatial::SpatialStack;
use super::types::{CursorShape, FrameContext, FrameWarning, PlatformRequest, RepaintRequest};

/// Frame-local UI runtime builder.
///
/// This type owns stable ID derivation and the frame output accumulator. Widget
/// crates can layer ergonomic component APIs on top without becoming the
/// lowest-level runtime abstraction.
pub struct Ui<'a> {
    context: FrameContext,
    root_input: UiInput,
    input_event_ordinals: Vec<usize>,
    root_primary_transaction_open: Vec<bool>,
    root_event_modifiers: Vec<Modifiers>,
    last_root_primary_press_ordinal: Option<usize>,
    memory: &'a mut UiMemory,
    ids: IdStack,
    output: FrameOutput,
    spatial: SpatialStack,
    pointer_plan_installed: bool,
    pointer_cancel_pending: bool,
}

impl<'a> Ui<'a> {
    /// Starts a UI frame and clears transient retained memory.
    #[must_use]
    pub fn begin_frame(context: FrameContext, memory: &'a mut UiMemory) -> Self {
        memory.begin_frame();
        let input_validation = context.input.validate_event_stream();
        let input_conflict = input_validation.as_ref().err().copied();
        memory.set_root_input_validation(input_validation);
        let (entry_modifiers, suspended) = memory.ordered_modifier_state();
        let modifier_fold = fold_root_event_modifiers(
            &context.input,
            input_conflict.is_some(),
            entry_modifiers,
            suspended,
        );
        memory.set_ordered_modifier_state(modifier_fold.retained, modifier_fold.suspended);
        let pointer_cancel_pending = !context.input.events.is_empty()
            && context.input.events.iter().any(|event| {
                matches!(
                    event,
                    UiInputEvent::PointerReleaseAll { .. }
                        | UiInputEvent::WindowFocusChanged(false)
                )
            });
        let legacy_pointer_cancel = context.input.events.is_empty()
            && (!context.input.window_focused || pointer_release_all_cancelled(&context.input));
        if legacy_pointer_cancel {
            memory.cancel_pointer_interaction();
        }
        let root_input = context.input.clone();
        let root_primary_transaction_open = primary_transaction_open_before_events(
            &root_input,
            memory.has_primary_pointer_transaction(),
        );
        memory.install_scoped_pointer_events(
            0..root_input.events.len(),
            [],
            root_primary_transaction_open.iter().copied(),
        );
        let last_root_primary_press_ordinal = root_input.events.iter().rposition(|event| {
            matches!(
                event,
                UiInputEvent::PointerButton {
                    button: MouseButton::Primary,
                    down: true,
                    ..
                }
            )
        });
        let input_event_ordinals = (0..root_input.events.len()).collect();
        let mut output = FrameOutput::new();
        if let Some(conflict) = input_conflict {
            output.push_warning(FrameWarning::InputStreamConflict { conflict });
        }
        Self {
            context,
            root_input,
            input_event_ordinals,
            root_primary_transaction_open,
            root_event_modifiers: modifier_fold.by_ordinal,
            last_root_primary_press_ordinal,
            memory,
            ids: IdStack::new(),
            output,
            spatial: SpatialStack::default(),
            pointer_plan_installed: false,
            pointer_cancel_pending,
        }
    }

    /// Returns the frame context with input localized to the current render scope.
    #[must_use]
    pub const fn context(&self) -> &FrameContext {
        &self.context
    }

    /// Returns the input snapshot localized to the current render scope.
    #[must_use]
    pub const fn input(&self) -> &UiInput {
        &self.context.input
    }

    /// Returns the final canonical root ordinal that pressed the primary button.
    ///
    /// Spatial localization never changes this root-input provenance. Legacy
    /// empty streams and canonical streams without a primary Press return
    /// `None`.
    #[doc(hidden)]
    #[must_use]
    pub const fn last_root_primary_press_ordinal(&self) -> Option<usize> {
        self.last_root_primary_press_ordinal
    }

    /// Returns retained UI memory.
    #[must_use]
    pub fn memory(&self) -> &UiMemory {
        self.memory
    }

    /// Returns mutable retained UI memory.
    pub fn memory_mut(&mut self) -> &mut UiMemory {
        self.memory
    }

    /// Returns input and mutable memory as separate borrows for widget composition.
    pub fn input_and_memory_mut(&mut self) -> (&UiInput, &mut UiMemory) {
        (&self.context.input, self.memory)
    }

    /// Resolves a neutral captured pointer gesture for text selection.
    ///
    /// Canonical actions retain their original root event ordinals even when
    /// the current spatial scope filtered earlier events. Legacy snapshot
    /// actions use `None`. Selection capture never creates a domain drag source.
    pub fn captured_selection_gesture(
        &mut self,
        id: WidgetId,
        rect: Rect,
        disabled: bool,
    ) -> CapturedSelectionGesture {
        let mut gesture = captured_selection_gesture_with_ordinals(
            id,
            rect,
            &self.context.input,
            &self.input_event_ordinals,
            self.memory,
            disabled,
        );
        for action in &mut gesture.actions {
            if let Some(ordinal) = action.ordinal {
                debug_assert!(ordinal < self.root_event_modifiers.len());
                action.modifiers = self
                    .root_event_modifiers
                    .get(ordinal)
                    .copied()
                    .unwrap_or_default();
            }
        }
        gesture
    }

    /// Resolves one authoritative domain-drag response with ordered actions.
    ///
    /// Canonical actions retain their original root event ordinals even when
    /// the current spatial scope filtered earlier events. Legacy snapshot
    /// actions use `None`. Ordinary and captured domain-drag calls for the same
    /// widget share the first response in a frame; actions are delivered only
    /// when this captured method is the first claimant.
    pub fn captured_domain_drag_gesture(
        &mut self,
        id: WidgetId,
        rect: Rect,
        disabled: bool,
    ) -> CapturedDomainDragGesture {
        let mut gesture = captured_domain_drag_gesture_with_ordinals(
            id,
            rect,
            &self.context.input,
            &self.input_event_ordinals,
            self.memory,
            disabled,
        );
        for action in &mut gesture.actions {
            if let Some(ordinal) = action.ordinal {
                debug_assert!(ordinal < self.root_event_modifiers.len());
                action.modifiers = self
                    .root_event_modifiers
                    .get(ordinal)
                    .copied()
                    .unwrap_or_default();
            }
        }
        gesture
    }

    /// Claims editing-domain input with original root event ordinals.
    ///
    /// A successful canonical claim returns `Some` ordinals even when spatial
    /// filtering removed earlier pointer events. Legacy synthesized input uses
    /// `None`. `Ok(None)` means the caller did not own or had already claimed
    /// this frame's editing stream.
    ///
    /// # Errors
    ///
    /// Returns the root input conflict recorded for this frame.
    pub fn claim_ordered_text_input_events(
        &mut self,
        id: WidgetId,
    ) -> Result<Option<Vec<OrderedTextInputEvent>>, InputStreamConflict> {
        if !self.memory.claim_text_input_events(id) {
            return match self.memory.root_input_conflict() {
                Some(conflict) => Err(conflict),
                None => Ok(None),
            };
        }

        let events = self
            .memory
            .effective_text_input_events(&self.context.input)?;
        if self.root_input.events.is_empty() {
            return Ok(Some(
                events
                    .into_iter()
                    .filter(is_editing_domain_event)
                    .map(|event| OrderedTextInputEvent {
                        ordinal: None,
                        event,
                    })
                    .collect(),
            ));
        }

        debug_assert_eq!(events.len(), self.input_event_ordinals.len());
        Ok(Some(
            events
                .into_iter()
                .zip(self.input_event_ordinals.iter().copied())
                .filter_map(|(event, ordinal)| {
                    is_editing_domain_event(&event).then_some(OrderedTextInputEvent {
                        ordinal: Some(ordinal),
                        event,
                    })
                })
                .collect(),
        ))
    }

    /// Prepares a focused text widget to own the ordered editing domain.
    ///
    /// Editable owners remain platform-inactive until they publish an accepted
    /// caret rectangle. Read-only owners never activate platform text input.
    pub fn prepare_text_input_owner(&mut self, owner: WidgetId, mode: TextInputOwnerMode) -> bool {
        self.ids.mark_seen(owner);
        if !self.memory.is_focused(owner) || !self.spatial.is_visible() {
            return false;
        }

        self.memory.set_text_input_owner_mode(owner, mode);
        true
    }

    /// Publishes visible caret geometry for the focused Editable text owner.
    ///
    /// Rejected geometry is side-effect-free so a clipped caret can retain the
    /// platform's previous rectangle while its viewport stages a reveal.
    pub fn publish_text_input_rect(&mut self, owner: WidgetId, rect: Rect) -> bool {
        if !self.memory.is_focused(owner)
            || self.memory.text_input_owner() != Some(owner)
            || self.memory.text_input_owner_mode() != Some(TextInputOwnerMode::Editable)
            || !self.spatial.is_visible()
        {
            return false;
        }
        let Some(rect) = self.spatial.project_rect(rect) else {
            return false;
        };

        self.drain_pending_text_input_stop();
        if self.memory.platform_text_input_is_active_for(owner) {
            self.output
                .push_platform_request(PlatformRequest::UpdateTextInputRect { rect });
        } else {
            self.memory.activate_platform_text_input(owner);
            self.output
                .push_platform_request(PlatformRequest::StartTextInput { rect: Some(rect) });
        }
        true
    }

    /// Stages a retained scroll offset for the following frame.
    #[doc(hidden)]
    pub fn stage_scroll_offset(&mut self, owner: WidgetId, offset: crate::Vec2) {
        self.memory.stage_scroll_offset(owner, offset);
    }

    /// Derives and registers a widget ID in the current scope.
    pub fn id(&mut self, key: impl Hash) -> WidgetId {
        self.ids.register_key(key)
    }

    /// Derives a widget ID without registering it yet.
    ///
    /// Layout code uses this to predeclare pointer targets before the matching
    /// widget call registers the same ID.
    #[must_use]
    pub fn make_id(&self, key: impl Hash) -> WidgetId {
        self.ids.make_id(key)
    }

    /// Resolves and installs one closed-world pointer target plan for this frame.
    ///
    /// The plan must be complete before the first routed behavior call. Its
    /// transform and clip scopes reuse the current RT-01 spatial state, while
    /// explicit paint ordinals make declaration order irrelevant.
    ///
    /// # Errors
    ///
    /// Returns [`PointerPlanError`] for a second plan, duplicate paint order,
    /// or widget ID assigned to different visual descriptors. Invalid plans
    /// install blocked-safe routes.
    pub fn resolve_pointer_targets(
        &mut self,
        declare: impl FnOnce(&mut PointerTargetPlan),
    ) -> Result<crate::PointerRoutes, PointerPlanError> {
        if self.pointer_plan_installed {
            self.memory.cancel_pointer_interaction();
            self.memory
                .install_pointer_routes(crate::PointerRoutes::BLOCKED, [], None, None);
            return Err(PointerPlanError::AlreadyInstalled);
        }
        self.pointer_plan_installed = true;

        let captured = self.memory.pointer_capture();
        let retained_drag = self
            .memory
            .domain_drag_gesture()
            .map(|(source, origin, threshold_crossed)| RetainedDragProbe {
                source,
                origin: Some(origin),
                threshold_crossed,
            })
            .or_else(|| {
                self.memory.drag_source().map(|source| RetainedDragProbe {
                    source,
                    origin: None,
                    threshold_crossed: true,
                })
            });
        let (press_probe, drop_probe) = pointer_transaction_probe(
            &self.root_input,
            captured.is_some() || self.memory.drag_source().is_some(),
        );
        let mut plan = PointerTargetPlan::new(
            self.root_input.pointer.position,
            press_probe,
            drop_probe,
            retained_drag,
            self.root_input.events.clone(),
            self.spatial.clone(),
        );
        declare(&mut plan);
        let resolved = match plan.resolve(captured) {
            Ok(resolved) => resolved,
            Err(error) => {
                self.memory.cancel_pointer_interaction();
                self.memory
                    .install_pointer_routes(crate::PointerRoutes::BLOCKED, [], None, None);
                return Err(error);
            }
        };

        let selected_owner = match resolved.routes.ordinary {
            crate::PointerRoute::Target(owner) => Some(owner),
            crate::PointerRoute::Unplanned | crate::PointerRoute::Blocked => None,
        };
        let owner_mismatch = !resolved.capture_valid
            || [
                self.memory.active(),
                self.memory.pressed(),
                self.memory.secondary_pressed(),
                self.memory.drag_source(),
                self.memory.pointer_gesture_owner(),
            ]
            .into_iter()
            .flatten()
            .any(|owner| Some(owner) != selected_owner);
        let mut routes = resolved.routes;
        let (planned_drag_release, planned_drag_source) = if owner_mismatch {
            self.memory.cancel_pointer_interaction();
            routes.drop = crate::PointerRoute::Blocked;
            (None, None)
        } else {
            (resolved.planned_drag_release, resolved.planned_drag_source)
        };
        self.memory.install_pointer_routes(
            routes,
            resolved.cursor_equivalents,
            planned_drag_release,
            planned_drag_source,
        );
        Ok(routes)
    }

    /// Registers an externally derived widget ID as present and checks duplicates.
    pub fn register_id(&mut self, id: WidgetId) -> WidgetId {
        self.ids.register(id);
        id
    }

    /// Pushes a stable ID scope and returns the scope ID.
    pub fn push_id_scope(&mut self, key: impl Hash) -> WidgetId {
        self.ids.push(key)
    }

    /// Pops the current stable ID scope.
    pub fn pop_id_scope(&mut self) -> Option<WidgetId> {
        self.ids.pop()
    }

    /// Runs a closure inside a stable ID scope.
    pub fn scope<T>(&mut self, key: impl Hash, f: impl FnOnce(&mut Self) -> T) -> T {
        self.ids.push(key);
        let result = f(self);
        self.ids.pop();
        result
    }

    /// Returns the accumulated output so far.
    #[must_use]
    pub const fn output(&self) -> &FrameOutput {
        &self.output
    }

    /// Appends one render primitive and updates the matching spatial scope.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.spatial.observe_primitive(&primitive);
        self.refresh_scoped_input();
        self.output.push_primitive(primitive);
    }

    /// Appends render primitives in order.
    pub fn extend_primitives(&mut self, primitives: impl IntoIterator<Item = Primitive>) {
        for primitive in primitives {
            self.push_primitive(primitive);
        }
    }

    /// Sets the semantic root node.
    pub fn set_semantic_root(&mut self, root: WidgetId) {
        self.output.set_semantic_root(root);
    }

    /// Appends one semantic node in traversal order.
    pub fn push_semantic_node(&mut self, mut node: SemanticNode) {
        self.ids.mark_seen(node.id);
        if let Some(bounds) = self.spatial.project_semantic_rect(node.bounds) {
            node.bounds = bounds;
        } else {
            if self.memory.is_focused(node.id) {
                self.memory.clear_focus();
                self.output.request_repaint(RepaintRequest::NextFrame);
            }
            node.bounds = Rect::ZERO;
            node.focusable = false;
            node.state.focused = false;
            node.actions
                .retain(|action| action.kind != SemanticActionKind::Focus);
        }
        self.output.push_semantic_node(node);
    }

    /// Requests repaint scheduling.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.output.request_repaint(request);
    }

    /// Adds an action invocation to the frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.output.push_action(invocation);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.output.invoke_action(action_id, source, context);
    }

    /// Appends one platform request.
    ///
    /// Raw text-input requests remain subject to the focused Editable owner and
    /// tracked platform-state authority. Prefer [`Self::start_text_input`] or
    /// [`Self::publish_text_input_rect`] for new text widgets.
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::StartTextInput { rect: Some(rect) } => {
                let Some(owner) = self.focused_editable_text_owner() else {
                    return;
                };
                let Some(rect) = self.spatial.project_rect(rect) else {
                    return;
                };
                self.drain_pending_text_input_stop();
                self.memory.activate_platform_text_input(owner);
                self.output
                    .push_platform_request(PlatformRequest::StartTextInput { rect: Some(rect) });
            }
            PlatformRequest::StartTextInput { rect: None } => {
                let Some(owner) = self.focused_editable_text_owner() else {
                    return;
                };
                if !self.spatial.is_visible() {
                    return;
                }
                self.drain_pending_text_input_stop();
                self.memory.activate_platform_text_input(owner);
                self.output
                    .push_platform_request(PlatformRequest::StartTextInput { rect: None });
            }
            PlatformRequest::UpdateTextInputRect { rect } => {
                let Some(owner) = self.focused_editable_text_owner() else {
                    return;
                };
                if !self.memory.platform_text_input_is_active_for(owner) {
                    return;
                }
                let Some(rect) = self.spatial.project_rect(rect) else {
                    return;
                };
                self.output
                    .push_platform_request(PlatformRequest::UpdateTextInputRect { rect });
            }
            PlatformRequest::StopTextInput => {
                self.memory.acknowledge_platform_text_input_stop();
                self.output
                    .push_platform_request(PlatformRequest::StopTextInput);
            }
            request => self.output.push_platform_request(request),
        }
    }

    /// Requests a cursor shape for a hovered or captured widget.
    ///
    /// Captured pointer owners stay authoritative even when the pointer leaves
    /// their rect. Other widgets cannot publish cursor intent while capture is
    /// owned elsewhere, and cancelled pointer frames suppress stale cursor
    /// output until a later frame establishes fresh hover or capture state.
    pub fn request_cursor_for(&mut self, owner: WidgetId, cursor: CursorShape) -> bool {
        if self.memory.pointer_interaction_cancelled()
            || crate::interaction::canonical_pointer_fenced(&self.context.input)
            || self.context.input.pointer.position.is_none()
        {
            return false;
        }
        if let Some(captured) = self.memory.pointer_capture() {
            if !self.memory.cursor_owner_matches_capture(captured, owner) {
                return false;
            }
        } else if !self.memory.is_hovered(owner) {
            return false;
        }

        self.output.request_cursor(cursor);
        true
    }

    /// Starts platform text input for a focused text-editing widget.
    ///
    /// The rectangle is expressed in current-scope logical coordinates and is
    /// transformed and clipped to screen-logical coordinates for platform
    /// adapters. Unfocused or spatially invisible widgets cannot acquire text
    /// input ownership through this helper.
    pub fn start_text_input(&mut self, owner: WidgetId, rect: Option<Rect>) -> bool {
        self.ids.mark_seen(owner);
        if !self.memory.is_focused(owner) {
            return false;
        }

        let rect = match rect {
            Some(rect) => {
                let Some(rect) = self.spatial.project_rect(rect) else {
                    self.memory.clear_focus();
                    self.output.request_repaint(RepaintRequest::NextFrame);
                    return false;
                };
                Some(rect)
            }
            None if self.spatial.is_visible() => None,
            None => {
                self.memory.clear_focus();
                self.output.request_repaint(RepaintRequest::NextFrame);
                return false;
            }
        };

        let was_active = self.memory.platform_text_input_is_active_for(owner);
        self.memory
            .set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
        self.drain_pending_text_input_stop();
        if was_active && self.memory.platform_text_input_is_active_for(owner) {
            if let Some(rect) = rect {
                self.output
                    .push_platform_request(PlatformRequest::UpdateTextInputRect { rect });
            }
            return true;
        }
        self.memory.activate_platform_text_input(owner);
        self.output
            .push_platform_request(PlatformRequest::StartTextInput { rect });
        true
    }

    /// Marks an async owner present and returns its stable incarnation token.
    pub fn mark_present_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.memory.mark_present_target(target)
    }

    /// Marks an async owner present using the previous live terminology.
    #[deprecated(note = "use mark_present_target")]
    pub fn mark_live_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.mark_present_target(target)
    }

    /// Starts a replacement incarnation and marks it present.
    pub fn restart_liveness_target(
        &mut self,
        target: impl Into<LivenessTargetId>,
    ) -> LivenessToken {
        self.memory.restart_liveness_target(target)
    }

    /// Cancels the exact active token incarnation.
    pub fn cancel_liveness_token(&mut self, token: LivenessToken) -> LivenessUpdateStatus {
        self.memory.cancel_liveness_token(token)
    }

    /// Removes the target's active incarnation.
    pub fn remove_live_target(
        &mut self,
        target: impl Into<LivenessTargetId>,
    ) -> LivenessRemovalStatus {
        self.memory.remove_live_target(target)
    }

    /// Appends one runtime warning.
    pub fn push_warning(&mut self, warning: FrameWarning) {
        self.output.push_warning(warning);
    }

    /// Finishes the frame and returns deterministic output.
    #[must_use]
    pub fn end_frame(mut self) -> FrameOutput {
        for duplicate in self.ids.duplicates() {
            self.output
                .push_warning(FrameWarning::DuplicateWidgetId { id: duplicate.id });
        }

        if self.pointer_cancel_pending && self.memory.cancel_pointer_interaction() {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }

        let semantic_tree_valid = match self.output.semantics.validate() {
            Ok(()) => true,
            Err(error) => {
                self.output
                    .push_warning(FrameWarning::InvalidSemanticTree { error });
                false
            }
        };
        if semantic_tree_valid && apply_escape_text_blur(&self.root_input, self.memory) {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        if apply_window_focus_text_blur(&self.root_input, self.memory) {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        if semantic_tree_valid
            && apply_pointer_text_owner_blur(&self.root_input, self.memory, &self.output.semantics)
        {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        if semantic_tree_valid
            && apply_keyboard_focus_traversal(&self.root_input, self.memory, &self.output.semantics)
        {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        let ids = &self.ids;
        if self
            .memory
            .reconcile_widget_owners(|owner| ids.was_seen(owner))
        {
            self.output.request_repaint(RepaintRequest::NextFrame);
        }
        if self.memory.take_pending_text_input_stop().is_some() {
            self.output
                .push_platform_request(PlatformRequest::StopTextInput);
        }

        self.memory.end_frame();
        let warnings = validate_primitive_stack(&self.output.primitives);
        self.output.warnings.extend(warnings);
        self.output
    }

    fn drain_pending_text_input_stop(&mut self) {
        if self.memory.take_pending_text_input_stop().is_some() {
            self.output
                .push_platform_request(PlatformRequest::StopTextInput);
        }
    }

    fn focused_editable_text_owner(&self) -> Option<WidgetId> {
        let owner = self.memory.text_input_owner()?;
        (self.memory.is_focused(owner)
            && self.memory.text_input_owner_mode() == Some(TextInputOwnerMode::Editable))
        .then_some(owner)
    }

    fn refresh_scoped_input(&mut self) {
        let localized = self.spatial.localize_input(
            &self.root_input,
            self.memory.pointer_release_cleanup_required(),
            self.memory.secondary_pressed().is_some(),
            self.memory.root_input_conflict().is_some(),
        );
        let scoped_primary_transaction_open = localized
            .event_ordinals
            .iter()
            .map(|ordinal| self.root_primary_transaction_open[*ordinal])
            .collect::<Vec<_>>();
        self.memory.install_scoped_pointer_events(
            localized.event_ordinals.iter().copied(),
            localized
                .cleanup_only
                .iter()
                .enumerate()
                .filter_map(|(index, cleanup_only)| cleanup_only.then_some(index)),
            scoped_primary_transaction_open,
        );
        self.context.input = localized.input;
        self.input_event_ordinals = localized.event_ordinals;
    }
}

fn pointer_release_all_cancelled(input: &UiInput) -> bool {
    input.pointer.release_all_cancelled()
}

fn primary_transaction_open_before_events(
    input: &UiInput,
    mut transaction_open: bool,
) -> Vec<bool> {
    input
        .events
        .iter()
        .map(|event| {
            let was_open = transaction_open;
            match event {
                UiInputEvent::PointerButton {
                    button: MouseButton::Primary,
                    down,
                    ..
                } => transaction_open = *down,
                UiInputEvent::PointerReleaseAll { .. }
                | UiInputEvent::WindowFocusChanged(false) => transaction_open = false,
                _ => {}
            }
            was_open
        })
        .collect()
}

struct ModifierFold {
    by_ordinal: Vec<Modifiers>,
    retained: Modifiers,
    suspended: bool,
}

fn fold_root_event_modifiers(
    input: &UiInput,
    conflicted: bool,
    entry: Modifiers,
    suspended: bool,
) -> ModifierFold {
    if input.events.is_empty() {
        return if input.window_focused {
            ModifierFold {
                by_ordinal: Vec::new(),
                retained: input.keyboard.modifiers,
                suspended: false,
            }
        } else {
            ModifierFold {
                by_ordinal: Vec::new(),
                retained: Modifiers::default(),
                suspended: true,
            }
        };
    }

    let mut current = entry;
    let mut tracking = !suspended;
    let mut by_ordinal = Vec::with_capacity(input.events.len());
    for event in &input.events {
        match event {
            UiInputEvent::WindowFocusChanged(false) => {
                by_ordinal.push(current);
                current = Modifiers::default();
                tracking = false;
            }
            UiInputEvent::WindowFocusChanged(true) => {
                by_ordinal.push(current);
                if !conflicted {
                    tracking = true;
                }
            }
            UiInputEvent::ModifiersChanged(modifiers) if tracking && !conflicted => {
                current = *modifiers;
                by_ordinal.push(current);
            }
            UiInputEvent::Key(event) if tracking && !conflicted => {
                current = event.modifiers;
                by_ordinal.push(current);
            }
            _ => by_ordinal.push(current),
        }
    }
    ModifierFold {
        by_ordinal,
        retained: current,
        suspended: !tracking,
    }
}

fn pointer_transaction_probe(
    input: &UiInput,
    retained_transaction: bool,
) -> (Option<PointerPressProbe>, PointerDropProbe) {
    if input.events.is_empty() {
        return (None, PointerDropProbe::Snapshot);
    }
    let mut transaction_started = retained_transaction;
    let mut press_probe = None;
    for (ordinal, event) in input.events.iter().enumerate() {
        match event {
            UiInputEvent::PointerButton {
                button: crate::MouseButton::Primary,
                down: true,
                position,
                ..
            } if !transaction_started => {
                transaction_started = true;
                press_probe = Some(PointerPressProbe {
                    ordinal,
                    position: *position,
                });
            }
            UiInputEvent::PointerButton {
                button: crate::MouseButton::Primary,
                down: false,
                position,
                ..
            } if transaction_started => {
                return (
                    press_probe,
                    PointerDropProbe::Release {
                        ordinal,
                        position: *position,
                    },
                );
            }
            UiInputEvent::PointerReleaseAll { .. } | UiInputEvent::WindowFocusChanged(false) => {
                return (press_probe, PointerDropProbe::Cancelled);
            }
            _ => {}
        }
    }
    (press_probe, PointerDropProbe::Snapshot)
}

fn is_editing_domain_event(event: &UiInputEvent) -> bool {
    matches!(
        event,
        UiInputEvent::ModifiersChanged(_)
            | UiInputEvent::Key(_)
            | UiInputEvent::Text(_)
            | UiInputEvent::ClipboardText(_)
            | UiInputEvent::ImeEnabled(_)
            | UiInputEvent::WindowFocusChanged(_)
    )
}
