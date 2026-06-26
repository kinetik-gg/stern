//! Data-only modal dialog contract conformance tests.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, Point, Rect, WidgetId,
};
use kinetik_ui_widgets::{
    ModalAction, ModalActionRole, ModalCloseReason, ModalDialog, ModalDialogOverlay,
    ModalFocusContainment, OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack,
};

fn overlay_id(raw: u64) -> OverlayId {
    OverlayId::from_raw(raw)
}

fn widget(key: &str) -> WidgetId {
    WidgetId::from_key(key)
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn modal_action(id: &str, role: ModalActionRole) -> ModalAction {
    ModalAction::new(action(id, id), role)
}

fn hidden_action(id: &str, role: ModalActionRole) -> ModalAction {
    let mut descriptor = action(id, id);
    descriptor.state.visible = false;
    ModalAction::new(descriptor, role)
}

fn disabled_action(id: &str, role: ModalActionRole) -> ModalAction {
    let mut descriptor = action(id, id);
    descriptor.state.enabled = false;
    ModalAction::new(descriptor, role)
}

fn dialog_with_actions(actions: impl IntoIterator<Item = ModalAction>) -> ModalDialog {
    ModalDialog::new(widget("dialog"), "Confirm").with_actions(actions)
}

fn overlay_with_dismissal(dismissal: OverlayDismissal) -> ModalDialogOverlay {
    let dialog = ModalDialog::new(widget("dialog"), "Confirm").with_focus(
        ModalFocusContainment::from_targets([widget("confirm")])
            .with_initial_focus(widget("confirm"))
            .with_return_focus(widget("launcher")),
    );
    ModalDialogOverlay::placed(
        overlay_id(1),
        Rect::new(40.0, 40.0, 120.0, 80.0),
        dialog,
        dismissal,
        ActionContext::Modal(widget("dialog")),
    )
}

#[test]
fn modal_dialog_conformance_opens_as_modal_overlay_entry() {
    let overlay = ModalDialogOverlay::new(
        OverlayEntry::new(
            overlay_id(7),
            OverlayKind::Popover,
            Rect::new(20.0, 30.0, 320.0, 180.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        ModalDialog::new(widget("dialog"), "Export").with_body("Export selected frames"),
        ActionContext::Modal(widget("dialog")),
    );
    let mut stack = OverlayStack::new();

    overlay.open_in(&mut stack);

    let entry = stack.top().expect("modal entry");
    assert_eq!(entry.id, overlay_id(7));
    assert_eq!(entry.kind, OverlayKind::Modal);
    assert!(entry.modal);
    assert_eq!(entry.dismissal, OverlayDismissal::OutsideClickOrEscape);
    assert_eq!(stack.focus_target(), Some(overlay_id(7)));
    assert_eq!(overlay.dialog.title, "Export");
    assert_eq!(
        overlay.dialog.body.as_ref().map(|body| body.text.as_str()),
        Some("Export selected frames")
    );
}

#[test]
fn modal_dialog_conformance_focus_metadata_is_ordered_and_contained() {
    let first = widget("first-field");
    let second = widget("second-field");
    let unknown = widget("missing-field");
    let launcher = widget("open-button");

    let mut focus = ModalFocusContainment::from_targets([first, second, first]);

    assert!(focus.set_initial_focus(second));
    assert!(!focus.set_initial_focus(unknown));
    focus = focus.with_return_focus(launcher);

    assert_eq!(focus.initial_focus(), Some(second));
    assert_eq!(focus.return_focus(), Some(launcher));
    assert_eq!(focus.contained_targets(), &[first, second]);
    assert!(!focus.contains_target(unknown));
}

#[test]
fn modal_dialog_conformance_empty_modal_has_no_focus_or_invokable_action() {
    let dialog = ModalDialog::new(widget("empty-dialog"), "Empty");
    let mut queue = ActionQueue::new();

    assert_eq!(dialog.focus.initial_focus(), None);
    assert!(dialog.visible_actions().is_empty());
    assert_eq!(
        dialog.invocation_for_visible(0, ActionContext::Modal(widget("empty-dialog"))),
        None
    );
    assert!(!dialog.invoke_role(
        ModalActionRole::Primary,
        &mut queue,
        ActionContext::Modal(widget("empty-dialog"))
    ));
    assert!(queue.is_empty());
}

#[test]
fn modal_dialog_conformance_visible_actions_filter_hidden_and_preserve_disabled() {
    let dialog = dialog_with_actions([
        modal_action("confirm", ModalActionRole::Primary),
        hidden_action("secret", ModalActionRole::Secondary),
        disabled_action("delete", ModalActionRole::Destructive),
        modal_action("cancel", ModalActionRole::Cancel),
    ]);

    let visible = dialog.visible_actions();

    assert_eq!(visible.len(), 3);
    assert_eq!(visible[0].action_id(), &ActionId::new("confirm"));
    assert_eq!(visible[1].action_id(), &ActionId::new("delete"));
    assert_eq!(visible[1].role, ModalActionRole::Destructive);
    assert!(!visible[1].can_invoke());
    assert_eq!(visible[2].action_id(), &ActionId::new("cancel"));
}

#[test]
fn modal_dialog_conformance_invocations_emit_metadata_without_executing_actions() {
    let context = ActionContext::Modal(widget("dialog"));
    let dialog = dialog_with_actions([
        modal_action("confirm", ModalActionRole::Primary),
        disabled_action("disabled", ModalActionRole::Secondary),
        hidden_action("hidden", ModalActionRole::Destructive),
    ]);
    let overlay = ModalDialogOverlay::placed(
        overlay_id(8),
        Rect::new(0.0, 0.0, 200.0, 120.0),
        dialog,
        OverlayDismissal::Manual,
        context.clone(),
    );
    let mut queue = ActionQueue::new();

    assert!(overlay.invoke_visible(0, &mut queue));
    assert!(!overlay.invoke_visible(1, &mut queue));
    assert_eq!(
        overlay.invocation_for_role(ModalActionRole::Destructive),
        None
    );

    let invocation = queue.pop_front().expect("modal action invocation");
    assert_eq!(invocation.action_id, ActionId::new("confirm"));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, context);
    assert!(queue.is_empty());
}

#[test]
fn modal_dialog_conformance_cancel_role_is_discoverable_and_gated() {
    let enabled_cancel = dialog_with_actions([
        modal_action("confirm", ModalActionRole::Primary),
        modal_action("cancel", ModalActionRole::Cancel),
    ]);
    let disabled_cancel = dialog_with_actions([disabled_action("cancel", ModalActionRole::Cancel)]);
    let hidden_cancel = dialog_with_actions([hidden_action("cancel", ModalActionRole::Cancel)]);

    assert_eq!(
        enabled_cancel.cancel_action().map(ModalAction::action_id),
        Some(&ActionId::new("cancel"))
    );
    assert!(
        enabled_cancel
            .invocation_for_role(ModalActionRole::Cancel, enabled_cancel.action_context())
            .is_some()
    );
    assert!(
        disabled_cancel
            .visible_action_by_role(ModalActionRole::Cancel)
            .is_some()
    );
    assert!(
        disabled_cancel
            .invocation_for_role(ModalActionRole::Cancel, disabled_cancel.action_context())
            .is_none()
    );
    assert!(hidden_cancel.cancel_action().is_none());
}

#[test]
fn modal_dialog_conformance_dismissal_policy_returns_close_metadata_only() {
    for (dismissal, outside_expected, escape_expected) in [
        (OverlayDismissal::Manual, None, None),
        (
            OverlayDismissal::OutsideClick,
            Some(ModalCloseReason::OutsideClick),
            None,
        ),
        (
            OverlayDismissal::Escape,
            None,
            Some(ModalCloseReason::Escape),
        ),
        (
            OverlayDismissal::OutsideClickOrEscape,
            Some(ModalCloseReason::OutsideClick),
            Some(ModalCloseReason::Escape),
        ),
    ] {
        let overlay = overlay_with_dismissal(dismissal);
        let mut stack = OverlayStack::new();
        overlay.open_in(&mut stack);

        let outside = overlay.dismissal_request(&stack, Some(Point::new(5.0, 5.0)), false);
        let escape = overlay.dismissal_request(&stack, None, true);

        assert_eq!(outside.map(|request| request.reason), outside_expected);
        assert_eq!(escape.map(|request| request.reason), escape_expected);
        assert_eq!(stack.entries().len(), 1);

        if let Some(request) = outside.or(escape) {
            assert_eq!(request.overlay_id, overlay_id(1));
            assert_eq!(request.dialog_id, widget("dialog"));
            assert_eq!(request.focus_return, Some(widget("launcher")));
        }
    }
}

#[test]
fn modal_dialog_conformance_outside_dismissal_wins_when_both_inputs_request_close() {
    let overlay = overlay_with_dismissal(OverlayDismissal::OutsideClickOrEscape);
    let mut stack = OverlayStack::new();
    overlay.open_in(&mut stack);

    let request = overlay
        .dismissal_request(&stack, Some(Point::new(5.0, 5.0)), true)
        .expect("combined dismissal request");

    assert_eq!(request.reason, ModalCloseReason::OutsideClick);
    assert_eq!(stack.entries().len(), 1);
}
