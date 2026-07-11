//! Deterministic observer subscription conformance.

use kinetik_ui_core::{
    HarnessPhase, LivenessTargetId, ObserverDeliverySkipReason, ObserverDeliveryStatus,
    ObserverDrain, ObserverNotificationId, RepaintRequest, SettlePendingCause, UiMemory,
    UiTestHarness, WidgetId,
};

fn notification(value: u64) -> ObserverNotificationId {
    ObserverNotificationId::new(value)
}

fn delivered_notifications(drain: &ObserverDrain) -> Vec<ObserverNotificationId> {
    drain
        .statuses()
        .iter()
        .filter_map(|status| match *status {
            ObserverDeliveryStatus::Delivered(delivery) => Some(delivery.notification_id()),
            ObserverDeliveryStatus::Skipped(_) => None,
        })
        .collect()
}

fn skipped_reasons(drain: &ObserverDrain) -> Vec<ObserverDeliverySkipReason> {
    drain
        .statuses()
        .iter()
        .filter_map(|status| match *status {
            ObserverDeliveryStatus::Delivered(_) => None,
            ObserverDeliveryStatus::Skipped(skipped) => Some(skipped.reason()),
        })
        .collect()
}

#[test]
fn live_subscription_receives_fifo_notifications_during_explicit_drain() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("preview");
    let (handle, _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        ui.memory_mut().subscribe_observer(token)
    });
    let subscription_id = handle.id();

    for id in 1..=3 {
        harness
            .memory_mut()
            .publish_observer(subscription_id, notification(id));
    }

    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });

    assert_eq!(
        delivered,
        vec![notification(1), notification(2), notification(3)]
    );
    assert_eq!(delivered_notifications(&drain), delivered);
    assert!(skipped_reasons(&drain).is_empty());
    assert_eq!(harness.memory().observers().queued_notification_count(), 0);
    assert!(handle.is_active());
}

#[test]
fn one_subscription_stays_valid_for_one_thousand_present_frames_without_refresh() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("long-job");
    let ((handle, token), _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });

    for frame in 1..1_000 {
        let (current, _) = harness.run_frame(|ui| ui.mark_present_target(target));
        assert_eq!(current, token, "frame {frame}");
    }

    harness
        .memory_mut()
        .publish_observer(handle.id(), notification(1));
    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
        assert_eq!(delivery.token(), token);
    });
    assert_eq!(delivered, vec![notification(1)]);
    assert_eq!(delivered_notifications(&drain), delivered);
    assert!(handle.is_active());
}

#[test]
fn dropped_and_unsubscribed_subscriptions_skip_before_liveness_validation() {
    let mut harness = UiTestHarness::new();
    let first_target = WidgetId::from_key("first");
    let second_target = WidgetId::from_key("second");

    let ((dropped_id, unsubscribed_id), _) = harness.run_frame(|ui| {
        let dropped_token = ui.mark_present_target(first_target);
        let dropped = ui.memory_mut().subscribe_observer(dropped_token);
        let dropped_id = dropped.id();
        drop(dropped);

        let unsubscribed_token = ui.mark_present_target(second_target);
        let unsubscribed = ui.memory_mut().subscribe_observer(unsubscribed_token);
        let unsubscribed_id = unsubscribed.id();
        assert!(ui.memory_mut().unsubscribe_observer(unsubscribed_id));

        (dropped_id, unsubscribed_id)
    });

    harness
        .memory_mut()
        .publish_observer(dropped_id, notification(1));
    harness
        .memory_mut()
        .publish_observer(unsubscribed_id, notification(2));
    let drain = harness
        .memory_mut()
        .drain_observers(|_, _| panic!("inactive subscriptions must not deliver"));

    assert_eq!(
        skipped_reasons(&drain),
        vec![
            ObserverDeliverySkipReason::Unsubscribed,
            ObserverDeliverySkipReason::Unsubscribed,
        ]
    );
}

#[test]
fn stale_or_missing_target_skips_delivery_without_mutating() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("thumbnail");
    let (handle, _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        ui.memory_mut().subscribe_observer(token)
    });

    let _ = harness.run_frame(|_| ());
    assert!(!harness.memory().liveness().is_active(target));
    harness
        .memory_mut()
        .publish_observer(handle.id(), notification(1));

    let drain = harness
        .memory_mut()
        .drain_observers(|_, _| panic!("stale subscription must not deliver"));
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::StaleTarget {
            target: LivenessTargetId::new(target),
        }]
    );
}

#[test]
fn restart_skips_old_subscription_and_delivers_new_subscription() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("reused-target");
    let ((old_handle, old_token), _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });
    let ((new_handle, new_token), _) = harness.run_frame(|ui| {
        let token = ui.restart_liveness_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });

    harness
        .memory_mut()
        .publish_observer(old_handle.id(), notification(1));
    harness
        .memory_mut()
        .publish_observer(new_handle.id(), notification(2));

    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });

    assert_eq!(delivered, vec![notification(2)]);
    assert_eq!(delivered_notifications(&drain), delivered);
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::StaleIncarnation {
            target: LivenessTargetId::new(target),
            token_incarnation: old_token.incarnation(),
            current_incarnation: new_token.incarnation(),
        }]
    );
}

