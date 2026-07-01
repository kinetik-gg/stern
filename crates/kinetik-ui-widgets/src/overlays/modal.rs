use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource, Point,
    Rect, WidgetId,
};

use super::{OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack};
/// Ordered focus containment metadata for a modal dialog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModalFocusContainment {
    initial_focus: Option<WidgetId>,
    return_focus: Option<WidgetId>,
    contained_targets: Vec<WidgetId>,
}

impl ModalFocusContainment {
    /// Creates empty modal focus metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates modal focus metadata from known contained focus targets.
    #[must_use]
    pub fn from_targets(targets: impl IntoIterator<Item = WidgetId>) -> Self {
        Self {
            initial_focus: None,
            return_focus: None,
            contained_targets: collect_unique_focus_targets(targets),
        }
    }

    /// Returns the preferred initial focus target when it is contained by the modal.
    #[must_use]
    pub const fn initial_focus(&self) -> Option<WidgetId> {
        self.initial_focus
    }

    /// Sets the preferred initial focus target when it is contained by the modal.
    ///
    /// Returns `false` and leaves the current initial focus unchanged when the
    /// target is unknown to this containment set.
    pub fn set_initial_focus(&mut self, target: WidgetId) -> bool {
        if !self.contains_target(target) {
            return false;
        }
        self.initial_focus = Some(target);
        true
    }

    /// Returns this focus metadata with the preferred initial focus target set.
    #[must_use]
    pub fn with_initial_focus(mut self, target: WidgetId) -> Self {
        self.set_initial_focus(target);
        self
    }

    /// Returns the focus target that should regain focus when the modal closes.
    #[must_use]
    pub const fn return_focus(&self) -> Option<WidgetId> {
        self.return_focus
    }

    /// Returns this focus metadata with close-time focus return metadata set.
    ///
    /// The return target may live outside the modal and is therefore not added
    /// to the contained target list.
    #[must_use]
    pub const fn with_return_focus(mut self, target: WidgetId) -> Self {
        self.return_focus = Some(target);
        self
    }

    /// Returns contained focus targets in deterministic traversal order.
    #[must_use]
    pub fn contained_targets(&self) -> &[WidgetId] {
        &self.contained_targets
    }

    /// Returns true when the target is part of the modal containment set.
    #[must_use]
    pub fn contains_target(&self, target: WidgetId) -> bool {
        self.contained_targets.contains(&target)
    }
}

/// Presentation metadata for a modal dialog body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalDialogBody {
    /// Plain body text or summary owned by the application.
    pub text: String,
}

impl ModalDialogBody {
    /// Creates body presentation metadata.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Semantic role of an action-backed modal dialog button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModalActionRole {
    /// Preferred accepting action.
    Primary,
    /// Non-primary supporting action.
    Secondary,
    /// Cancels or backs out of the dialog.
    Cancel,
    /// Potentially destructive accepting action.
    Destructive,
}

/// Action-backed modal button metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalAction {
    /// Application-owned action descriptor.
    pub action: ActionDescriptor,
    /// Modal-specific presentation role for the action button.
    pub role: ModalActionRole,
}

impl ModalAction {
    /// Creates modal action metadata.
    #[must_use]
    pub const fn new(action: ActionDescriptor, role: ModalActionRole) -> Self {
        Self { action, role }
    }

    /// Returns the backing action ID.
    #[must_use]
    pub const fn action_id(&self) -> &ActionId {
        &self.action.id
    }

    /// Returns true when the action should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the visible action can be invoked.
    #[must_use]
    pub const fn can_invoke(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates an invocation for this modal action when it is visible and enabled.
    #[must_use]
    pub fn invocation(&self, context: ActionContext) -> Option<ActionInvocation> {
        self.can_invoke()
            .then(|| ActionInvocation::new(self.action.id.clone(), ActionSource::Button, context))
    }
}

impl From<(ActionDescriptor, ModalActionRole)> for ModalAction {
    fn from((action, role): (ActionDescriptor, ModalActionRole)) -> Self {
        Self::new(action, role)
    }
}

/// Data-only modal dialog model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalDialog {
    /// Stable dialog widget identity.
    pub id: WidgetId,
    /// Short title presented by the dialog surface.
    pub title: String,
    /// Optional body presentation metadata.
    pub body: Option<ModalDialogBody>,
    /// Focus containment and focus-return metadata.
    pub focus: ModalFocusContainment,
    actions: Vec<ModalAction>,
}

