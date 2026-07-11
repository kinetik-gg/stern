//! Ordered drag-threshold and captured-selection gesture conformance.

use kinetik_ui_core::{
    ClipId, CursorShape, InputWheelDelta, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point,
    PointerButtonState, PointerOrder, PointerTarget, Primitive, Rect, SelectionGesturePhase, Size,
    TextInputEvent, Transform, UiInputEvent, UiTestHarness, Vec2, WidgetId, draggable, drop_target,
    pressable, scrollable, tooltip_trigger,
};

const FULL: Rect = Rect::new(0.0, 0.0, 160.0, 80.0);

fn run_drag(harness: &mut UiTestHarness) -> kinetik_ui_core::Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, FULL, input, memory, false)
        })
        .0
}

fn run_press(harness: &mut UiTestHarness) -> kinetik_ui_core::Response {
    harness
        .run_frame(|ui| {
            let id = ui.id("press");
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, FULL, input, memory, false)
        })
        .0
}

fn crossed_drag_harness() -> UiTestHarness {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut harness);
    harness.set_pointer_position(Point::new(14.0, 10.0));
    let crossed = run_drag(&mut harness);
    assert!(crossed.dragged);
    assert_eq!(harness.memory().drag_source(), Some(crossed.id));
    harness
}

#[test]
fn below_threshold_release_clicks_and_exact_threshold_suppresses_click() {
    let mut below = UiTestHarness::new();
    below.set_pointer_position(Point::new(10.0, 10.0));
    below.pointer_press(MouseButton::Primary);
    let pressed = run_drag(&mut below);
    below.set_pointer_position(Point::new(13.0, 10.0));
    let moved = run_drag(&mut below);
    assert!(!moved.dragged);
    assert_eq!(below.memory().drag_source(), None);
    below.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut below);
    assert!(released.clicked);
    assert!(!released.double_clicked);
    assert_eq!(below.memory().released_drag_source(), None);
    assert_eq!(below.memory().pointer_capture(), None);

    let mut exact = UiTestHarness::new();
    exact.set_pointer_position(Point::new(10.0, 10.0));
    exact.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut exact);
    exact.set_pointer_position(Point::new(14.0, 10.0));
    let crossing = run_drag(&mut exact);
    assert!(crossing.dragged);
    assert_eq!(crossing.drag_delta, Vec2::new(4.0, 0.0));
    assert_eq!(exact.memory().drag_source(), Some(pressed.id));
    exact.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut exact);
    assert!(!released.clicked);
    assert_eq!(exact.memory().released_drag_source(), Some(released.id));
}

#[test]
fn crossing_reports_full_displacement_then_subsequent_delta_and_never_unlatches() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut harness);

    harness.set_pointer_position(Point::new(12.0, 10.0));
    assert!(!run_drag(&mut harness).dragged);

    harness.set_pointer_position(Point::new(14.0, 10.0));
    let crossing = run_drag(&mut harness);
    assert!(crossing.dragged);
    assert_eq!(crossing.drag_delta, Vec2::new(4.0, 0.0));

    harness.set_pointer_position(Point::new(17.0, 10.0));
    let later = run_drag(&mut harness);
    assert!(later.dragged);
    assert_eq!(later.drag_delta, Vec2::new(3.0, 0.0));

    harness.set_pointer_position(Point::new(11.0, 10.0));
    let moved_back = run_drag(&mut harness);
    assert!(moved_back.dragged);
    assert_eq!(moved_back.drag_delta, Vec2::new(-6.0, 0.0));

    harness.pointer_release(MouseButton::Primary);
    let released = run_drag(&mut harness);
    assert!(!released.clicked);
    assert_eq!(harness.memory().released_drag_source(), Some(released.id));
}

#[test]
fn same_frame_crossing_release_reports_inside_motion_but_outside_only_cleans_up() {
    let mut inside = UiTestHarness::new();
    inside.set_pointer_position(Point::new(10.0, 10.0));
    inside.pointer_press(MouseButton::Primary);
    inside.set_pointer_position(Point::new(14.0, 10.0));
    inside.pointer_release(MouseButton::Primary);
    let response = run_drag(&mut inside);
    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(4.0, 0.0));
    assert!(!response.clicked);
    assert_eq!(inside.memory().released_drag_source(), Some(response.id));

    let mut outside = UiTestHarness::new();
    outside.set_pointer_position(Point::new(10.0, 10.0));
    outside.pointer_press(MouseButton::Primary);
    outside.set_pointer_position(Point::new(200.0, 10.0));
    outside.pointer_release(MouseButton::Primary);
    let response = run_drag(&mut outside);
    assert!(!response.dragged);
    assert_eq!(response.drag_delta, Vec2::ZERO);
    assert!(!response.clicked);
    assert_eq!(outside.memory().pointer_capture(), None);
    assert_eq!(outside.memory().released_drag_source(), Some(response.id));
}

#[test]
fn release_event_position_can_cross_threshold_without_a_move_event() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(14.0, 10.0)),
    });

    let response = run_drag(&mut harness);
    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(4.0, 0.0));
    assert!(!response.clicked);
    assert_eq!(harness.memory().released_drag_source(), Some(response.id));
}

#[test]
fn pressable_suppresses_threshold_release_without_becoming_a_drag_source() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_press(&mut harness);
    harness.set_pointer_position(Point::new(14.0, 10.0));
    let moved = run_press(&mut harness);
    assert!(!moved.dragged);
    assert_eq!(harness.memory().drag_source(), None);
    harness.pointer_release(MouseButton::Primary);
    let released = run_press(&mut harness);
    assert!(!released.clicked);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn below_threshold_canonical_double_click_preserves_live_click_count() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    let _ = run_press(&mut harness);
    harness.set_pointer_position(Point::new(12.0, 10.0));
    let _ = run_press(&mut harness);
    harness.pointer_release(MouseButton::Primary);
    harness.set_click_count(2);
    let response = run_press(&mut harness);
    assert!(response.clicked);
    assert!(response.double_clicked);
}

