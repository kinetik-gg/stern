//! Deterministic async-style liveness token conformance.

use kinetik_ui_core::{
    HarnessPhase, LivenessTargetId, LivenessUpdateStatus, RepaintRequest, SettlePendingCause,
    UiTestHarness, WidgetId,
};

#[test]
fn matching_live_token_update_applies_once() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("preview");

    let (token, output) = harness.run_frame(|ui| ui.mark_live_target(target));

    assert!(output.warnings.is_empty());
    assert_eq!(token.target(), LivenessTargetId::new(target));
    assert_eq!(token.generation().value(), 1);

    let mut applied = 0;
    let status = harness.memory().apply_liveness_update(token, || {
        applied += 1;
    });

    assert_eq!(status, LivenessUpdateStatus::Applied);
    assert_eq!(applied, 1);
}

#[test]
fn dropped_target_update_returns_stale_target_without_mutating() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("thumbnail");

    let (token, _) = harness.run_frame(|ui| ui.mark_live_target(target));
    let ((), output) = harness.run_frame(|_| ());

    assert!(output.warnings.is_empty());
    assert!(!harness.memory().liveness().is_live(target));

    let mut mutations = Vec::new();
    let status = harness.memory().apply_liveness_update(token, || {
        mutations.push("should not run");
    });

    assert_eq!(
        status,
        LivenessUpdateStatus::StaleTarget {
            target: LivenessTargetId::new(target)
        }
    );
    assert!(mutations.is_empty());
}

#[test]
fn same_id_reentry_after_absence_renews_generation_without_reviving_old_token() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("reused-target");

    let (old_token, _) = harness.run_frame(|ui| ui.mark_live_target(target));
    let _ = harness.run_frame(|_| ());

    let mut absent_mutations = 0;
    let absent_status = harness.memory().apply_liveness_update(old_token, || {
        absent_mutations += 1;
    });

    assert_eq!(
        absent_status,
        LivenessUpdateStatus::StaleTarget {
            target: LivenessTargetId::new(target)
        }
    );
    assert_eq!(absent_mutations, 0);

    let (new_token, _) = harness.run_frame(|ui| ui.mark_live_target(target));

    assert!(new_token.generation() > old_token.generation());

    let mut mutations = Vec::new();
    let old_status = harness.memory().apply_liveness_update(old_token, || {
        mutations.push("old token");
    });
    let new_status = harness.memory().apply_liveness_update(new_token, || {
        mutations.push("new token");
    });

    assert_eq!(
        old_status,
        LivenessUpdateStatus::StaleGeneration {
            target: LivenessTargetId::new(target),
            token_generation: old_token.generation(),
            current_generation: new_token.generation(),
        }
    );
    assert_eq!(new_status, LivenessUpdateStatus::Applied);
    assert_eq!(mutations, vec!["new token"]);
}

#[test]
fn unseen_target_update_returns_stale_target_during_next_frame() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("viewport");

    let (token, _) = harness.run_frame(|ui| ui.mark_live_target(target));
    let ((status, mutations), _) = harness.run_frame(|ui| {
        let mut mutations = 0;
        let status = ui.memory().apply_liveness_update(token, || {
            mutations += 1;
        });
        (status, mutations)
    });

    assert_eq!(
        status,
        LivenessUpdateStatus::StaleTarget {
            target: LivenessTargetId::new(target)
        }
    );
    assert_eq!(mutations, 0);
}

#[test]
fn renewing_same_target_makes_old_token_stale_generation() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("inspector");

    let (old_token, _) = harness.run_frame(|ui| ui.mark_live_target(target));
    let (new_token, _) = harness.run_frame(|ui| ui.mark_live_target(target));

    assert_eq!(old_token.generation().value(), 1);
    assert_eq!(new_token.generation().value(), 2);

    let mut mutations = 0;
    let old_status = harness.memory().apply_liveness_update(old_token, || {
        mutations += 1;
    });
    let new_status = harness.memory().apply_liveness_update(new_token, || {
        mutations += 1;
    });

    assert_eq!(
        old_status,
        LivenessUpdateStatus::StaleGeneration {
            target: LivenessTargetId::new(target),
            token_generation: old_token.generation(),
            current_generation: new_token.generation(),
        }
    );
    assert_eq!(new_status, LivenessUpdateStatus::Applied);
    assert_eq!(mutations, 1);
}

#[test]
fn marking_same_target_again_in_one_frame_renews_generation() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("timeline");

    let ((first, second), _) = harness.run_frame(|ui| {
        let first = ui.mark_live_target(target);
        let second = ui.mark_live_target(target);
        (first, second)
    });

    assert_eq!(first.generation().value(), 1);
    assert_eq!(second.generation().value(), 2);
    assert_eq!(
        harness.memory().liveness().validate(first),
        LivenessUpdateStatus::StaleGeneration {
            target: LivenessTargetId::new(target),
            token_generation: first.generation(),
            current_generation: second.generation(),
        }
    );
    assert_eq!(
        harness.memory().liveness().validate(second),
        LivenessUpdateStatus::Applied
    );
}

#[test]
fn liveness_does_not_create_frame_output_or_settle_work() {
    let mut harness = UiTestHarness::new();
    let target = WidgetId::from_key("idle-target");

    let ((), output, trace) = harness.run_frame_with_trace(|ui| {
        ui.mark_live_target(target);
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
        ui.mark_live_target(target);
    });

    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 1);
    assert_eq!(result.pending_cause(), None::<SettlePendingCause>);
}
