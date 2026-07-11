//! Same-frame primary-transaction routing conformance for captured selection.

use kinetik_ui_core::{
    CapturedSelectionGesture, ClipId, DomainDragGesturePhase, MouseButton, Point, PointerOrder,
    PointerRoute, PointerTarget, Primitive, Rect, SelectionGesturePhase, UiInput, UiInputEvent,
    UiMemory, UiTestHarness, Vec2, drop_target, pressable,
};

const A_RECT: Rect = Rect::new(0.0, 0.0, 40.0, 40.0);
const B_RECT: Rect = Rect::new(100.0, 0.0, 40.0, 40.0);
const DROP_RECT: Rect = Rect::new(40.0, 0.0, 30.0, 40.0);
const A_POINT: Point = Point::new(10.0, 10.0);
const B_POINT: Point = Point::new(110.0, 10.0);

fn queue_completed_a_to_b(harness: &mut UiTestHarness) {
    harness.set_pointer_position(A_POINT);
    harness.pointer_press(MouseButton::Primary);
    harness.pointer_release(MouseButton::Primary);
    harness.set_pointer_position(B_POINT);
    harness.pointer_press(MouseButton::Primary);
}

fn run_pair(
    harness: &mut UiTestHarness,
    b_first: bool,
) -> (CapturedSelectionGesture, CapturedSelectionGesture) {
    harness
        .run_frame(|ui| {
            let a = ui.id("selection-a");
            let b = ui.id("selection-b");
            if b_first {
                let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
                let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
                (a_gesture, b_gesture)
            } else {
                let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
                let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
                (a_gesture, b_gesture)
            }
        })
        .0
}

fn action_trace(gesture: &CapturedSelectionGesture) -> Vec<(Option<usize>, SelectionGesturePhase)> {
    gesture
        .actions
        .iter()
        .map(|action| (action.ordinal, action.phase))
        .collect()
}

fn push_primary(harness: &mut UiTestHarness, down: bool, position: Option<Point>) {
    harness.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down,
        click_count: 1,
        position,
    });
}

fn begin_captured_selection(harness: &mut UiTestHarness) {
    harness.set_pointer_position(A_POINT);
    harness.pointer_press(MouseButton::Primary);
    let gesture = harness
        .run_frame(|ui| {
            let owner = ui.id("selection-a");
            ui.captured_selection_gesture(owner, A_RECT, false)
        })
        .0;
    assert_eq!(
        action_trace(&gesture),
        vec![(Some(1), SelectionGesturePhase::Press)]
    );
}

#[test]
fn completed_transactions_transfer_to_b_in_both_evaluation_orders() {
    for b_first in [false, true] {
        let mut harness = UiTestHarness::new();
        queue_completed_a_to_b(&mut harness);

        let (a_gesture, b_gesture) = run_pair(&mut harness, b_first);
        assert_eq!(
            action_trace(&b_gesture),
            vec![(Some(4), SelectionGesturePhase::Press)]
        );
        if b_first {
            assert!(a_gesture.actions.is_empty());
        } else {
            assert_eq!(
                action_trace(&a_gesture),
                vec![
                    (Some(1), SelectionGesturePhase::Press),
                    (Some(2), SelectionGesturePhase::Release),
                ]
            );
        }
        assert_eq!(
            harness.memory().pointer_capture(),
            Some(b_gesture.response.id)
        );
    }
}

#[test]
fn unreleased_second_press_fails_closed_in_both_evaluation_orders() {
    for b_first in [false, true] {
        let mut harness = UiTestHarness::new();
        harness.set_pointer_position(A_POINT);
        harness.pointer_press(MouseButton::Primary);
        harness.set_pointer_position(B_POINT);
        harness.pointer_press(MouseButton::Primary);

        let (a_gesture, b_gesture) = run_pair(&mut harness, b_first);
        assert!(b_gesture.actions.is_empty());
        assert_eq!(a_gesture.actions[0].phase, SelectionGesturePhase::Press);
        assert_eq!(
            harness.memory().pointer_capture(),
            Some(a_gesture.response.id)
        );
    }
}

#[test]
fn unmatched_first_press_blocks_later_hit_even_without_a_position_or_target() {
    for b_first in [false, true] {
        for first_position in [None, Some(Point::new(240.0, 10.0))] {
            let mut harness = UiTestHarness::new();
            if let Some(position) = first_position {
                harness.set_pointer_position(position);
                harness.pointer_press(MouseButton::Primary);
            } else {
                harness.input_mut().push_event(UiInputEvent::PointerButton {
                    button: MouseButton::Primary,
                    down: true,
                    click_count: 1,
                    position: None,
                });
            }
            harness.set_pointer_position(B_POINT);
            harness.pointer_press(MouseButton::Primary);

            let (a_gesture, b_gesture) = run_pair(&mut harness, b_first);
            assert!(a_gesture.actions.is_empty());
            assert!(b_gesture.actions.is_empty());
            assert_eq!(harness.memory().pointer_capture(), None);
        }
    }
}

