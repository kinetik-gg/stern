use std::hash::Hash;

use crate::input::UiInput;
use crate::memory::UiMemory;
use crate::render::Primitive;
use crate::{
    ActionContext, ActionId, ActionInvocation, ActionSource, IdStack, LivenessTargetId,
    LivenessToken, Rect, SemanticActionKind, SemanticNode, WidgetId,
};

use super::focus::{
    apply_escape_text_blur, apply_keyboard_focus_traversal, apply_pointer_text_owner_blur,
    apply_window_focus_text_blur,
};
use super::output::FrameOutput;
use super::pointer::{PointerPlanError, PointerTargetPlan};
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
    memory: &'a mut UiMemory,
    ids: IdStack,
    output: FrameOutput,
    spatial: SpatialStack,
    pointer_plan_installed: bool,
}

impl<'a> Ui<'a> {
    /// Starts a UI frame and clears transient retained memory.
    #[must_use]
    pub fn begin_frame(context: FrameContext, memory: &'a mut UiMemory) -> Self {
        memory.begin_frame();
        if !context.input.window_focused || pointer_release_all_cancelled(&context.input) {
            memory.cancel_pointer_interaction();
        }
        let root_input = context.input.clone();
        Self {
            context,
            root_input,
            memory,
            ids: IdStack::new(),
            output: FrameOutput::new(),
            spatial: SpatialStack::default(),
            pointer_plan_installed: false,
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
                .install_pointer_routes(crate::PointerRoutes::BLOCKED, []);
            return Err(PointerPlanError::AlreadyInstalled);
        }
        self.pointer_plan_installed = true;

        let captured = self.memory.pointer_capture();
        let mut plan =
            PointerTargetPlan::new(self.root_input.pointer.position, self.spatial.clone());
        declare(&mut plan);
        let resolved = match plan.resolve(captured) {
            Ok(resolved) => resolved,
            Err(error) => {
                self.memory.cancel_pointer_interaction();
                self.memory
                    .install_pointer_routes(crate::PointerRoutes::BLOCKED, []);
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
            ]
            .into_iter()
            .flatten()
            .any(|owner| Some(owner) != selected_owner);
        if owner_mismatch {
            self.memory.cancel_pointer_interaction();
        }
        self.memory
            .install_pointer_routes(resolved.routes, resolved.cursor_equivalents);
        Ok(resolved.routes)
    }

    /// Registers an externally derived widget ID for duplicate detection.
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
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        match request {
            PlatformRequest::StartTextInput { rect: Some(rect) } => {
                if let Some(rect) = self.spatial.project_rect(rect) {
                    self.output
                        .push_platform_request(PlatformRequest::StartTextInput {
                            rect: Some(rect),
                        });
                }
            }
            PlatformRequest::StartTextInput { rect: None } if !self.spatial.is_visible() => {}
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

        let previous_owner = self.memory.text_input_owner();
        if previous_owner == Some(owner) {
            return true;
        }
        let stopped_owner = self.memory.take_pending_text_input_stop();
        self.memory.set_text_input_owner(owner);
        if stopped_owner.is_some_and(|stopped| stopped != owner) {
            self.output
                .push_platform_request(PlatformRequest::StopTextInput);
        }
        self.output
            .push_platform_request(PlatformRequest::StartTextInput { rect });
        true
    }

    /// Marks a target live for deterministic external update validation.
    ///
    /// Calling this again for the same target renews its generation, making
    /// older tokens for that target stale.
    pub fn mark_live_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.memory.mark_live_target(target)
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
        if self.memory.take_pending_text_input_stop().is_some() {
            self.output
                .push_platform_request(PlatformRequest::StopTextInput);
        }

        self.memory.end_frame();
        let warnings = validate_primitive_stack(&self.output.primitives);
        self.output.warnings.extend(warnings);
        self.output
    }

    fn refresh_scoped_input(&mut self) {
        self.context.input = self.spatial.localize_input(
            &self.root_input,
            self.memory.pointer_capture().is_some(),
            self.memory.secondary_pressed().is_some(),
        );
    }
}

fn pointer_release_all_cancelled(input: &UiInput) -> bool {
    input.pointer.release_all_cancelled()
}