impl ModalDialog {
    /// Creates an empty modal dialog model.
    #[must_use]
    pub fn new(id: WidgetId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            body: None,
            focus: ModalFocusContainment::new(),
            actions: Vec::new(),
        }
    }

    /// Returns this dialog with body presentation metadata.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(ModalDialogBody::new(body));
        self
    }

    /// Returns this dialog with focus containment metadata.
    #[must_use]
    pub fn with_focus(mut self, focus: ModalFocusContainment) -> Self {
        self.focus = focus;
        self
    }

    /// Returns this dialog with ordered modal action metadata.
    #[must_use]
    pub fn with_actions(mut self, actions: impl IntoIterator<Item = ModalAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Adds modal action metadata.
    pub fn push_action(&mut self, action: ModalAction) {
        self.actions.push(action);
    }

    /// Returns modal actions in presentation order.
    #[must_use]
    pub fn actions(&self) -> &[ModalAction] {
        &self.actions
    }

    /// Returns visible modal actions in presentation order.
    #[must_use]
    pub fn visible_actions(&self) -> Vec<&ModalAction> {
        self.visible_actions_iter().collect()
    }

    /// Returns visible modal actions as a borrowed iterator.
    pub fn visible_actions_iter(&self) -> impl Iterator<Item = &ModalAction> + '_ {
        self.actions.iter().filter(|action| action.visible())
    }

    /// Returns the first visible modal action with the requested role.
    #[must_use]
    pub fn visible_action_by_role(&self, role: ModalActionRole) -> Option<&ModalAction> {
        self.visible_actions_iter()
            .find(|action| action.role == role)
    }

    /// Returns the first visible cancel action, if any.
    #[must_use]
    pub fn cancel_action(&self) -> Option<&ModalAction> {
        self.visible_action_by_role(ModalActionRole::Cancel)
    }

    /// Returns the default modal action context for this dialog identity.
    #[must_use]
    pub const fn action_context(&self) -> ActionContext {
        ActionContext::Modal(self.id)
    }

    /// Creates an invocation for an enabled visible modal action by visible index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_actions_iter()
            .nth(visible_index)
            .and_then(|action| action.invocation(context))
    }

    /// Creates an invocation for the first visible enabled action with the requested role.
    #[must_use]
    pub fn invocation_for_role(
        &self,
        role: ModalActionRole,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_action_by_role(role)
            .and_then(|action| action.invocation(context))
    }

    /// Invokes an enabled visible modal action by visible index into an action queue.
    pub fn invoke_visible(
        &self,
        visible_index: usize,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) = self.invocation_for_visible(visible_index, context) else {
            return false;
        };
        queue.push(invocation);
        true
    }

    /// Invokes the first visible enabled action with the requested role into an action queue.
    pub fn invoke_role(
        &self,
        role: ModalActionRole,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) = self.invocation_for_role(role, context) else {
            return false;
        };
        queue.push(invocation);
        true
    }
}

/// Reason a modal dialog close was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalCloseReason {
    /// The modal requested close because the pointer activated outside the modal.
    OutsideClick,
    /// The modal requested close because Escape was pressed.
    Escape,
    /// The modal was closed directly by application-owned state.
    Programmatic,
}

/// Deterministic modal close request metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalCloseRequest {
    /// Overlay identity that should close.
    pub overlay_id: OverlayId,
    /// Stable dialog widget identity.
    pub dialog_id: WidgetId,
    /// Reason the modal requested close.
    pub reason: ModalCloseReason,
    /// Focus target that should regain focus after close, when known.
    pub focus_return: Option<WidgetId>,
}

impl ModalCloseRequest {
    /// Creates modal close request metadata.
    #[must_use]
    pub const fn new(
        overlay_id: OverlayId,
        dialog_id: WidgetId,
        reason: ModalCloseReason,
        focus_return: Option<WidgetId>,
    ) -> Self {
        Self {
            overlay_id,
            dialog_id,
            reason,
            focus_return,
        }
    }
}

/// Data-only modal dialog overlay descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ModalDialogOverlay {
    /// Overlay stack entry for placement, z-order, focus, and dismissal.
    pub entry: OverlayEntry,
    /// Dialog presentation, focus, and action metadata.
    pub dialog: ModalDialog,
    /// Context captured for action invocations emitted by this dialog surface.
    pub context: ActionContext,
}

