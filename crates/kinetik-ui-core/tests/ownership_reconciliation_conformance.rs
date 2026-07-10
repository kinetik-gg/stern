//! End-frame interaction-owner reconciliation conformance.

use kinetik_ui_core::{
    ClipId, MouseButton, PlatformRequest, Point, PointerOrder, PointerTarget, Primitive, Rect,
    RepaintRequest, SemanticNode, SemanticRole, UiTestHarness, Vec2, WidgetId, drop_target,
    pressable,
};

const FULL: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

fn install_pointer_transaction(harness: &mut UiTestHarness, owner: WidgetId) {
    harness.memory_mut().capture_pointer(owner);
    harness.memory_mut().activate(owner);
    harness.memory_mut().press(owner);
    harness.memory_mut().press_secondary(owner);
    harness.memory_mut().start_drag(owner);
}

fn assert_no_pointer_transaction(harness: &UiTestHarness) {
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pressed(), None);
    assert_eq!(harness.memory().secondary_pressed(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn missing_pointer_transaction_cancels_atomically_without_synthetic_release() {
    let removed = WidgetId::from_key("removed-pointer-owner");
    let replacement = WidgetId::from_key("replacement");
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    install_pointer_transaction(&mut harness, removed);

    let ((), present_output) = harness.run_frame(|ui| {
        ui.register_id(removed);
    });
    assert_eq!(harness.memory().pointer_capture(), Some(removed));
    assert_eq!(present_output.repaint, RepaintRequest::None);

    let ((), removed_output) = harness.run_frame(|_| {});
    assert_no_pointer_transaction(&harness);
    assert!(harness.memory().pointer_interaction_cancelled());
    assert_eq!(removed_output.repaint, RepaintRequest::NextFrame);

    harness.pointer_release(MouseButton::Primary);
    let ((response, drop), _) = harness.run_frame(|ui| {
        ui.register_id(replacement);
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(replacement, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(replacement, FULL, input, memory, false);
        (response, drop)
    });
    assert!(!response.clicked);
    assert!(!drop.dropped);
    assert_eq!(drop.source, None);
}

#[test]
fn missing_text_owner_stops_once_and_same_id_does_not_resurrect() {
    let owner = WidgetId::from_key("removed-text-owner");
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let _ = harness.run_frame(|ui| {
        ui.register_id(owner);
    });
    assert_eq!(harness.memory().focused(), Some(owner));
    assert_eq!(harness.memory().text_input_owner(), Some(owner));

    let ((), removed_output) = harness.run_frame(|_| {});
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(
        removed_output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
    assert_eq!(removed_output.repaint, RepaintRequest::NextFrame);

    let ((), next_output) = harness.run_frame(|_| {});
    assert!(next_output.platform_requests.is_empty());

    let ((), reappeared_output) = harness.run_frame(|ui| {
        ui.register_id(owner);
    });
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert!(reappeared_output.platform_requests.is_empty());
}

#[test]
fn registered_same_frame_text_handoff_emits_one_ordered_stop_start_pair() {
    let previous = WidgetId::from_key("previous-text-owner");
    let next = WidgetId::from_key("next-text-owner");
    let rect = Rect::new(10.0, 20.0, 80.0, 24.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(previous);
    harness.memory_mut().set_text_input_owner(previous);

    let ((), output) = harness.run_frame(|ui| {
        ui.register_id(previous);
        ui.register_id(next);
        ui.memory_mut().focus(next);
        assert!(ui.start_text_input(next, Some(rect)));
    });

    assert_eq!(harness.memory().focused(), Some(next));
    assert_eq!(harness.memory().text_input_owner(), Some(next));
    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn registered_disabled_clipped_and_hidden_owners_are_not_removals() {
    let disabled = WidgetId::from_key("disabled-owner");
    let clipped = WidgetId::from_key("clipped-owner");
    let hidden = WidgetId::from_key("hidden-owner");

    let mut disabled_harness = UiTestHarness::new();
    install_pointer_transaction(&mut disabled_harness, disabled);
    let _ = disabled_harness.run_frame(|ui| {
        ui.register_id(disabled);
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(disabled, FULL, input, memory, true);
        assert!(response.state.disabled);
    });
    assert_eq!(disabled_harness.memory().pointer_capture(), Some(disabled));

    let mut clipped_harness = UiTestHarness::new();
    clipped_harness.set_pointer_position(Point::new(10.0, 10.0));
    install_pointer_transaction(&mut clipped_harness, clipped);
    let clip = ClipId::from_raw(1);
    let _ = clipped_harness.run_frame(|ui| {
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(200.0, 200.0, 10.0, 10.0),
        });
        ui.register_id(clipped);
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(clipped, FULL, input, memory, false);
        assert!(!response.state.hovered);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
    });
    assert_eq!(clipped_harness.memory().pointer_capture(), Some(clipped));

    let mut hidden_harness = UiTestHarness::new();
    install_pointer_transaction(&mut hidden_harness, hidden);
    let _ = hidden_harness.run_frame(|ui| {
        ui.register_id(hidden);
    });
    assert_eq!(hidden_harness.memory().pointer_capture(), Some(hidden));
}

#[test]
fn pointer_plan_is_not_presence_but_registration_preserves_valid_capture() {
    let owner = WidgetId::from_key("planned-owner");
    let mut omitted = UiTestHarness::new();
    omitted.set_pointer_position(Point::new(10.0, 10.0));
    install_pointer_transaction(&mut omitted, owner);
    let ((), omitted_output) = omitted.run_frame(|ui| {
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(owner, FULL, PointerOrder::new(1)));
        })
        .expect("valid plan");
    });
    assert_no_pointer_transaction(&omitted);
    assert_eq!(omitted_output.repaint, RepaintRequest::NextFrame);

    let mut present = UiTestHarness::new();
    present.set_pointer_position(Point::new(10.0, 10.0));
    install_pointer_transaction(&mut present, owner);
    let _ = present.run_frame(|ui| {
        ui.register_id(owner);
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(owner, FULL, PointerOrder::new(1)));
        })
        .expect("valid plan");
    });
    assert_eq!(present.memory().pointer_capture(), Some(owner));
    assert_eq!(present.memory().drag_source(), Some(owner));
}

