//! Logical text-owner mode and platform IME lifecycle conformance.

use kinetik_ui_core::{
    ClipId, PlatformRequest, Primitive, Rect, TextInputOwnerMode, Transform, UiInputEvent,
    UiTestHarness, Vec2, WidgetId,
};

#[test]
fn read_only_owner_claims_ordered_input_once_without_platform_ime() {
    let owner = WidgetId::from_key("read-only-owner");
    let mut harness = UiTestHarness::new();
    harness.text_commit("ignored by policy layer");

    let ((first, second), output) = harness.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        let first = ui
            .claim_ordered_text_input_events(owner)
            .expect("valid root input")
            .expect("read-only owner claims once");
        let second = ui
            .claim_ordered_text_input_events(owner)
            .expect("valid root input");
        (first, second)
    });

    assert_eq!(first.len(), 1);
    assert!(matches!(first[0].event, UiInputEvent::Text(_)));
    assert!(second.is_none());
    assert_eq!(harness.memory().text_input_owner(), Some(owner));
    assert_eq!(
        harness.memory().text_input_owner_mode(),
        Some(TextInputOwnerMode::ReadOnly)
    );
    assert!(output.platform_requests.is_empty());
}

#[test]
fn editable_owner_starts_then_updates_with_projected_visible_caret() {
    let owner = WidgetId::from_key("editable-owner");
    let local = Rect::new(2.0, 3.0, 1.0, 8.0);
    let expected = Rect::new(12.0, 23.0, 1.0, 8.0);
    let mut harness = UiTestHarness::new();

    let ((), first) = harness.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(owner, local));
        ui.push_primitive(Primitive::TransformEnd);
    });

    assert_eq!(
        first.platform_requests,
        vec![PlatformRequest::StartTextInput {
            rect: Some(expected),
        }]
    );

    let next = Rect::new(4.0, 5.0, 1.0, 8.0);
    let ((), second) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(owner, next));
        ui.push_primitive(Primitive::TransformEnd);
    });

    assert_eq!(
        second.platform_requests,
        vec![PlatformRequest::UpdateTextInputRect {
            rect: Rect::new(14.0, 25.0, 1.0, 8.0),
        }]
    );
}

#[test]
fn editable_and_read_only_transitions_stop_and_restart_exactly() {
    let owner = WidgetId::from_key("mode-owner");
    let rect = Rect::new(4.0, 6.0, 1.0, 10.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let ((), read_only) = harness.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        assert!(!ui.publish_text_input_rect(owner, rect));
    });
    assert_eq!(
        read_only.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
    assert_eq!(
        harness.memory().text_input_owner_mode(),
        Some(TextInputOwnerMode::ReadOnly)
    );

    let ((), editable) = harness.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(owner, rect));
    });
    assert_eq!(
        editable.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: Some(rect) }]
    );
}

#[test]
fn mode_flip_before_pending_stop_drains_stop_before_restart() {
    let owner = WidgetId::from_key("mode-flip-owner");
    let rect = Rect::new(2.0, 2.0, 1.0, 12.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let ((), output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.memory_mut()
            .set_text_input_owner_mode(owner, TextInputOwnerMode::ReadOnly);
        ui.memory_mut()
            .set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
        assert!(ui.publish_text_input_rect(owner, rect));
    });

    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn logical_owner_handoff_stops_before_starting_new_owner() {
    let old_owner = WidgetId::from_key("old-owner");
    let new_owner = WidgetId::from_key("new-owner");
    let rect = Rect::new(8.0, 9.0, 1.0, 12.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(old_owner);
    harness.memory_mut().set_text_input_owner(old_owner);

    let ((), output) = harness.run_frame(|ui| {
        ui.memory_mut().focus(new_owner);
        assert!(ui.prepare_text_input_owner(new_owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(new_owner, rect));
    });

    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
    assert_eq!(harness.memory().text_input_owner(), Some(new_owner));
}

#[test]
fn legacy_same_owner_reacquisition_cancels_stop_but_prepare_restarts() {
    let owner = WidgetId::from_key("legacy-reacquire");
    let rect = Rect::new(1.0, 1.0, 1.0, 10.0);

    let mut legacy = UiTestHarness::new();
    legacy.memory_mut().focus(owner);
    legacy.memory_mut().set_text_input_owner(owner);
    legacy.memory_mut().clear_text_input_owner();
    legacy.memory_mut().set_text_input_owner(owner);
    let ((), legacy_output) = legacy.run_frame(|ui| {
        ui.register_id(owner);
    });
    assert!(legacy_output.platform_requests.is_empty());

    let mut prepared = UiTestHarness::new();
    prepared.memory_mut().focus(owner);
    prepared.memory_mut().set_text_input_owner(owner);
    prepared.memory_mut().clear_text_input_owner();
    let ((), prepared_output) = prepared.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(owner, rect));
    });
    assert_eq!(
        prepared_output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn prepare_then_legacy_reacquisition_cancels_undrained_stop() {
    let owner = WidgetId::from_key("prepare-legacy-owner");
    let rect = Rect::new(2.0, 3.0, 1.0, 10.0);

    let mut without_geometry = UiTestHarness::new();
    without_geometry.memory_mut().focus(owner);
    without_geometry.memory_mut().set_text_input_owner(owner);
    let ((), idle) = without_geometry.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        ui.memory_mut().set_text_input_owner(owner);
    });
    assert!(idle.platform_requests.is_empty());
    assert_eq!(
        without_geometry.memory().text_input_owner_mode(),
        Some(TextInputOwnerMode::Editable)
    );

    let mut with_geometry = UiTestHarness::new();
    with_geometry.memory_mut().focus(owner);
    with_geometry.memory_mut().set_text_input_owner(owner);
    let ((), updated) = with_geometry.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        ui.memory_mut().set_text_input_owner(owner);
        assert!(ui.publish_text_input_rect(owner, rect));
    });
    assert_eq!(
        updated.platform_requests,
        vec![PlatformRequest::UpdateTextInputRect { rect }]
    );
}

