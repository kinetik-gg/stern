//! Deterministic durable async-liveness conformance.

use kinetik_ui_core::{
    HarnessPhase, LivenessIncarnation, LivenessRegistry, LivenessRemovalStatus, LivenessTargetId,
    LivenessToken, LivenessUpdateStatus, RepaintRequest, SettlePendingCause, UiMemory,
    UiTestHarness, WidgetId,
};

fn target(name: &str) -> WidgetId {
    WidgetId::from_key(name)
}

fn stale_target(id: WidgetId) -> LivenessUpdateStatus {
    LivenessUpdateStatus::StaleTarget {
        target: LivenessTargetId::new(id),
    }
}

fn cancelled(id: WidgetId, incarnation: LivenessIncarnation) -> LivenessUpdateStatus {
    LivenessUpdateStatus::Cancelled {
        target: LivenessTargetId::new(id),
        incarnation,
    }
}

fn stale_incarnation(
    id: WidgetId,
    token_incarnation: LivenessIncarnation,
    current_incarnation: LivenessIncarnation,
) -> LivenessUpdateStatus {
    LivenessUpdateStatus::StaleIncarnation {
        target: LivenessTargetId::new(id),
        token_incarnation,
        current_incarnation,
    }
}

#[test]
fn continuous_presence_keeps_one_token_for_one_thousand_frames() {
    let mut harness = UiTestHarness::new();
    let id = target("preview");
    let (first, output) = harness.run_frame(|ui| ui.mark_present_target(id));

    assert!(output.warnings.is_empty());
    assert_eq!(first.target(), LivenessTargetId::new(id));
    assert_eq!(first.incarnation(), LivenessIncarnation::FIRST);

    for frame in 1..1_000 {
        let (token, output) = harness.run_frame(|ui| {
            let first_mark = ui.mark_present_target(id);
            let second_mark = ui.mark_present_target(id);
            assert_eq!(first_mark, second_mark, "same-frame mark at {frame}");
            first_mark
        });
        assert_eq!(token, first, "continuous frame {frame}");
        assert!(output.warnings.is_empty());
    }

    assert!(harness.memory().liveness().is_present(id));
    assert!(harness.memory().liveness().is_active(id));
    assert_eq!(harness.memory().liveness().active_count(), 1);
    assert_eq!(harness.memory().liveness().tombstone_count(), 0);

    let mut applied = 0;
    for _ in 0..2 {
        assert_eq!(
            harness
                .memory()
                .apply_liveness_update(first, || applied += 1),
            LivenessUpdateStatus::Applied
        );
    }
    assert_eq!(applied, 2, "accepted calls are not one-shot deduplicated");
}

#[test]
fn unseen_active_token_applies_until_omission_finalizes() {
    let mut harness = UiTestHarness::new();
    let id = target("thumbnail");
    let (token, _) = harness.run_frame(|ui| ui.mark_present_target(id));

    let ((status, mutations), _) = harness.run_frame(|ui| {
        assert!(!ui.memory().liveness().is_present(id));
        assert!(ui.memory().liveness().is_active(id));
        assert_eq!(
            ui.memory().liveness().current_incarnation(id),
            Some(token.incarnation())
        );
        let mut mutations = 0;
        let status = ui.memory().apply_liveness_update(token, || mutations += 1);
        (status, mutations)
    });

    assert_eq!(status, LivenessUpdateStatus::Applied);
    assert_eq!(mutations, 1);
    assert!(!harness.memory().liveness().is_active(id));
    assert_eq!(harness.memory().liveness().tombstone_count(), 1);
    assert_eq!(
        harness.memory().liveness().validate(token),
        stale_target(id)
    );
}

#[test]
fn apply_and_remove_orders_are_deterministic_and_removal_is_idempotent() {
    let id = target("remove-race");

    let mut result_first = UiTestHarness::new();
    let (token, _) = result_first.run_frame(|ui| ui.mark_present_target(id));
    let mut applied = 0;
    assert_eq!(
        result_first
            .memory()
            .apply_liveness_update(token, || applied += 1),
        LivenessUpdateStatus::Applied
    );
    assert_eq!(
        result_first.memory_mut().remove_live_target(id),
        LivenessRemovalStatus::Removed
    );
    assert_eq!(applied, 1);
    assert_eq!(
        result_first.memory().liveness().validate(token),
        stale_target(id)
    );

    let mut remove_first = UiTestHarness::new();
    let (token, _) = remove_first.run_frame(|ui| ui.mark_present_target(id));
    assert_eq!(
        remove_first.memory_mut().remove_live_target(id),
        LivenessRemovalStatus::Removed
    );
    assert_eq!(
        remove_first.memory_mut().remove_live_target(id),
        LivenessRemovalStatus::AlreadyAbsent
    );
    let mut rejected = 0;
    assert_eq!(
        remove_first
            .memory()
            .apply_liveness_update(token, || rejected += 1),
        stale_target(id)
    );
    assert_eq!(rejected, 0);
}