#[test]
fn spatially_suppressed_unreleased_press_still_blocks_later_in_clip_press() {
    for first_position in [None, Some(A_POINT)] {
        let mut harness = UiTestHarness::new();
        push_primary(&mut harness, true, first_position);
        push_primary(&mut harness, true, Some(B_POINT));

        let gesture = harness
            .run_frame(|ui| {
                let b = ui.id("selection-b");
                let clip = ClipId::from_raw(91);
                ui.push_primitive(Primitive::ClipBegin {
                    id: clip,
                    rect: B_RECT,
                });
                let gesture = ui.captured_selection_gesture(b, B_RECT, false);
                ui.push_primitive(Primitive::ClipEnd { id: clip });
                gesture
            })
            .0;

        assert!(gesture.actions.is_empty());
        assert_eq!(harness.memory().pointer_capture(), None);
    }
}

#[test]
fn spatially_suppressed_completed_transaction_allows_later_in_clip_press() {
    let mut harness = UiTestHarness::new();
    push_primary(&mut harness, true, Some(A_POINT));
    push_primary(&mut harness, false, Some(A_POINT));
    push_primary(&mut harness, true, Some(B_POINT));

    let gesture = harness
        .run_frame(|ui| {
            let b = ui.id("selection-b");
            let clip = ClipId::from_raw(92);
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: B_RECT,
            });
            let gesture = ui.captured_selection_gesture(b, B_RECT, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;

    assert_eq!(
        action_trace(&gesture),
        vec![(Some(2), SelectionGesturePhase::Press)]
    );
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(gesture.response.id)
    );
}

#[test]
fn standalone_primary_transaction_fallback_allows_release_but_blocks_duplicate_press() {
    let a = kinetik_ui_core::WidgetId::from_key("standalone-a");
    let b = kinetik_ui_core::WidgetId::from_key("standalone-b");

    let mut completed_input = UiInput::default();
    completed_input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(A_POINT),
    });
    completed_input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(A_POINT),
    });
    completed_input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(B_POINT),
    });
    let mut completed_memory = UiMemory::new();
    let _ = pressable(a, A_RECT, &completed_input, &mut completed_memory, false);
    let _ = pressable(b, B_RECT, &completed_input, &mut completed_memory, false);
    assert_eq!(completed_memory.pointer_capture(), Some(b));

    let mut duplicate_input = UiInput::default();
    duplicate_input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(A_POINT),
    });
    duplicate_input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(B_POINT),
    });
    let mut duplicate_memory = UiMemory::new();
    let _ = pressable(a, A_RECT, &duplicate_input, &mut duplicate_memory, false);
    let _ = pressable(b, B_RECT, &duplicate_input, &mut duplicate_memory, false);
    assert_eq!(duplicate_memory.pointer_capture(), Some(a));
}