#[test]
fn semantic_and_text_input_evidence_mark_presence_without_duplicate_warnings() {
    let semantic_owner = WidgetId::from_key("semantic-owner");
    let text_owner = WidgetId::from_key("text-owner");
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(semantic_owner);

    let ((), semantic_output) = harness.run_frame(|ui| {
        ui.set_semantic_root(semantic_owner);
        ui.push_semantic_node(
            SemanticNode::new(semantic_owner, SemanticRole::Button, FULL).focusable(true),
        );
    });
    assert_eq!(harness.memory().focused(), Some(semantic_owner));
    assert!(semantic_output.warnings.is_empty());

    harness.memory_mut().focus(text_owner);
    harness.memory_mut().set_text_input_owner(text_owner);
    let ((), text_output) = harness.run_frame(|ui| {
        assert!(ui.start_text_input(text_owner, Some(FULL)));
    });
    assert_eq!(harness.memory().focused(), Some(text_owner));
    assert_eq!(harness.memory().text_input_owner(), Some(text_owner));
    assert!(text_output.warnings.is_empty());
    assert!(text_output.platform_requests.is_empty());
}

#[test]
fn reconciliation_preserves_unrelated_retained_and_application_state() {
    let removed = WidgetId::from_key("removed-owner");
    let retained = WidgetId::from_key("retained-state");
    let mut application_text = String::from("application-owned");
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(removed);
    harness.memory_mut().set_text_input_owner(removed);
    harness
        .memory_mut()
        .set_scroll_offset(retained, Vec2::new(3.0, 7.0));
    harness.memory_mut().open_popover(retained);

    let _ = harness.run_frame(|_| {
        application_text.push_str(" state");
    });

    assert_eq!(application_text, "application-owned state");
    assert_eq!(
        harness.memory().scroll_offset(retained),
        Vec2::new(3.0, 7.0)
    );
    assert!(harness.memory().is_popover_open(retained));
}