#[test]
fn captured_selection_preserves_root_ordinals_without_domain_drag_or_replay() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit("x".to_owned())));
    harness.set_pointer_position(Point::new(18.0, 10.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness.pointer_release(MouseButton::Primary);

    let ((first, second), _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        let first = ui.captured_selection_gesture(id, FULL, false);
        let second = ui.captured_selection_gesture(id, FULL, false);
        (first, second)
    });

    assert_eq!(
        first
            .actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(1), SelectionGesturePhase::Press),
            (Some(3), SelectionGesturePhase::Move),
            (Some(5), SelectionGesturePhase::Release),
        ]
    );
    assert_eq!(first.actions[1].delta, Vec2::new(8.0, 0.0));
    assert!(second.actions.is_empty());
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn captured_selection_cancellation_is_not_replayed_in_one_frame() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.captured_selection_gesture(id, FULL, false)
    });

    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    let ((first, second), _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        let first = ui.captured_selection_gesture(id, FULL, false);
        let second = ui.captured_selection_gesture(id, FULL, false);
        (first, second)
    });

    assert_eq!(
        first
            .actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(0), SelectionGesturePhase::Move),
            (Some(1), SelectionGesturePhase::Cancel),
        ]
    );
    assert!(second.actions.is_empty());
}

#[test]
fn spatial_filtering_keeps_original_action_ordinals_with_gaps() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(200.0, 10.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit(
            "before".to_owned(),
        )));
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let clip = ClipId::from_raw(91);
    let (gesture, _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: FULL,
        });
        let gesture = ui.captured_selection_gesture(id, FULL, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        gesture
    });

    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| action.ordinal)
            .collect::<Vec<_>>(),
        vec![Some(3), Some(5), Some(6)]
    );
    assert!(gesture.response.clicked);
}

#[test]
fn ordered_text_claim_exposes_root_ordinals_without_pointer_reparsing() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(200.0, 10.0));
    harness.input_mut().push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(0.0, 1.0)),
        position: Some(Point::new(200.0, 10.0)),
    });
    harness.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(10.0, 10.0)),
    });
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit(
            "typed".to_owned(),
        )));

    let clip = ClipId::from_raw(94);
    let ((gesture, editing), _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.memory_mut().focus(id);
        ui.memory_mut().set_text_input_owner(id);
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: FULL,
        });
        let gesture = ui.captured_selection_gesture(id, FULL, false);
        let editing = ui
            .claim_ordered_text_input_events(id)
            .expect("valid root stream")
            .expect("focused owner claim");
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        (gesture, editing)
    });

    assert_eq!(gesture.actions[0].ordinal, Some(2));
    assert_eq!(editing.len(), 1);
    assert_eq!(editing[0].ordinal, Some(3));
    assert!(matches!(
        &editing[0].event,
        UiInputEvent::Text(TextInputEvent::Commit(text)) if text == "typed"
    ));
}

#[test]
fn canonical_release_outside_clip_is_cancel_only_with_original_ordinal() {
    let clip = ClipId::from_raw(92);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(pressed.response.id)
    );

    harness.set_pointer_position(Point::new(50.0, 10.0));
    harness.pointer_release(MouseButton::Primary);
    let cancelled = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;

    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(1));
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert!(!cancelled.response.clicked);
    assert!(!cancelled.response.dragged);
    assert_eq!(harness.memory().pointer_capture(), None);

    let mut ordinary = UiTestHarness::new();
    ordinary.set_pointer_position(Point::new(10.0, 10.0));
    ordinary.pointer_press(MouseButton::Primary);
    let _ = ordinary.run_frame(|ui| {
        let id = ui.id("press");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: clip_rect,
        });
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(id, FULL, input, memory, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        response
    });
    ordinary.set_pointer_position(Point::new(50.0, 10.0));
    ordinary.pointer_release(MouseButton::Primary);
    let response = ordinary
        .run_frame(|ui| {
            let id = ui.id("press");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = pressable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        })
        .0;
    assert!(!response.clicked);
    assert_eq!(ordinary.memory().pointer_capture(), None);
}

#[test]
fn canonical_secondary_release_outside_clip_is_cleanup_only() {
    let clip = ClipId::from_raw(95);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Secondary);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("press");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: clip_rect,
        });
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(id, FULL, input, memory, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        response
    });
    harness.set_pointer_position(Point::new(50.0, 10.0));
    harness.pointer_release(MouseButton::Secondary);
    let response = harness
        .run_frame(|ui| {
            let id = ui.id("press");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = pressable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        })
        .0;
    assert!(!response.secondary_clicked);
    assert_eq!(harness.memory().secondary_pressed(), None);
}

#[test]
fn release_all_emits_one_original_ordinal_cancel_and_clears_selection_capture() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(pressed.response.id)
    );

    harness.input_mut().release_pointer_buttons();
    let cancelled = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(0));
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);

    let clip = ClipId::from_raw(93);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut clipped = UiTestHarness::new();
    clipped.set_pointer_position(Point::new(10.0, 10.0));
    clipped.pointer_press(MouseButton::Primary);
    let _ = clipped.run_frame(|ui| {
        let id = ui.id("selection");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: clip_rect,
        });
        let gesture = ui.captured_selection_gesture(id, FULL, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        gesture
    });
    clipped.set_pointer_position(Point::new(50.0, 10.0));
    clipped.input_mut().release_pointer_buttons();
    let cancelled = clipped
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(1));
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
}

#[test]
fn ordered_move_before_release_all_is_not_discarded_by_frame_cleanup() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.captured_selection_gesture(id, FULL, false)
    });

    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.input_mut().release_pointer_buttons();
    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(0), SelectionGesturePhase::Move),
            (Some(1), SelectionGesturePhase::Cancel),
        ]
    );
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn secondary_owner_clears_when_the_participating_widget_becomes_disabled() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Secondary);
    let owner = run_press(&mut harness).id;
    assert_eq!(harness.memory().secondary_pressed(), Some(owner));

    let response = harness
        .run_frame(|ui| {
            let id = ui.id("press");
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, FULL, input, memory, true)
        })
        .0;
    assert!(response.state.disabled);
    assert_eq!(harness.memory().secondary_pressed(), None);
}