#[test]
fn queue_then_cancel_skips_but_drain_then_cancel_may_deliver_once() {
    let target = WidgetId::from_key("cancel-order");

    let mut queue_first = UiTestHarness::new();
    let ((handle, token), _) = queue_first.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });
    queue_first
        .memory_mut()
        .publish_observer(handle.id(), notification(1));
    queue_first.memory_mut().cancel_liveness_token(token);
    let drain = queue_first
        .memory_mut()
        .drain_observers(|_, _| panic!("cancelled notification must not deliver"));
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::Cancelled {
            target: LivenessTargetId::new(target),
            incarnation: token.incarnation(),
        }]
    );

    let mut drain_first = UiTestHarness::new();
    let ((handle, token), _) = drain_first.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });
    drain_first
        .memory_mut()
        .publish_observer(handle.id(), notification(1));
    let mut delivered = 0;
    let drain = drain_first
        .memory_mut()
        .drain_observers(|_, _| delivered += 1);
    assert_eq!(delivered_notifications(&drain), vec![notification(1)]);
    assert_eq!(delivered, 1);
    drain_first.memory_mut().cancel_liveness_token(token);
}

#[test]
fn pruned_cancellation_reason_degrades_to_stale_target() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("pruned-cancel");
    let ((handle, token), _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        (ui.memory_mut().subscribe_observer(token), token)
    });
    harness.memory_mut().cancel_liveness_token(token);
    let _ = harness.run_frame(|_| ());
    assert_eq!(harness.memory().liveness().tombstone_count(), 0);

    harness
        .memory_mut()
        .publish_observer(handle.id(), notification(1));
    let drain = harness
        .memory_mut()
        .drain_observers(|_, _| panic!("pruned cancellation remains stale"));
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::StaleTarget {
            target: LivenessTargetId::new(target),
        }]
    );
}

#[test]
fn observer_registry_equality_ignores_only_private_liveness_scope() {
    let target = WidgetId::from_key("observer-equality");
    let mut left = UiMemory::new();
    let mut right = UiMemory::new();
    let left_token = left.mark_present_target(target);
    let right_token = right.mark_present_target(target);
    let left_handle = left.subscribe_observer(left_token);
    let right_handle = right.subscribe_observer(right_token);

    assert_ne!(left_token, right_token);
    assert_eq!(left.observers(), right.observers());
    assert!(left_handle.is_active());
    assert!(right_handle.is_active());
}

#[test]
fn reentrant_publish_during_drain_waits_for_later_drain_pass() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("timeline");
    let (handle, _) = harness.run_frame(|ui| {
        let token = ui.mark_present_target(target);
        ui.memory_mut().subscribe_observer(token)
    });
    let subscription_id = handle.id();
    harness
        .memory_mut()
        .publish_observer(subscription_id, notification(1));

    let mut delivered = Vec::new();
    let first_drain = harness.memory_mut().drain_observers(|observers, delivery| {
        delivered.push(delivery.notification_id());
        observers.publish(subscription_id, notification(2));
    });
    assert_eq!(delivered_notifications(&first_drain), vec![notification(1)]);
    assert_eq!(harness.memory().observers().queued_notification_count(), 1);

    let second_drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });
    assert_eq!(delivered, vec![notification(1), notification(2)]);
    assert_eq!(
        delivered_notifications(&second_drain),
        vec![notification(2)]
    );
}

#[test]
fn observer_operations_do_not_create_frame_output_or_harness_pending_work() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("idle-observer");

    let ((), output, trace) = harness.run_frame_with_trace(|ui| {
        let token = ui.mark_present_target(target);
        let handle = ui.memory_mut().subscribe_observer(token);
        ui.memory_mut()
            .publish_observer(handle.id(), notification(1));
        let drain = ui.memory_mut().drain_observers(|_, _| {});
        assert_eq!(delivered_notifications(&drain), vec![notification(1)]);
    });

    assert!(output.primitives.is_empty());
    assert!(output.semantics.nodes().is_empty());
    assert_eq!(output.repaint, RepaintRequest::None);
    assert!(output.actions.is_empty());
    assert!(output.platform_requests.is_empty());
    assert!(output.warnings.is_empty());
    assert_eq!(
        trace.phases(),
        &[
            HarnessPhase::FrameBegin,
            HarnessPhase::Build,
            HarnessPhase::FrameFinalization,
            HarnessPhase::InspectSemantics,
            HarnessPhase::InspectActions,
            HarnessPhase::InspectPlatformRequests,
            HarnessPhase::InspectRepaint,
            HarnessPhase::InspectWarnings,
        ]
    );

    let result = harness.settle_frames(1, |ui| {
        let token = ui.mark_present_target(target);
        let handle = ui.memory_mut().subscribe_observer(token);
        ui.memory_mut()
            .publish_observer(handle.id(), notification(1));
        let _ = ui.memory_mut().drain_observers(|_, _| {});
    });
    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 1);
    assert_eq!(result.pending_cause(), None::<SettlePendingCause>);
}
