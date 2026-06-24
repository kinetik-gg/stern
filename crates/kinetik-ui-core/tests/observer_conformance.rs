//! Deterministic observer subscription conformance.

use kinetik_ui_core::{
    HarnessPhase, LivenessTargetId, ObserverDeliverySkipReason, ObserverDeliveryStatus,
    ObserverDrain, ObserverNotificationId, RepaintRequest, SettlePendingCause, UiTestHarness,
    WidgetId,
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
fn live_subscription_receives_one_notification_during_explicit_drain() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("preview");

    let (handle, _) = harness.run_frame(|ui| {
        let token = ui.mark_live_target(target);
        ui.memory_mut().subscribe_observer(token)
    });
    let subscription_id = handle.id();

    harness
        .memory_mut()
        .publish_observer(subscription_id, notification(1));

    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });

    assert_eq!(delivered, vec![notification(1)]);
    assert_eq!(delivered_notifications(&drain), vec![notification(1)]);
    assert!(skipped_reasons(&drain).is_empty());
    assert_eq!(harness.memory().observers().queued_notification_count(), 0);
}

#[test]
fn dropped_and_unsubscribed_subscriptions_skip_observable_deliveries() {
    let mut harness = UiTestHarness::new();
    let first_target = WidgetId::from_key("first");
    let second_target = WidgetId::from_key("second");

    let ((dropped_id, unsubscribed_id), _) = harness.run_frame(|ui| {
        let dropped_token = ui.mark_live_target(first_target);
        let dropped = ui.memory_mut().subscribe_observer(dropped_token);
        let dropped_id = dropped.id();
        drop(dropped);

        let unsubscribed_token = ui.mark_live_target(second_target);
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

    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });

    assert!(delivered.is_empty());
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
        let token = ui.mark_live_target(target);
        ui.memory_mut().subscribe_observer(token)
    });
    let subscription_id = handle.id();

    let ((), _) = harness.run_frame(|_| ());
    assert!(!harness.memory().liveness().is_live(target));

    harness
        .memory_mut()
        .publish_observer(subscription_id, notification(1));

    let mut mutations = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        mutations.push(delivery.notification_id());
    });

    assert!(mutations.is_empty());
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::StaleTarget {
            target: LivenessTargetId::new(target),
        }]
    );
}

#[test]
fn same_target_renewal_delivers_current_token_and_skips_old_token() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("reused-target");

    let ((old_handle, old_token), _) = harness.run_frame(|ui| {
        let old_token = ui.mark_live_target(target);
        (ui.memory_mut().subscribe_observer(old_token), old_token)
    });
    let old_subscription = old_handle.id();

    let ((new_handle, new_token), _) = harness.run_frame(|ui| {
        let new_token = ui.mark_live_target(target);
        (ui.memory_mut().subscribe_observer(new_token), new_token)
    });
    let new_subscription = new_handle.id();

    harness
        .memory_mut()
        .publish_observer(old_subscription, notification(1));
    harness
        .memory_mut()
        .publish_observer(new_subscription, notification(2));

    let mut delivered = Vec::new();
    let drain = harness.memory_mut().drain_observers(|_, delivery| {
        delivered.push(delivery.notification_id());
    });

    assert_eq!(delivered, vec![notification(2)]);
    assert_eq!(delivered_notifications(&drain), vec![notification(2)]);
    assert_eq!(
        skipped_reasons(&drain),
        vec![ObserverDeliverySkipReason::StaleGeneration {
            target: LivenessTargetId::new(target),
            token_generation: old_token.generation(),
            current_generation: new_token.generation(),
        }]
    );
}

#[test]
fn reentrant_publish_during_drain_waits_for_later_drain_pass() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("timeline");

    let (handle, _) = harness.run_frame(|ui| {
        let token = ui.mark_live_target(target);
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

    assert_eq!(delivered, vec![notification(1)]);
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
    assert_eq!(harness.memory().observers().queued_notification_count(), 0);
}

#[test]
fn observer_operations_do_not_create_frame_output_or_harness_pending_work() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("idle-observer");

    let ((), output, trace) = harness.run_frame_with_trace(|ui| {
        let token = ui.mark_live_target(target);
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
        let token = ui.mark_live_target(target);
        let handle = ui.memory_mut().subscribe_observer(token);
        ui.memory_mut()
            .publish_observer(handle.id(), notification(1));
        let _ = ui.memory_mut().drain_observers(|_, _| {});
    });

    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 1);
    assert_eq!(result.pending_cause(), None::<SettlePendingCause>);
}