#[test]
fn selection_mode_change_cannot_publish_a_retained_domain_drop() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut harness);
    harness.set_pointer_position(Point::new(14.0, 10.0));
    let crossed = run_drag(&mut harness);
    assert_eq!(harness.memory().drag_source(), Some(crossed.id));

    harness.pointer_release(MouseButton::Primary);
    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(gesture.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn planned_target_first_drop_requires_current_domain_drag_intent_and_eligibility() {
    for disabled in [false, true] {
        let mut harness = crossed_drag_harness();
        harness.pointer_release(MouseButton::Primary);
        let drop = harness
            .run_frame(|ui| {
                let source = ui.id("drag");
                let target = ui.id("drop");
                ui.resolve_pointer_targets(|plan| {
                    let source_target =
                        PointerTarget::new(source, FULL, PointerOrder::new(20)).enabled(!disabled);
                    plan.target(if disabled {
                        source_target.domain_drag_source()
                    } else {
                        source_target
                    });
                    plan.target(
                        PointerTarget::new(target, FULL, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid fail-closed source-intent plan");
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, FULL, input, memory, false);
                if disabled {
                    let (input, memory) = ui.input_and_memory_mut();
                    let _ = draggable(source, FULL, input, memory, true);
                } else {
                    let _ = ui.captured_selection_gesture(source, FULL, false);
                }
                drop
            })
            .0;
        assert_eq!(drop.source, None);
        assert!(!drop.dropped);
    }
}

#[test]
fn selection_gesture_cannot_be_promoted_to_a_domain_drag() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let selection_id = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            assert_eq!(gesture.actions[0].phase, SelectionGesturePhase::Press);
            id
        })
        .0;

    harness.set_pointer_position(Point::new(14.0, 10.0));
    let response = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, FULL, input, memory, false)
        })
        .0;
    assert_eq!(response.id, selection_id);
    assert!(!response.dragged);
    assert_eq!(response.drag_delta, Vec2::ZERO);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn legacy_pre_press_relocation_is_not_replayed_as_drag_motion() {
    let mut harness = UiTestHarness::new();
    harness.input_mut().events.clear();
    harness.input_mut().pointer.position = Some(Point::new(100.0, 10.0));
    harness.input_mut().pointer.delta = Vec2::new(90.0, 0.0);
    harness.input_mut().pointer.primary = PointerButtonState::new(true, true, false);

    let pressed = run_press(&mut harness);
    assert!(pressed.state.active);
    assert!(!pressed.dragged);
    assert_eq!(harness.memory().drag_source(), None);

    harness.input_mut().events.clear();
    harness.input_mut().pointer.delta = Vec2::ZERO;
    harness.input_mut().pointer.primary = PointerButtonState::new(false, false, true);
    let released = run_press(&mut harness);
    assert!(released.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn same_frame_clipped_press_release_and_release_all_retain_ordered_cleanup() {
    let clip = ClipId::from_raw(96);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);

    let mut primary = UiTestHarness::new();
    primary.set_pointer_position(Point::new(10.0, 10.0));
    primary.pointer_press(MouseButton::Primary);
    primary.set_pointer_position(Point::new(50.0, 10.0));
    primary.pointer_release(MouseButton::Primary);
    let gesture = primary
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;
    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(1), SelectionGesturePhase::Press),
            (Some(3), SelectionGesturePhase::Cancel),
        ]
    );
    assert_eq!(primary.memory().pointer_capture(), None);

    let mut secondary = UiTestHarness::new();
    secondary.set_pointer_position(Point::new(10.0, 10.0));
    secondary.pointer_press(MouseButton::Secondary);
    secondary.set_pointer_position(Point::new(50.0, 10.0));
    secondary.pointer_release(MouseButton::Secondary);
    let response = secondary
        .run_frame(|ui| {
            let id = ui.id("press");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = pressable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        })
        .0;
    assert!(!response.secondary_clicked);
    assert_eq!(secondary.memory().secondary_pressed(), None);

    let mut release_all = UiTestHarness::new();
    release_all.set_pointer_position(Point::new(10.0, 10.0));
    release_all.pointer_press(MouseButton::Primary);
    release_all.set_pointer_position(Point::new(50.0, 10.0));
    release_all.input_mut().release_pointer_buttons();
    let gesture = release_all
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_selection_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;
    assert_eq!(gesture.actions.last().unwrap().ordinal, Some(3));
    assert_eq!(
        gesture.actions.last().unwrap().phase,
        SelectionGesturePhase::Cancel
    );
    assert_eq!(release_all.memory().pointer_capture(), None);
}

#[test]
fn clipped_drag_cleanup_cannot_publish_or_accept_a_drop_in_either_order() {
    let clip = ClipId::from_raw(97);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        let _ = harness.run_frame(|ui| {
            let id = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });
        harness.set_pointer_position(Point::new(14.0, 10.0));
        let _ = harness.run_frame(|ui| {
            let id = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });
        assert!(harness.memory().drag_source().is_some());

        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        let ((source_response, drop_response), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let pair = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, FULL, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, FULL, input, memory, false);
                (source, drop)
            };
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            pair
        });
        assert!(!source_response.clicked);
        assert!(!source_response.dragged);
        assert_eq!(drop_response.source, None);
        assert!(!drop_response.dropped);
        assert_eq!(harness.memory().drag_source(), None);
        assert_eq!(harness.memory().released_drag_source(), None);
    }
}