#[test]
fn different_legacy_owner_preserves_old_stop_before_new_start() {
    let old_owner = WidgetId::from_key("legacy-old");
    let new_owner = WidgetId::from_key("legacy-new");
    let rect = Rect::new(3.0, 4.0, 1.0, 11.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(old_owner);
    harness.memory_mut().set_text_input_owner(old_owner);
    harness.memory_mut().clear_text_input_owner();
    harness.memory_mut().set_text_input_owner(new_owner);
    harness.memory_mut().focus(new_owner);

    let ((), output) = harness.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(new_owner, TextInputOwnerMode::Editable));
        assert!(ui.publish_text_input_rect(new_owner, rect));
    });

    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn rejected_publication_preserves_pending_stop_and_owner_state() {
    let owner = WidgetId::from_key("clipped-owner");
    let clip = ClipId::from_raw(91);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let ((), output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.memory_mut()
            .set_text_input_owner_mode(owner, TextInputOwnerMode::ReadOnly);
        ui.memory_mut()
            .set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        assert!(!ui.publish_text_input_rect(owner, Rect::new(20.0, 20.0, 1.0, 8.0)));
        ui.push_primitive(Primitive::ClipEnd { id: clip });

        assert_eq!(ui.memory().focused(), Some(owner));
        assert_eq!(ui.memory().text_input_owner(), Some(owner));
        assert_eq!(
            ui.memory().text_input_owner_mode(),
            Some(TextInputOwnerMode::Editable)
        );
        ui.memory_mut().set_text_input_owner(owner);
    });

    assert!(output.platform_requests.is_empty());
}

#[test]
fn raw_text_requests_cannot_bypass_read_only_or_inactive_authority() {
    let owner = WidgetId::from_key("raw-read-only-owner");
    let rect = Rect::new(1.0, 2.0, 1.0, 8.0);
    let mut read_only = UiTestHarness::new();

    let ((), read_only_output) = read_only.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        ui.push_platform_request(PlatformRequest::StartTextInput { rect: Some(rect) });
        ui.push_platform_request(PlatformRequest::StartTextInput { rect: None });
        ui.push_platform_request(PlatformRequest::UpdateTextInputRect { rect });
    });
    assert!(read_only_output.platform_requests.is_empty());

    let mut inactive = UiTestHarness::new();
    let ((), inactive_output) = inactive.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        ui.push_platform_request(PlatformRequest::UpdateTextInputRect { rect });
    });
    assert!(inactive_output.platform_requests.is_empty());
}

#[test]
fn accepted_raw_start_after_handoff_drains_stop_first_and_tracks_activity() {
    let old_owner = WidgetId::from_key("raw-old-owner");
    let new_owner = WidgetId::from_key("raw-new-owner");
    let rect = Rect::new(3.0, 4.0, 1.0, 9.0);
    let next_rect = Rect::new(5.0, 6.0, 1.0, 9.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(old_owner);
    harness.memory_mut().set_text_input_owner(old_owner);

    let ((), handoff) = harness.run_frame(|ui| {
        ui.memory_mut().focus(new_owner);
        assert!(ui.prepare_text_input_owner(new_owner, TextInputOwnerMode::Editable));
        ui.push_platform_request(PlatformRequest::StartTextInput { rect: Some(rect) });
    });
    assert_eq!(
        handoff.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );

    let ((), update) = harness.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(new_owner, TextInputOwnerMode::Editable));
        ui.push_platform_request(PlatformRequest::UpdateTextInputRect { rect: next_rect });
    });
    assert_eq!(
        update.platform_requests,
        vec![PlatformRequest::UpdateTextInputRect { rect: next_rect }]
    );
}

#[test]
fn spatially_rejected_raw_start_leaves_pending_stop_for_finalization() {
    let old_owner = WidgetId::from_key("raw-clipped-old");
    let new_owner = WidgetId::from_key("raw-clipped-new");
    let clip = ClipId::from_raw(93);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(old_owner);
    harness.memory_mut().set_text_input_owner(old_owner);

    let ((), output) = harness.run_frame(|ui| {
        ui.memory_mut().focus(new_owner);
        assert!(ui.prepare_text_input_owner(new_owner, TextInputOwnerMode::Editable));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        ui.push_platform_request(PlatformRequest::StartTextInput {
            rect: Some(Rect::new(20.0, 20.0, 1.0, 8.0)),
        });
        assert!(ui.output().platform_requests.is_empty());
        ui.push_primitive(Primitive::ClipEnd { id: clip });
    });

    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
}