#[test]
fn same_owner_repeated_transactions_preserve_root_order() {
    let mut harness = UiTestHarness::new();
    push_primary(&mut harness, true, Some(A_POINT));
    push_primary(&mut harness, false, Some(A_POINT));
    push_primary(&mut harness, true, Some(A_POINT));
    push_primary(&mut harness, false, Some(A_POINT));

    let (gesture, competing) = harness
        .run_frame(|ui| {
            let owner = ui.id("selection-a");
            let competing = ui.id("selection-b");
            (
                ui.captured_selection_gesture(owner, A_RECT, false),
                ui.captured_selection_gesture(competing, A_RECT, false),
            )
        })
        .0;
    assert_eq!(
        action_trace(&gesture),
        vec![
            (Some(0), SelectionGesturePhase::Press),
            (Some(1), SelectionGesturePhase::Release),
            (Some(2), SelectionGesturePhase::Press),
            (Some(3), SelectionGesturePhase::Release),
        ]
    );
    assert!(competing.actions.is_empty());
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn explicit_plan_frozen_to_first_transaction_never_routes_later_b_press() {
    for b_first in [false, true] {
        let mut harness = UiTestHarness::new();
        queue_completed_a_to_b(&mut harness);

        let ((route, a_gesture, b_gesture), _) = harness.run_frame(|ui| {
            let a = ui.id("selection-a");
            let b = ui.id("selection-b");
            let route = ui
                .resolve_pointer_targets(|plan| {
                    plan.target(PointerTarget::new(a, A_RECT, PointerOrder::new(10)));
                    plan.target(PointerTarget::new(b, B_RECT, PointerOrder::new(20)));
                })
                .expect("valid two-field plan")
                .ordinary;
            let (a_gesture, b_gesture) = if b_first {
                let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
                let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
                (a_gesture, b_gesture)
            } else {
                let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
                let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
                (a_gesture, b_gesture)
            };
            (route, a_gesture, b_gesture)
        });

        assert_eq!(route, PointerRoute::Target(a_gesture.response.id));
        assert!(b_gesture.actions.is_empty());
        assert_eq!(harness.memory().pointer_capture(), None);

        let mut blocked = UiTestHarness::new();
        queue_completed_a_to_b(&mut blocked);
        let ((route, a_gesture, b_gesture), _) = blocked.run_frame(|ui| {
            let a = ui.id("selection-a");
            let b = ui.id("selection-b");
            let route = ui
                .resolve_pointer_targets(|plan| {
                    plan.blocker(A_RECT, PointerOrder::new(10));
                })
                .expect("valid blocked plan")
                .ordinary;
            let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
            let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
            (route, a_gesture, b_gesture)
        });
        assert_eq!(route, PointerRoute::Blocked);
        assert!(a_gesture.actions.is_empty());
        assert!(b_gesture.actions.is_empty());
        assert_eq!(blocked.memory().pointer_capture(), None);
    }
}

#[test]
fn outside_clip_cleanup_release_allows_only_the_later_b_press() {
    let mut harness = UiTestHarness::new();
    begin_captured_selection(&mut harness);
    harness.set_pointer_position(B_POINT);
    harness.pointer_release(MouseButton::Primary);
    harness.pointer_press(MouseButton::Primary);

    let ((a_gesture, b_gesture), _) = harness.run_frame(|ui| {
        let a = ui.id("selection-a");
        let b = ui.id("selection-b");
        let clip = ClipId::from_raw(73);
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: A_RECT,
        });
        let a_gesture = ui.captured_selection_gesture(a, A_RECT, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        let b_gesture = ui.captured_selection_gesture(b, B_RECT, false);
        (a_gesture, b_gesture)
    });

    assert_eq!(
        action_trace(&a_gesture),
        vec![(Some(1), SelectionGesturePhase::Cancel)]
    );
    assert_eq!(
        action_trace(&b_gesture),
        vec![(Some(2), SelectionGesturePhase::Press)]
    );
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(b_gesture.response.id)
    );
}

#[test]
fn cancellation_fences_do_not_authorize_a_later_press() {
    for focus_loss in [false, true] {
        let mut harness = UiTestHarness::new();
        begin_captured_selection(&mut harness);
        if focus_loss {
            harness.set_window_focused(false);
        } else {
            harness.input_mut().release_pointer_buttons();
        }
        harness.set_pointer_position(B_POINT);
        harness.pointer_press(MouseButton::Primary);

        let (a_gesture, b_gesture) = run_pair(&mut harness, false);
        assert_eq!(
            action_trace(&a_gesture),
            vec![(Some(0), SelectionGesturePhase::Cancel)]
        );
        assert!(b_gesture.actions.is_empty());
        assert_eq!(harness.memory().pointer_capture(), None);
    }
}

#[test]
fn conflicted_release_does_not_create_transfer_authority() {
    let mut harness = UiTestHarness::new();
    begin_captured_selection(&mut harness);
    harness.pointer_release(MouseButton::Primary);
    harness.set_pointer_position(B_POINT);
    harness.pointer_press(MouseButton::Primary);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);

    let (a_gesture, b_gesture) = run_pair(&mut harness, false);
    assert_eq!(
        action_trace(&a_gesture),
        vec![(Some(0), SelectionGesturePhase::Cancel)]
    );
    assert!(b_gesture.actions.is_empty());
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn released_owner_provenance_resets_at_the_next_frame() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(A_POINT);
    harness.pointer_press(MouseButton::Primary);
    harness.pointer_release(MouseButton::Primary);
    let a_gesture = harness
        .run_frame(|ui| {
            let a = ui.id("selection-a");
            ui.captured_selection_gesture(a, A_RECT, false)
        })
        .0;
    assert_eq!(a_gesture.actions.len(), 2);

    harness.set_pointer_position(B_POINT);
    harness.pointer_press(MouseButton::Primary);
    let b_gesture = harness
        .run_frame(|ui| {
            let b = ui.id("selection-b");
            ui.captured_selection_gesture(b, B_RECT, false)
        })
        .0;
    assert_eq!(
        action_trace(&b_gesture),
        vec![(Some(1), SelectionGesturePhase::Press)]
    );
    assert_eq!(
        harness.memory().pointer_capture(),
        Some(b_gesture.response.id)
    );
}