#[test]
fn external_unplanned_target_cannot_commit_a_cleanup_only_source_release() {
    let clip = ClipId::from_raw(104);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        let _ = harness.run_frame(|ui| {
            let source = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(source, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });
        harness.set_pointer_position(Point::new(14.0, 10.0));
        let _ = harness.run_frame(|ui| {
            let source = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(source, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);

        let ((source, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                (source, drop)
            } else {
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            }
        });
        assert!(!source.dragged);
        assert_eq!(drop.source, None);
        assert!(!drop.dropped);
    }
}

#[test]
fn planned_drop_rejects_release_outside_the_captured_source_clip() {
    let clip = ClipId::from_raw(98);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        let _ = harness.run_frame(|ui| {
            let source = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(source, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });
        harness.set_pointer_position(Point::new(14.0, 10.0));
        let _ = harness.run_frame(|ui| {
            let source = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = draggable(source, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        });

        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        let ((routes, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.with_clip(clip_rect, |plan| {
                        plan.target(
                            PointerTarget::new(source, FULL, PointerOrder::new(20))
                                .domain_drag_source(),
                        );
                    });
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(10))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid clipped-source plan");
            let drop = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let _ = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                drop
            } else {
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let _ = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                let (input, memory) = ui.input_and_memory_mut();
                drop_target(target, target_rect, input, memory, false)
            };
            (routes, drop)
        });
        assert_eq!(routes.drop, kinetik_ui_core::PointerRoute::Blocked);
        assert_eq!(drop.source, None);
        assert!(!drop.dropped);
        assert_eq!(harness.memory().released_drag_source(), None);
    }
}

#[test]
fn planned_active_drop_rejects_pointer_outside_captured_source_clip() {
    let clip = ClipId::from_raw(105);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = crossed_drag_harness();
        harness.set_pointer_position(Point::new(50.0, 10.0));
        let ((routes, source, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.with_clip(clip_rect, |plan| {
                        plan.target(
                            PointerTarget::new(source, FULL, PointerOrder::new(20))
                                .domain_drag_source(),
                        );
                    });
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid clipped active-source plan");
            let (source, drop) = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                (source, drop)
            } else {
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: clip_rect,
                });
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            };
            (routes, source, drop)
        });
        assert_eq!(routes.drop, kinetik_ui_core::PointerRoute::Blocked);
        assert!(!source.dragged);
        assert_eq!(drop.source, None);
        assert!(!drop.response.state.hovered);
        assert!(!drop.dropped);
    }
}

#[test]
fn planned_drop_uses_first_release_geometry_in_both_evaluation_orders() {
    let source_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = crossed_drag_harness();
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        harness.set_pointer_position(Point::new(100.0, 10.0));
        let ((routes, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect, PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(10))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid release-time plan");
            let drop = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let _ = draggable(source, source_rect, input, memory, false);
                drop
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let _ = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                drop_target(target, target_rect, input, memory, false)
            };
            (routes, drop)
        });
        assert_eq!(
            routes.drop,
            kinetik_ui_core::PointerRoute::Target(drop.response.id)
        );
        assert!(drop.dropped);
    }

    let mut outside_then_inside = crossed_drag_harness();
    outside_then_inside.set_pointer_position(Point::new(100.0, 10.0));
    outside_then_inside.pointer_release(MouseButton::Primary);
    outside_then_inside.set_pointer_position(Point::new(50.0, 10.0));
    outside_then_inside.pointer_press(MouseButton::Primary);
    outside_then_inside.pointer_release(MouseButton::Primary);
    let ((routes, drop), _) = outside_then_inside.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let routes = ui
            .resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, source_rect, PointerOrder::new(20))
                        .domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, target_rect, PointerOrder::new(10))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid first-release plan");
        let (input, memory) = ui.input_and_memory_mut();
        let _ = draggable(source, source_rect, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, target_rect, input, memory, false);
        (routes, drop)
    });
    assert_eq!(routes.drop, kinetik_ui_core::PointerRoute::Blocked);
    assert!(!drop.dropped);
    assert_eq!(drop.source, None);
}

#[test]
fn planned_drop_keeps_the_first_crossed_release_authoritative() {
    let source_rect = Rect::new(0.0, 0.0, 160.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = crossed_drag_harness();
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(Point::new(100.0, 10.0));
        harness.pointer_release(MouseButton::Primary);

        let ((source, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            ui.resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, source_rect, PointerOrder::new(20))
                        .domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, target_rect, PointerOrder::new(30))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid multi-transaction release plan");
            if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            }
        });
        assert!(drop.dropped);
        assert_eq!(drop.source, Some(source.id));
        assert_eq!(harness.memory().released_drag_source(), Some(source.id));
    }
}

#[test]
fn later_crossed_release_cannot_activate_a_below_threshold_plan() {
    let source_rect = Rect::new(0.0, 0.0, 80.0, 20.0);
    let target_rect = Rect::new(12.0, 0.0, 48.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(Point::new(13.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);

        let ((routes, source, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect, PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid below-threshold first transaction plan");
            let (source, drop) = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            };
            (routes, source, drop)
        });
        assert_eq!(
            routes.drop,
            kinetik_ui_core::PointerRoute::Target(drop.response.id)
        );
        assert!(source.dragged);
        assert!(!drop.dropped);
        assert_eq!(drop.source, None);
    }
}

#[test]
fn planned_release_crossing_threshold_is_target_first_safe() {
    let source_rect = Rect::new(0.0, 0.0, 80.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        let _ = run_drag(&mut harness);
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);

        let ((source, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            ui.resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, source_rect, PointerOrder::new(20))
                        .domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, target_rect, PointerOrder::new(30))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid release-crossing plan");
            if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            }
        });
        assert!(source.dragged);
        assert_eq!(source.drag_delta, Vec2::new(40.0, 0.0));
        assert!(drop.dropped);
        assert_eq!(drop.source, Some(source.id));
    }
}