#[test]
fn apply_and_cancel_orders_are_deterministic_and_cancel_is_idempotent() {
    let id = target("cancel-race");

    let mut result_first = UiTestHarness::new();
    let (token, _) = result_first.run_frame(|ui| ui.mark_present_target(id));
    let mut applied = 0;
    assert_eq!(
        result_first
            .memory()
            .apply_liveness_update(token, || applied += 1),
        LivenessUpdateStatus::Applied
    );
    let expected = cancelled(id, token.incarnation());
    assert_eq!(
        result_first.memory_mut().cancel_liveness_token(token),
        expected
    );
    assert_eq!(
        result_first.memory_mut().cancel_liveness_token(token),
        expected
    );
    assert_eq!(applied, 1);

    let mut cancel_first = UiTestHarness::new();
    let (token, _) = cancel_first.run_frame(|ui| ui.mark_present_target(id));
    let expected = cancelled(id, token.incarnation());
    assert_eq!(
        cancel_first.memory_mut().cancel_liveness_token(token),
        expected
    );
    let mut rejected = 0;
    assert_eq!(
        cancel_first
            .memory()
            .apply_liveness_update(token, || rejected += 1),
        expected
    );
    assert_eq!(rejected, 0);
}

#[test]
fn replacement_has_precedence_over_an_older_cancelled_tombstone() {
    let mut harness = UiTestHarness::new();
    let id = target("cancelled-replacement");
    let (old, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    assert_eq!(
        harness.memory_mut().cancel_liveness_token(old),
        cancelled(id, old.incarnation())
    );

    let (replacement, _) = harness.run_frame(|ui| ui.restart_liveness_target(id));
    let stale = stale_incarnation(id, old.incarnation(), replacement.incarnation());
    assert_eq!(harness.memory().liveness().validate(old), stale);
    assert_eq!(harness.memory_mut().cancel_liveness_token(old), stale);
    assert_eq!(
        harness.memory().liveness().validate(replacement),
        LivenessUpdateStatus::Applied
    );
    assert!(harness.memory().liveness().is_active(id));
}

#[test]
fn omission_reentry_and_restart_never_reuse_an_incarnation() {
    let mut harness = UiTestHarness::new();
    let id = target("reused-target");
    let (first, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    let _ = harness.run_frame(|_| ());

    let (second, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    assert!(second.incarnation() > first.incarnation());
    assert_eq!(
        harness.memory().liveness().validate(first),
        stale_incarnation(id, first.incarnation(), second.incarnation())
    );

    let (third, _) = harness.run_frame(|ui| ui.restart_liveness_target(id));
    assert!(third.incarnation() > second.incarnation());
    assert_eq!(
        harness.memory().liveness().validate(second),
        stale_incarnation(id, second.incarnation(), third.incarnation())
    );
}

#[test]
fn registry_scope_rejects_equal_target_and_incarnation_from_another_registry() {
    let id = target("cross-registry");
    let mut left = LivenessRegistry::new();
    let mut right = LivenessRegistry::new();
    let left_token = left.mark_present(id);
    let right_token = right.mark_present(id);

    assert_eq!(left_token.target(), right_token.target());
    assert_eq!(left_token.incarnation(), right_token.incarnation());
    assert_ne!(left_token, right_token);
    assert_eq!(
        left, right,
        "observational equality ignores authority scope"
    );
    assert_eq!(left.validate(right_token), stale_target(id));
    assert_eq!(right.validate(left_token), stale_target(id));
}

#[test]
fn ui_memory_equality_is_scope_neutral_but_tokens_are_not_interchangeable() {
    let id = target("memory-equality");
    let mut left = UiMemory::new();
    let mut right = UiMemory::new();
    assert_eq!(left, right);

    let left_token = left.mark_present_target(id);
    let right_token = right.mark_present_target(id);
    assert_eq!(left, right);
    assert_ne!(left_token, right_token);
    assert_eq!(left.liveness().validate(right_token), stale_target(id));
}

#[test]
fn cancelled_tombstone_survives_following_frame_without_refresh_then_prunes() {
    let mut harness = UiTestHarness::new();
    let id = target("cancel-tombstone");
    let (token, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    let expected = cancelled(id, token.incarnation());
    assert_eq!(harness.memory_mut().cancel_liveness_token(token), expected);
    assert_eq!(harness.memory().liveness().tombstone_count(), 1);

    let ((before, repeated, count), _) = harness.run_frame(|ui| {
        let before = ui.memory().liveness().validate(token);
        let repeated = ui.memory_mut().cancel_liveness_token(token);
        let count = ui.memory().liveness().tombstone_count();
        (before, repeated, count)
    });
    assert_eq!(before, expected);
    assert_eq!(repeated, expected);
    assert_eq!(count, 1);

    assert_eq!(harness.memory().liveness().tombstone_count(), 0);
    assert_eq!(
        harness.memory().liveness().validate(token),
        stale_target(id)
    );
}

#[test]
fn omitted_and_removed_tombstones_prune_after_one_following_frame() {
    let mut omitted = UiTestHarness::new();
    let omitted_id = target("omitted-tombstone");
    let (omitted_token, _) = omitted.run_frame(|ui| ui.mark_present_target(omitted_id));
    let _ = omitted.run_frame(|_| ());
    assert_eq!(omitted.memory().liveness().tombstone_count(), 1);
    let ((status, count), _) = omitted.run_frame(|ui| {
        (
            ui.memory().liveness().validate(omitted_token),
            ui.memory().liveness().tombstone_count(),
        )
    });
    assert_eq!(status, stale_target(omitted_id));
    assert_eq!(count, 1);
    assert_eq!(omitted.memory().liveness().tombstone_count(), 0);

    let mut removed = UiTestHarness::new();
    let removed_id = target("removed-tombstone");
    let (removed_token, _) = removed.run_frame(|ui| ui.mark_present_target(removed_id));
    assert_eq!(
        removed.memory_mut().remove_live_target(removed_id),
        LivenessRemovalStatus::Removed
    );
    let ((status, count), _) = removed.run_frame(|ui| {
        (
            ui.memory().liveness().validate(removed_token),
            ui.memory().liveness().tombstone_count(),
        )
    });
    assert_eq!(status, stale_target(removed_id));
    assert_eq!(count, 1);
    assert_eq!(removed.memory().liveness().tombstone_count(), 0);
}

#[test]
fn pruned_history_cannot_create_aba_after_same_id_reuse() {
    let mut harness = UiTestHarness::new();
    let id = target("aba");
    let (old, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    let _ = harness.run_frame(|_| ());
    let _ = harness.run_frame(|_| ());
    assert_eq!(harness.memory().liveness().tombstone_count(), 0);

    let (replacement, _) = harness.run_frame(|ui| ui.mark_present_target(id));
    assert!(replacement.incarnation() > old.incarnation());
    assert_eq!(
        harness.memory().liveness().validate(old),
        stale_incarnation(id, old.incarnation(), replacement.incarnation())
    );
}

#[test]
fn widget_presence_does_not_preserve_async_presence() {
    let mut harness = UiTestHarness::new();
    let id = target("widget-is-not-async-owner");
    let (token, _) = harness.run_frame(|ui| {
        ui.register_id(id);
        ui.mark_present_target(id)
    });

    let _ = harness.run_frame(|ui| ui.register_id(id));
    assert!(!harness.memory().liveness().is_active(id));
    assert_eq!(
        harness.memory().liveness().validate(token),
        stale_target(id)
    );
}

#[test]
fn liveness_token_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LivenessToken>();
}

#[allow(deprecated)]
#[test]
fn deprecated_generation_and_live_aliases_forward_without_renewal() {
    use kinetik_ui_core::LivenessGeneration;

    let id = target("compatibility");
    let mut registry = LivenessRegistry::new();
    let first = registry.mark_live(id);
    let second = registry.mark_live(id);
    let generation: LivenessGeneration = first.generation();

    assert_eq!(first, second);
    assert_eq!(generation, first.incarnation());
    assert!(registry.is_live(id));
    assert_eq!(registry.current_generation(id), Some(generation));
}

#[test]
fn liveness_does_not_create_frame_output_or_settle_work() {
    let mut harness = UiTestHarness::new();
    let id = target("idle-target");

    let ((), output, trace) = harness.run_frame_with_trace(|ui| {
        ui.mark_present_target(id);
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
        ui.mark_present_target(id);
    });
    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 1);
    assert_eq!(result.pending_cause(), None::<SettlePendingCause>);
}