#[test]
fn unplanned_domain_drag_keeps_first_release_after_later_selection_transaction() {
    let source_rect = Rect::new(0.0, 0.0, 35.0, 40.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(A_POINT);
    harness.pointer_press(MouseButton::Primary);
    harness.set_pointer_position(Point::new(20.0, 10.0));
    let source = harness
        .run_frame(|ui| {
            let source = ui.id("drag-source");
            let gesture = ui.captured_domain_drag_gesture(source, source_rect, false);
            assert!(gesture.response.dragged);
            source
        })
        .0;

    harness.set_pointer_position(Point::new(50.0, 10.0));
    harness.pointer_release(MouseButton::Primary);
    harness.set_pointer_position(B_POINT);
    harness.pointer_press(MouseButton::Primary);
    harness.pointer_release(MouseButton::Primary);
    let ((drag, selection, drop), _) = harness.run_frame(|ui| {
        let source = ui.id("drag-source");
        let selection_owner = ui.id("selection-b");
        let drop_owner = ui.id("drop-target");
        let drag = ui.captured_domain_drag_gesture(source, source_rect, false);
        let selection = ui.captured_selection_gesture(selection_owner, B_RECT, false);
        let drop = {
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(drop_owner, DROP_RECT, input, memory, false)
        };
        (drag, selection, drop)
    });

    assert_eq!(
        drag.actions
            .iter()
            .map(|action| (action.ordinal, action.phase))
            .collect::<Vec<_>>(),
        vec![
            (Some(0), DomainDragGesturePhase::Move),
            (Some(1), DomainDragGesturePhase::Release),
        ]
    );
    assert_eq!(
        action_trace(&selection),
        vec![
            (Some(3), SelectionGesturePhase::Press),
            (Some(4), SelectionGesturePhase::Release),
        ]
    );
    assert!(!drop.dropped);
    assert_eq!(drop.source, None);
    assert_eq!(harness.memory().released_drag_source(), Some(source));
}

#[test]
fn planned_domain_drop_keeps_first_release_after_later_primary_release() {
    let source_rect = Rect::new(0.0, 0.0, 35.0, 40.0);
    for target_first in [false, true] {
        let mut harness = UiTestHarness::new();
        push_primary(&mut harness, true, Some(A_POINT));
        harness.input_mut().push_event(UiInputEvent::PointerMoved {
            position: Point::new(20.0, 10.0),
            delta: Vec2::new(10.0, 0.0),
        });
        push_primary(&mut harness, false, Some(Point::new(50.0, 10.0)));
        push_primary(&mut harness, true, Some(A_POINT));
        push_primary(&mut harness, false, Some(A_POINT));

        let ((source, drag, drop), _) = harness.run_frame(|ui| {
            let source = ui.id("drag-source");
            let target = ui.id("drop-target");
            ui.resolve_pointer_targets(|plan| {
                plan.target(
                    PointerTarget::new(source, source_rect, PointerOrder::new(10))
                        .domain_drag_source(),
                );
                plan.target(
                    PointerTarget::new(target, DROP_RECT, PointerOrder::new(20))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid first-release domain-drag plan");
            let (drag, drop) = if target_first {
                let drop = {
                    let (input, memory) = ui.input_and_memory_mut();
                    drop_target(target, DROP_RECT, input, memory, false)
                };
                let drag = ui.captured_domain_drag_gesture(source, source_rect, false);
                (drag, drop)
            } else {
                let drag = ui.captured_domain_drag_gesture(source, source_rect, false);
                let drop = {
                    let (input, memory) = ui.input_and_memory_mut();
                    drop_target(target, DROP_RECT, input, memory, false)
                };
                (drag, drop)
            };
            (source, drag, drop)
        });

        assert_eq!(
            drag.actions
                .iter()
                .filter(|action| action.phase == DomainDragGesturePhase::Release)
                .map(|action| action.ordinal)
                .collect::<Vec<_>>(),
            vec![Some(2), Some(4)]
        );
        assert!(drop.dropped);
        assert_eq!(drop.source, Some(source));
        assert_eq!(harness.memory().released_drag_source(), Some(source));
    }
}