#[test]
fn same_frame_planned_active_drag_is_target_first_safe_after_stray_release() {
    let source_rect = Rect::new(0.0, 0.0, 80.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for stray_release in [false, true] {
        for target_first in [false, true] {
            let mut harness = UiTestHarness::new();
            harness.set_pointer_position(Point::new(8.0, 10.0));
            if stray_release {
                harness.pointer_release(MouseButton::Primary);
            }
            harness.set_pointer_position(Point::new(10.0, 10.0));
            harness.pointer_press(MouseButton::Primary);
            harness.set_pointer_position(Point::new(50.0, 10.0));

            let ((source, drop), _) = harness.run_frame(|ui| {
                let source = ui.id("drag");
                let target = ui.id("drop");
                ui.resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect, PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid same-frame active drag plan");
                if target_first {
                    let (input, memory) = ui.input_and_memory_mut();
                    let drop = drop_target(target, target_rect, input, memory, false);
                    let (input, memory) = ui.input_and_memory_mut();
                    let source = draggable(source, source_rect, input, memory, false);
                    (source, drop)
                } else {
                    let (input, memory) = ui.input_and_memory_mut();
                    let source = draggable(source, source_rect, input, memory, false);
                    let (input, memory) = ui.input_and_memory_mut();
                    let drop = drop_target(target, target_rect, input, memory, false);
                    (source, drop)
                }
            });
            assert!(source.dragged);
            assert!(drop.response.state.hovered);
            assert_eq!(drop.source, Some(source.id));
            assert!(!drop.dropped);
        }
    }
}

#[test]
fn planned_target_first_probe_preserves_move_back_latch_and_source_transform() {
    let mut latched = UiTestHarness::new();
    latched.set_pointer_position(Point::new(10.0, 10.0));
    latched.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut latched);
    latched.set_pointer_position(Point::new(14.0, 10.0));
    latched.set_pointer_position(Point::new(11.0, 10.0));
    latched.pointer_release(MouseButton::Primary);
    let ((source, drop), _) = latched.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
            );
            plan.target(
                PointerTarget::new(target, FULL, PointerOrder::new(30))
                    .ordinary_owner(None)
                    .drop_owner(target),
            );
        })
        .expect("valid move-back plan");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (source, drop)
    });
    assert!(source.dragged);
    assert_eq!(source.drag_delta, Vec2::new(1.0, 0.0));
    assert!(drop.dropped);

    let transform = Transform::scale(Vec2::new(2.0, 2.0));
    let target_rect = Rect::new(24.0, 0.0, 8.0, 40.0);
    let mut transformed = UiTestHarness::new();
    transformed.set_pointer_position(Point::new(20.0, 20.0));
    transformed.pointer_press(MouseButton::Primary);
    let _ = transformed.run_frame(|ui| {
        let source = ui.id("drag");
        ui.push_primitive(Primitive::TransformBegin(transform));
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        ui.push_primitive(Primitive::TransformEnd);
        source
    });
    transformed.set_pointer_position(Point::new(28.0, 20.0));
    transformed.pointer_release(MouseButton::Primary);
    let ((source, drop), _) = transformed.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        ui.resolve_pointer_targets(|plan| {
            plan.with_transform(transform, |plan| {
                plan.target(
                    PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
                );
            });
            plan.target(
                PointerTarget::new(target, target_rect, PointerOrder::new(30))
                    .ordinary_owner(None)
                    .drop_owner(target),
            );
        })
        .expect("valid transformed source plan");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, target_rect, input, memory, false);
        ui.push_primitive(Primitive::TransformBegin(transform));
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        ui.push_primitive(Primitive::TransformEnd);
        (source, drop)
    });
    assert!(source.dragged);
    assert_eq!(source.drag_delta, Vec2::new(4.0, 0.0));
    assert!(drop.dropped);
    assert_eq!(drop.source, Some(source.id));
}

#[test]
fn same_frame_planned_drag_uses_press_and_release_probes_not_final_snapshot() {
    let source_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);
        harness.set_pointer_position(Point::new(100.0, 10.0));

        let ((source, drop), output) = harness.run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            let wheel = ui.id("wheel");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect, PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                    plan.target(PointerTarget::wheel_only(
                        wheel,
                        Rect::new(90.0, 0.0, 20.0, 20.0),
                        PointerOrder::new(40),
                    ));
                })
                .expect("valid same-frame drag plan");
            assert_eq!(
                routes.ordinary,
                kinetik_ui_core::PointerRoute::Target(source)
            );
            assert_eq!(routes.wheel, kinetik_ui_core::PointerRoute::Target(wheel));
            assert!(!ui.memory().pointer_interaction_cancelled());
            if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            }
        });
        assert!(
            !source.dragged,
            "target_first={target_first}, warnings={:?}",
            output.warnings
        );
        assert_eq!(source.drag_delta, Vec2::ZERO);
        assert!(drop.dropped);
        assert_eq!(drop.source, Some(source.id));
        assert_eq!(harness.memory().released_drag_source(), Some(source.id));
    }

    let mut pre_release = UiTestHarness::new();
    pre_release.set_pointer_position(Point::new(50.0, 10.0));
    pre_release.pointer_release(MouseButton::Primary);
    pre_release.set_pointer_position(Point::new(10.0, 10.0));
    pre_release.pointer_press(MouseButton::Primary);
    pre_release.set_pointer_position(Point::new(50.0, 10.0));
    pre_release.pointer_release(MouseButton::Primary);
    let ((source, drop), output) = pre_release.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(source, source_rect, PointerOrder::new(20)).domain_drag_source(),
            );
            plan.target(
                PointerTarget::new(target, target_rect, PointerOrder::new(30))
                    .ordinary_owner(None)
                    .drop_owner(target),
            );
        })
        .expect("valid post-press release plan");
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, source_rect, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, target_rect, input, memory, false);
        (source, drop)
    });
    assert!(output.warnings.is_empty());
    assert!(drop.dropped);
    assert_eq!(drop.source, Some(source.id));
}