impl ModalDialogOverlay {
    /// Creates a modal dialog overlay from an existing stack entry and dialog model.
    ///
    /// The entry is normalized to `OverlayKind::Modal` with modal capture so it
    /// can safely compose with `OverlayStack`.
    #[must_use]
    pub fn new(entry: OverlayEntry, dialog: ModalDialog, context: ActionContext) -> Self {
        let mut entry = entry;
        entry.kind = OverlayKind::Modal;
        entry.modal = true;
        Self {
            entry,
            dialog,
            context,
        }
    }

    /// Creates a modal dialog overlay from a target rectangle.
    #[must_use]
    pub fn placed(
        id: OverlayId,
        rect: Rect,
        dialog: ModalDialog,
        dismissal: OverlayDismissal,
        context: ActionContext,
    ) -> Self {
        Self::new(
            OverlayEntry::new(id, OverlayKind::Modal, rect).dismiss_on(dismissal),
            dialog,
            context,
        )
    }

    /// Opens this modal at the top of an overlay stack.
    pub fn open_in(&self, stack: &mut OverlayStack) {
        stack.open(self.entry.clone());
    }

    /// Creates a programmatic close request for this modal.
    #[must_use]
    pub const fn close_request(&self, reason: ModalCloseReason) -> ModalCloseRequest {
        ModalCloseRequest::new(
            self.entry.id,
            self.dialog.id,
            reason,
            self.dialog.focus.return_focus(),
        )
    }

    /// Applies existing overlay-stack dismissal rules and returns modal close metadata.
    #[must_use]
    pub fn dismissal_request(
        &self,
        stack: &OverlayStack,
        outside_activation: Option<Point>,
        escape_pressed: bool,
    ) -> Option<ModalCloseRequest> {
        if outside_activation.is_some_and(|point| {
            stack
                .outside_click_close_requests(point)
                .contains(&self.entry.id)
        }) {
            return Some(self.close_request(ModalCloseReason::OutsideClick));
        }

        if escape_pressed && stack.escape_close_request() == Some(self.entry.id) {
            return Some(self.close_request(ModalCloseReason::Escape));
        }

        None
    }

    /// Returns visible modal actions in presentation order.
    #[must_use]
    pub fn visible_actions(&self) -> Vec<&ModalAction> {
        self.dialog.visible_actions()
    }

    /// Returns visible modal actions as a borrowed iterator.
    pub fn visible_actions_iter(&self) -> impl Iterator<Item = &ModalAction> + '_ {
        self.dialog.visible_actions_iter()
    }

    /// Returns the first visible modal action with the requested role.
    #[must_use]
    pub fn visible_action_by_role(&self, role: ModalActionRole) -> Option<&ModalAction> {
        self.dialog.visible_action_by_role(role)
    }

    /// Creates an invocation for an enabled visible modal action by visible index.
    #[must_use]
    pub fn invocation_for_visible(&self, visible_index: usize) -> Option<ActionInvocation> {
        self.dialog
            .invocation_for_visible(visible_index, self.context.clone())
    }

    /// Creates an invocation for the first visible enabled action with the requested role.
    #[must_use]
    pub fn invocation_for_role(&self, role: ModalActionRole) -> Option<ActionInvocation> {
        self.dialog.invocation_for_role(role, self.context.clone())
    }

    /// Invokes an enabled visible modal action by visible index into an action queue.
    pub fn invoke_visible(&self, visible_index: usize, queue: &mut ActionQueue) -> bool {
        let Some(invocation) = self.invocation_for_visible(visible_index) else {
            return false;
        };
        queue.push(invocation);
        true
    }

    /// Invokes the first visible enabled action with the requested role into an action queue.
    pub fn invoke_role(&self, role: ModalActionRole, queue: &mut ActionQueue) -> bool {
        let Some(invocation) = self.invocation_for_role(role) else {
            return false;
        };
        queue.push(invocation);
        true
    }
}

fn collect_unique_focus_targets(targets: impl IntoIterator<Item = WidgetId>) -> Vec<WidgetId> {
    let mut unique = Vec::new();
    for target in targets {
        if !unique.contains(&target) {
            unique.push(target);
        }
    }
    unique
}