#[test]
fn raw_stop_synchronizes_state_so_next_geometry_restarts() {
    let owner = WidgetId::from_key("raw-stop-owner");
    let rect = Rect::new(1.0, 1.0, 1.0, 10.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let ((), output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.push_platform_request(PlatformRequest::StopTextInput);
        assert!(ui.publish_text_input_rect(owner, rect));
    });

    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn wrong_unfocused_and_read_only_publications_are_side_effect_free() {
    let owner = WidgetId::from_key("publication-owner");
    let other = WidgetId::from_key("publication-other");
    let rect = Rect::new(1.0, 2.0, 1.0, 9.0);
    let mut harness = UiTestHarness::new();

    let ((), output) = harness.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
        assert!(!ui.publish_text_input_rect(owner, rect));
        assert!(!ui.publish_text_input_rect(other, rect));
    });
    assert!(output.platform_requests.is_empty());
    assert_eq!(
        harness.memory().text_input_owner_mode(),
        Some(TextInputOwnerMode::ReadOnly)
    );

    harness.memory_mut().clear_focus();
    let ((), unfocused) = harness.run_frame(|ui| {
        assert!(!ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        assert!(!ui.publish_text_input_rect(owner, rect));
    });
    assert!(unfocused.platform_requests.is_empty());
}

#[test]
fn invisible_prepare_marks_presence_without_changing_owner_state() {
    let owner = WidgetId::from_key("invisible-owner");
    let clip = ClipId::from_raw(92);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness
        .memory_mut()
        .set_text_input_owner_mode(owner, TextInputOwnerMode::ReadOnly);

    let ((), output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::ZERO,
        });
        assert!(!ui.prepare_text_input_owner(owner, TextInputOwnerMode::Editable));
        ui.push_primitive(Primitive::ClipEnd { id: clip });
    });

    assert_eq!(harness.memory().focused(), Some(owner));
    assert_eq!(harness.memory().text_input_owner(), Some(owner));
    assert_eq!(
        harness.memory().text_input_owner_mode(),
        Some(TextInputOwnerMode::ReadOnly)
    );
    assert!(output.platform_requests.is_empty());
}

#[test]
fn reconciliation_stops_only_platform_active_missing_owners() {
    let read_only = WidgetId::from_key("missing-read-only");
    let mut logical = UiTestHarness::new();
    let _ = logical.run_frame(|ui| {
        ui.memory_mut().focus(read_only);
        assert!(ui.prepare_text_input_owner(read_only, TextInputOwnerMode::ReadOnly));
    });
    let ((), logical_missing) = logical.run_frame(|_| {});
    assert!(logical_missing.platform_requests.is_empty());
    assert_eq!(logical.memory().text_input_owner(), None);

    let editable = WidgetId::from_key("missing-editable");
    let mut active = UiTestHarness::new();
    active.memory_mut().focus(editable);
    active.memory_mut().set_text_input_owner(editable);
    let ((), active_missing) = active.run_frame(|_| {});
    assert_eq!(
        active_missing.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
    assert_eq!(active.memory().text_input_owner(), None);
}

#[test]
fn compatibility_start_supports_none_update_and_read_only_upgrade() {
    let owner = WidgetId::from_key("compat-owner");
    let rect = Rect::new(5.0, 6.0, 1.0, 12.0);
    let mut harness = UiTestHarness::new();

    let ((), start) = harness.run_frame(|ui| {
        ui.memory_mut().focus(owner);
        assert!(ui.start_text_input(owner, None));
    });
    assert_eq!(
        start.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: None }]
    );

    let ((), update) = harness.run_frame(|ui| {
        assert!(ui.start_text_input(owner, Some(rect)));
    });
    assert_eq!(
        update.platform_requests,
        vec![PlatformRequest::UpdateTextInputRect { rect }]
    );

    let ((), read_only) = harness.run_frame(|ui| {
        assert!(ui.prepare_text_input_owner(owner, TextInputOwnerMode::ReadOnly));
    });
    assert_eq!(
        read_only.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );

    let ((), upgraded) = harness.run_frame(|ui| {
        assert!(ui.start_text_input(owner, Some(rect)));
    });
    assert_eq!(
        upgraded.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: Some(rect) }]
    );
}

#[test]
fn owner_mode_and_platform_activity_participate_in_memory_equality() {
    let owner = WidgetId::from_key("equality-owner");
    let mut logical = kinetik_ui_core::UiMemory::new();
    let mut active = kinetik_ui_core::UiMemory::new();
    logical.focus(owner);
    active.focus(owner);
    logical.set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
    active.set_text_input_owner(owner);

    assert_ne!(logical, active);
    active.clear_text_input_owner();
    logical.clear_text_input_owner();
    let _ = active.take_pending_text_input_stop();
    assert_eq!(logical, active);
}