#[test]
fn ordered_drag_termination_preserves_prior_output_and_fences_later_input() {
    let mut release_then_focus = crossed_drag_harness();
    release_then_focus.pointer_release(MouseButton::Primary);
    release_then_focus
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    let ((source, drop), _) = release_then_focus.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
            );
            plan.target(
                PointerTarget::new(target, FULL, PointerOrder::new(30))
                    .ordinary_owner(None)
                    .drop_owner(target),
            );
        })
        .expect("valid causal release plan");
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        (source, drop)
    });
    assert!(!source.clicked);
    assert!(drop.dropped);

    let mut focus_then_release = crossed_drag_harness();
    focus_then_release
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    focus_then_release.pointer_release(MouseButton::Primary);
    let drop = focus_then_release
        .run_frame(|ui| {
            let source = ui.id("drag");
            let target = ui.id("drop");
            ui.resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, FULL, PointerOrder::new(30))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid cancelled release plan");
            let (input, memory) = ui.input_and_memory_mut();
            let _ = draggable(source, FULL, input, memory, false);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, FULL, input, memory, false)
        })
        .0;
    assert!(!drop.dropped);
    assert_eq!(drop.source, None);

    let mut movement_then_cancel = UiTestHarness::new();
    movement_then_cancel.set_pointer_position(Point::new(10.0, 10.0));
    movement_then_cancel.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut movement_then_cancel);
    movement_then_cancel.set_pointer_position(Point::new(14.0, 10.0));
    movement_then_cancel.input_mut().release_pointer_buttons();
    let response = run_drag(&mut movement_then_cancel);
    assert!(response.dragged);
    assert_eq!(response.drag_delta, Vec2::new(4.0, 0.0));
    assert_eq!(movement_then_cancel.memory().drag_source(), None);
    assert_eq!(movement_then_cancel.memory().released_drag_source(), None);
}

#[test]
fn unrelated_behavior_cannot_erase_owner_output_before_a_later_fence() {
    let mut movement = crossed_drag_harness();
    movement.set_pointer_position(Point::new(17.0, 10.0));
    movement
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    let ((unrelated, source), _) = movement.run_frame(|ui| {
        let source = ui.id("drag");
        let unrelated = ui.id("unrelated");
        let (input, memory) = ui.input_and_memory_mut();
        let unrelated = pressable(unrelated, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (unrelated, source)
    });
    assert!(!unrelated.clicked);
    assert!(source.dragged);
    assert_eq!(source.drag_delta, Vec2::new(3.0, 0.0));
    assert!(movement.memory().pointer_interaction_cancelled());

    let mut released = crossed_drag_harness();
    released.set_pointer_position(Point::new(10.0, 10.0));
    released.pointer_release(MouseButton::Primary);
    released
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    let ((source, drop), _) = released.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let unrelated = ui.id("unrelated");
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
            );
            plan.target(
                PointerTarget::new(target, FULL, PointerOrder::new(30))
                    .ordinary_owner(None)
                    .drop_owner(target),
            );
        })
        .expect("valid pre-fence release plan");
        let (input, memory) = ui.input_and_memory_mut();
        let _ = pressable(unrelated, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (source, drop)
    });
    assert!(!source.clicked);
    assert!(drop.dropped);
    assert_eq!(drop.source, Some(source.id));
}

#[test]
fn split_button_owners_each_preserve_pre_fence_primary_output() {
    let secondary = WidgetId::from_key("root").child("secondary");

    for secondary_first in [false, true] {
        let mut movement = crossed_drag_harness();
        movement.memory_mut().press_secondary(secondary);
        movement.set_pointer_position(Point::new(17.0, 10.0));
        movement
            .input_mut()
            .push_event(UiInputEvent::WindowFocusChanged(false));
        let ((secondary_response, source, cursor_published), _) = movement.run_frame(|ui| {
            let source = ui.id("drag");
            ui.register_id(secondary);
            if secondary_first {
                let (input, memory) = ui.input_and_memory_mut();
                let secondary_response = pressable(secondary, FULL, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                let cursor_published = ui.request_cursor_for(source.id, CursorShape::Grabbing);
                (secondary_response, source, cursor_published)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, FULL, input, memory, false);
                let cursor_published = ui.request_cursor_for(source.id, CursorShape::Grabbing);
                let (input, memory) = ui.input_and_memory_mut();
                let secondary_response = pressable(secondary, FULL, input, memory, false);
                (secondary_response, source, cursor_published)
            }
        });
        assert!(!secondary_response.secondary_clicked);
        assert!(source.dragged);
        assert_eq!(source.drag_delta, Vec2::new(3.0, 0.0));
        assert!(!source.state.hovered);
        assert!(!cursor_published);
        assert_eq!(movement.memory().secondary_pressed(), None);
        assert!(movement.memory().pointer_interaction_cancelled());
    }

    let mut released = crossed_drag_harness();
    released.memory_mut().press_secondary(secondary);
    released.set_pointer_position(Point::new(50.0, 10.0));
    released.pointer_release(MouseButton::Primary);
    released
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    let source = released
        .run_frame(|ui| {
            let source = ui.id("drag");
            ui.register_id(secondary);
            let (input, memory) = ui.input_and_memory_mut();
            let _ = pressable(secondary, FULL, input, memory, false);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(source, FULL, input, memory, false)
        })
        .0;
    assert_eq!(released.memory().released_drag_source(), Some(source.id));
    assert_eq!(released.memory().secondary_pressed(), None);
    assert!(released.memory().pointer_interaction_cancelled());
}

#[test]
fn owner_mismatch_blocks_same_frame_planned_drop_in_both_orders() {
    let stale_owner = WidgetId::from_key("root").child("stale-owner");
    let source_rect = Rect::new(0.0, 0.0, 80.0, 20.0);
    let target_rect = Rect::new(40.0, 0.0, 20.0, 20.0);

    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.memory_mut().press_secondary(stale_owner);
        harness.set_pointer_position(Point::new(10.0, 10.0));
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(Point::new(50.0, 10.0));
        harness.pointer_release(MouseButton::Primary);

        let ((routes, source, drop), _) = harness.run_frame(|ui| {
            ui.register_id(stale_owner);
            let source = ui.id("drag");
            let target = ui.id("drop");
            let routes = ui
                .resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect, PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect, PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid owner-mismatch plan");
            let (source, drop) = if target_first {
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                (source, drop)
            } else {
                let (input, memory) = ui.input_and_memory_mut();
                let source = draggable(source, source_rect, input, memory, false);
                let (input, memory) = ui.input_and_memory_mut();
                let drop = drop_target(target, target_rect, input, memory, false);
                (source, drop)
            };
            (routes, source, drop)
        });
        assert_eq!(routes.drop, kinetik_ui_core::PointerRoute::Blocked);
        assert!(!source.dragged);
        assert!(!drop.dropped);
        assert_eq!(drop.source, None);
        assert_eq!(harness.memory().secondary_pressed(), None);
        assert!(harness.memory().pointer_interaction_cancelled());
    }
}

#[test]
fn global_fences_survive_clips_and_wheel_consumes_only_pre_fence_events() {
    let clip = ClipId::from_raw(103);
    let mut clipped = UiTestHarness::new();
    clipped.set_pointer_position(Point::new(50.0, 10.0));
    clipped.input_mut().release_pointer_buttons();
    clipped.set_pointer_position(Point::new(10.0, 10.0));
    clipped.pointer_press(MouseButton::Primary);
    clipped.pointer_release(MouseButton::Primary);
    let response = clipped
        .run_frame(|ui| {
            let id = ui.id("press");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            });
            let (input, memory) = ui.input_and_memory_mut();
            let response = pressable(id, FULL, input, memory, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            response
        })
        .0;
    assert!(!response.clicked);
    assert!(!response.state.active);
    assert!(clipped.memory().pointer_interaction_cancelled());

    let mut wheel = UiTestHarness::new();
    wheel.set_pointer_position(Point::new(10.0, 10.0));
    wheel.wheel(Vec2::new(0.0, -20.0));
    wheel
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    wheel.wheel(Vec2::new(0.0, -30.0));
    let scroll = wheel
        .run_frame(|ui| {
            let unrelated = ui.id("unrelated");
            let scroll = ui.id("scroll");
            let (input, memory) = ui.input_and_memory_mut();
            let _ = pressable(unrelated, FULL, input, memory, false);
            let (input, memory) = ui.input_and_memory_mut();
            scrollable(scroll, FULL, Size::new(320.0, 320.0), input, memory, false)
        })
        .0;
    assert_eq!(scroll.delta, Vec2::new(0.0, 20.0));
    assert!(!scroll.response.state.hovered);
}

#[test]
fn no_owner_focus_loss_suppresses_passive_hover_but_keeps_pre_fence_wheel() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.wheel(Vec2::new(0.0, -20.0));
    harness
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));

    let ((tooltip, scroll), _) = harness.run_frame(|ui| {
        let tooltip = ui.id("tooltip");
        let scroll = ui.id("scroll");
        let (input, memory) = ui.input_and_memory_mut();
        let tooltip = tooltip_trigger(tooltip, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let scroll = scrollable(scroll, FULL, Size::new(320.0, 320.0), input, memory, false);
        (tooltip, scroll)
    });
    assert!(!tooltip.state.hovered);
    assert!(!tooltip.tooltip_requested);
    assert!(!scroll.response.state.hovered);
    assert_eq!(scroll.delta, Vec2::new(0.0, 20.0));
}

#[test]
fn focus_loss_fences_secondary_and_future_pointer_events_with_causal_selection_data() {
    let mut secondary = UiTestHarness::new();
    secondary.set_pointer_position(Point::new(10.0, 10.0));
    secondary.pointer_press(MouseButton::Secondary);
    let _ = run_press(&mut secondary);
    secondary
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    secondary.pointer_release(MouseButton::Secondary);
    let response = run_press(&mut secondary);
    assert!(!response.secondary_clicked);
    assert_eq!(secondary.memory().secondary_pressed(), None);
    assert!(secondary.memory().pointer_interaction_cancelled());

    let mut no_owner = UiTestHarness::new();
    no_owner.set_pointer_position(Point::new(10.0, 10.0));
    let _ = run_press(&mut no_owner);
    no_owner
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    no_owner.pointer_press(MouseButton::Primary);
    no_owner.pointer_release(MouseButton::Primary);
    assert!(!run_press(&mut no_owner).clicked);

    let mut selection = UiTestHarness::new();
    selection.set_pointer_position(Point::new(10.0, 10.0));
    selection.pointer_press(MouseButton::Primary);
    let _ = selection.run_frame(|ui| {
        let id = ui.id("selection");
        ui.captured_selection_gesture(id, FULL, false)
    });
    selection.set_pointer_position(Point::new(12.0, 10.0));
    selection
        .input_mut()
        .push_event(UiInputEvent::WindowFocusChanged(false));
    selection.set_pointer_position(Point::new(80.0, 10.0));
    selection.pointer_press(MouseButton::Secondary);
    selection.set_click_count(2);
    let gesture = selection
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(gesture.actions.len(), 2);
    assert_eq!(gesture.actions[0].phase, SelectionGesturePhase::Move);
    assert_eq!(gesture.actions[0].position, Some(Point::new(12.0, 10.0)));
    assert_eq!(gesture.actions[0].click_count, 0);
    assert_eq!(gesture.actions[1].phase, SelectionGesturePhase::Cancel);
    assert_eq!(gesture.actions[1].position, None);
    assert_eq!(gesture.actions[1].click_count, 0);
}

#[test]
fn conflicted_selection_release_is_cancel_only() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("selection");
        ui.captured_selection_gesture(id, FULL, false)
    });

    harness.pointer_release(MouseButton::Primary);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(gesture.actions.len(), 1);
    assert_eq!(gesture.actions[0].phase, SelectionGesturePhase::Cancel);
    assert!(!gesture.response.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn conflicting_snapshot_only_focus_loss_cannot_invent_an_ordered_cancel() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let owner = run_press(&mut harness).id;
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness.input_mut().window_focused = false;

    let response = run_press(&mut harness);
    assert!(!response.clicked);
    assert_eq!(harness.memory().pointer_capture(), Some(owner));
    assert!(!harness.memory().pointer_interaction_cancelled());
}

#[test]
fn root_conflict_blocks_tooltip_and_scroll_hover_without_discarding_canonical_wheel() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.wheel_pixels(Vec2::new(0.0, -20.0));
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);

    let ((tooltip, scroll), output) = harness.run_frame(|ui| {
        let tooltip_id = ui.id("tooltip");
        let scroll_id = ui.id("scroll");
        let (input, memory) = ui.input_and_memory_mut();
        let tooltip = tooltip_trigger(tooltip_id, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let scroll = scrollable(
            scroll_id,
            FULL,
            Size::new(320.0, 320.0),
            input,
            memory,
            false,
        );
        (tooltip, scroll)
    });
    assert!(!tooltip.state.hovered);
    assert!(!tooltip.tooltip_requested);
    assert!(!scroll.response.state.hovered);
    assert_eq!(scroll.delta, Vec2::new(0.0, 20.0));
    assert_eq!(output.warnings.len(), 1);
}

#[test]
fn canonical_unplanned_drop_commit_fails_closed_at_every_threshold() {
    let mut below = UiTestHarness::new();
    below.set_pointer_position(Point::new(10.0, 10.0));
    below.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut below);
    below.set_pointer_position(Point::new(13.0, 10.0));
    let _ = run_drag(&mut below);
    below.pointer_release(MouseButton::Primary);
    let ((drop, source), _) = below.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (drop, source)
    });
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);
    assert!(source.clicked);

    let mut crossed = UiTestHarness::new();
    crossed.set_pointer_position(Point::new(10.0, 10.0));
    crossed.pointer_press(MouseButton::Primary);
    let _ = run_drag(&mut crossed);
    crossed.set_pointer_position(Point::new(14.0, 10.0));
    let _ = run_drag(&mut crossed);
    crossed.pointer_release(MouseButton::Primary);
    let ((drop, source), _) = crossed.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let source = draggable(source, FULL, input, memory, false);
        (drop, source)
    });
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);
    assert!(!source.clicked);
}

#[test]
fn drop_uses_canonical_release_geometry_and_rejects_missing_event_position() {
    let source = kinetik_ui_core::WidgetId::from_key("source");
    let target = kinetik_ui_core::WidgetId::from_key("target");

    let mut missing = UiTestHarness::new();
    missing.memory_mut().capture_pointer(source);
    missing.memory_mut().activate(source);
    missing.memory_mut().press(source);
    missing.memory_mut().start_drag(source);
    missing.input_mut().pointer.position = Some(Point::new(10.0, 10.0));
    missing.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: None,
    });
    let drop = missing
        .run_frame(|ui| {
            ui.register_id(source);
            ui.register_id(target);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, FULL, input, memory, false)
        })
        .0;
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);

    let mut ordered = UiTestHarness::new();
    ordered.memory_mut().capture_pointer(source);
    ordered.memory_mut().activate(source);
    ordered.memory_mut().press(source);
    ordered.memory_mut().start_drag(source);
    ordered.input_mut().pointer.position = Some(Point::new(10.0, 10.0));
    ordered.input_mut().pointer.primary.down = true;
    ordered.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(10.0, 10.0)),
    });
    ordered.input_mut().push_event(UiInputEvent::PointerMoved {
        position: Point::new(200.0, 10.0),
        delta: Vec2::new(190.0, 0.0),
    });
    let drop = ordered
        .run_frame(|ui| {
            ui.register_id(source);
            ui.register_id(target);
            ui.resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, FULL, PointerOrder::new(20)).domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, FULL, PointerOrder::new(30))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid canonical geometry plan");
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, FULL, input, memory, false)
        })
        .0;
    assert_eq!(drop.source, Some(source));
    assert!(drop.dropped);
    assert!(drop.response.state.hovered);
}

#[test]
fn conflicted_release_cleans_existing_capture_without_click_drag_or_drop() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let pressed = run_drag(&mut harness);
    assert_eq!(harness.memory().pointer_capture(), Some(pressed.id));

    harness.pointer_release(MouseButton::Primary);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let ((response, drop), output) = harness.run_frame(|ui| {
        let source = ui.id("drag");
        let target = ui.id("drop");
        let (input, memory) = ui.input_and_memory_mut();
        let response = draggable(source, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let drop = drop_target(target, FULL, input, memory, false);
        (response, drop)
    });

    assert!(!response.clicked);
    assert!(!response.dragged);
    assert_eq!(drop.source, None);
    assert!(!drop.dropped);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().released_drag_source(), None);
    assert_eq!(output.warnings.len(), 1);
}

#[test]
fn plain_pointer_capture_release_cleans_without_synthesizing_a_click() {
    let mut harness = UiTestHarness::new();
    let owner = kinetik_ui_core::WidgetId::from_key("plain-capture");
    harness.memory_mut().capture_pointer(owner);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let response = harness
        .run_frame(|ui| {
            ui.register_id(owner);
            let (input, memory) = ui.input_and_memory_mut();
            pressable(owner, FULL, input, memory, false)
        })
        .0;
    assert!(!response.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn canonical_button_without_event_position_never_uses_the_final_snapshot_position() {
    let mut missing_press = UiTestHarness::new();
    missing_press.set_pointer_position(Point::new(10.0, 10.0));
    missing_press
        .input_mut()
        .push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: None,
        });
    let response = run_press(&mut missing_press);
    assert!(!response.state.active);
    assert_eq!(missing_press.memory().pointer_capture(), None);

    let mut missing_release = UiTestHarness::new();
    missing_release.set_pointer_position(Point::new(10.0, 10.0));
    missing_release.pointer_press(MouseButton::Primary);
    let pressed = run_press(&mut missing_release);
    assert_eq!(missing_release.memory().pointer_capture(), Some(pressed.id));
    missing_release
        .input_mut()
        .push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            click_count: 1,
            position: None,
        });
    let response = run_press(&mut missing_release);
    assert!(!response.clicked);
    assert_eq!(missing_release.memory().pointer_capture(), None);
}
