//! Causal `DomainDrag` action and first-claim conformance.

use kinetik_ui_core::{
    ClipId, DomainDragGesturePhase, Modifiers, MouseButton, Point, PointerButtonState,
    PointerOrder, PointerTarget, Primitive, Rect, TextInputEvent, Transform, UiInput, UiInputEvent,
    UiMemory, UiTestHarness, Vec2, WidgetId, draggable, draggable_transformed, drop_target,
};

const FULL: Rect = Rect::new(0.0, 0.0, 160.0, 80.0);
const MISS: Rect = Rect::new(300.0, 0.0, 40.0, 40.0);
const CTRL: Modifiers = Modifiers {
    ctrl: true,
    alt: false,
    shift: false,
    super_key: false,
};
const SHIFT: Modifiers = Modifiers {
    ctrl: false,
    alt: false,
    shift: true,
    super_key: false,
};

fn release_outcome_at(end: Point) -> kinetik_ui_core::CapturedDomainDragGesture {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_pointer_position(end);
    harness.pointer_release(MouseButton::Primary);
    harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0
}

fn release_actions(
    gesture: &kinetik_ui_core::CapturedDomainDragGesture,
) -> Vec<(Option<usize>, bool)> {
    gesture
        .actions
        .iter()
        .filter(|action| action.phase == DomainDragGesturePhase::Release)
        .map(|action| (action.ordinal, action.release_clicked))
        .collect()
}

fn captured_in_source_scope(
    ui: &mut kinetik_ui_core::Ui<'_>,
    id: WidgetId,
) -> kinetik_ui_core::CapturedDomainDragGesture {
    let transform = Transform::scale(Vec2::new(2.0, 2.0));
    let clip = ClipId::from_raw(402);
    ui.push_primitive(Primitive::TransformBegin(transform));
    ui.push_primitive(Primitive::ClipBegin {
        id: clip,
        rect: FULL,
    });
    let gesture = ui.captured_domain_drag_gesture(id, FULL, false);
    ui.push_primitive(Primitive::ClipEnd { id: clip });
    ui.push_primitive(Primitive::TransformEnd);
    gesture
}

fn start_crossed_scoped_drag(harness: &mut UiTestHarness) {
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    let pressed = harness
        .run_frame(|ui| {
            let id = ui.id("source");
            captured_in_source_scope(ui, id)
        })
        .0;
    assert_eq!(pressed.actions[0].phase, DomainDragGesturePhase::Press);

    harness.set_pointer_position(Point::new(28.0, 20.0));
    let crossed = harness
        .run_frame(|ui| {
            let id = ui.id("source");
            captured_in_source_scope(ui, id)
        })
        .0;
    assert!(crossed.response.dragged);
    assert_eq!(crossed.response.drag_delta, Vec2::new(4.0, 0.0));
}

#[test]
fn release_actions_pin_below_exact_and_above_threshold_outcomes() {
    let below = release_outcome_at(Point::new(13.0, 10.0));
    assert!(below.response.clicked);
    assert_eq!(release_actions(&below), vec![(Some(3), true)]);

    let exact = release_outcome_at(Point::new(14.0, 10.0));
    assert!(!exact.response.clicked);
    assert!(exact.response.dragged);
    assert_eq!(release_actions(&exact), vec![(Some(3), false)]);

    let above = release_outcome_at(Point::new(18.0, 10.0));
    assert!(!above.response.clicked);
    assert!(above.response.dragged);
    assert_eq!(release_actions(&above), vec![(Some(3), false)]);
}

#[test]
fn multiframe_below_threshold_release_retains_causal_metadata() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    let press = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(press.actions[0].click_count, 2);

    harness.set_pointer_position(Point::new(13.0, 10.0));
    let movement = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(movement.actions[0].delta, Vec2::new(3.0, 0.0));
    assert_eq!(movement.actions[0].click_count, 2);

    harness.pointer_release(MouseButton::Primary);
    harness.set_click_count(2);
    let release = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert!(release.response.clicked);
    assert_eq!(release.actions[0].phase, DomainDragGesturePhase::Release);
    assert_eq!(release.actions[0].delta, Vec2::ZERO);
    assert_eq!(release.actions[0].click_count, 2);
    assert!(release.actions[0].release_clicked);
}

#[test]
fn each_same_frame_release_carries_its_own_click_result() {
    for crossed_first in [false, true] {
        let mut harness = UiTestHarness::new();
        let transactions = if crossed_first {
            [
                (Point::new(10.0, 10.0), Point::new(15.0, 10.0)),
                (Point::new(30.0, 10.0), Point::new(32.0, 10.0)),
            ]
        } else {
            [
                (Point::new(10.0, 10.0), Point::new(12.0, 10.0)),
                (Point::new(30.0, 10.0), Point::new(35.0, 10.0)),
            ]
        };
        for (start, end) in transactions {
            harness.set_pointer_position(start);
            harness.pointer_press(MouseButton::Primary);
            harness.set_pointer_position(end);
            harness.pointer_release(MouseButton::Primary);
        }

        let gesture = harness
            .run_frame(|ui| {
                let id = ui.id("drag");
                ui.captured_domain_drag_gesture(id, FULL, false)
            })
            .0;
        let outcomes = release_actions(&gesture)
            .into_iter()
            .map(|(_, clicked)| clicked)
            .collect::<Vec<_>>();
        assert_eq!(
            outcomes,
            if crossed_first {
                vec![false, true]
            } else {
                vec![true, false]
            }
        );
        assert!(gesture.response.clicked);
        assert!(gesture.response.dragged);
    }
}

#[test]
fn outside_and_missing_position_releases_never_claim_a_click() {
    let outside = release_outcome_at(Point::new(200.0, 10.0));
    assert!(!outside.response.clicked);
    assert_eq!(release_actions(&outside), vec![(Some(3), false)]);

    let mut missing = UiTestHarness::new();
    missing.set_pointer_position(Point::new(10.0, 10.0));
    missing.pointer_press(MouseButton::Primary);
    missing.input_mut().push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: None,
    });
    let gesture = missing
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert!(!gesture.response.clicked);
    assert_eq!(release_actions(&gesture), vec![(Some(2), false)]);
}

#[test]
fn spatial_gaps_preserve_root_ordinals_local_positions_and_modifiers() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(400.0, 10.0));
    harness.set_modifiers(CTRL);
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit("gap".to_owned())));
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    harness.set_modifiers(SHIFT);
    harness.set_pointer_position(Point::new(24.0, 20.0));
    harness.pointer_release(MouseButton::Primary);

    let transform = Transform::scale(Vec2::new(2.0, 2.0));
    let clip = ClipId::from_raw(401);
    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.push_primitive(Primitive::TransformBegin(transform));
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: FULL,
            });
            let gesture = ui.captured_domain_drag_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            ui.push_primitive(Primitive::TransformEnd);
            gesture
        })
        .0;

    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| (action.ordinal, action.modifiers))
            .collect::<Vec<_>>(),
        vec![(Some(4), CTRL), (Some(6), SHIFT), (Some(7), SHIFT)]
    );
    assert_eq!(gesture.actions[0].position, Some(Point::new(10.0, 10.0)));
    assert_eq!(gesture.actions[1].position, Some(Point::new(12.0, 10.0)));
    assert_eq!(gesture.actions[0].delta, Vec2::ZERO);
    assert_eq!(gesture.actions[1].delta, Vec2::new(2.0, 0.0));
    assert_eq!(gesture.actions[2].delta, Vec2::ZERO);
    assert!(gesture.actions.iter().all(|action| action.click_count == 2));
    assert!(gesture.actions[2].release_clicked);
}

#[test]
fn legacy_actions_use_no_ordinal_and_snapshot_modifiers() {
    let mut harness = UiTestHarness::new();
    let input = harness.input_mut();
    input.keyboard.modifiers = CTRL;
    input.pointer.position = Some(Point::new(10.0, 10.0));
    input.pointer.primary = PointerButtonState::new(true, true, false);
    input.pointer.click_count = 3;
    assert!(input.events.is_empty());

    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(gesture.actions.len(), 1);
    assert_eq!(gesture.actions[0].phase, DomainDragGesturePhase::Press);
    assert_eq!(gesture.actions[0].ordinal, None);
    assert_eq!(gesture.actions[0].modifiers, CTRL);
    assert_eq!(gesture.actions[0].delta, Vec2::ZERO);
    assert_eq!(gesture.actions[0].click_count, 3);
    assert!(!gesture.actions[0].release_clicked);
}

#[test]
fn focus_loss_emits_one_non_clicking_cancel_with_event_time_modifiers() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(3);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("drag");
        ui.captured_domain_drag_gesture(id, FULL, false)
    });

    harness.set_modifiers(CTRL);
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.set_window_focused(false);
    let gesture = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    let cancels = gesture
        .actions
        .iter()
        .filter(|action| action.phase == DomainDragGesturePhase::Cancel)
        .collect::<Vec<_>>();
    assert_eq!(cancels.len(), 1);
    assert_eq!(cancels[0].ordinal, Some(2));
    assert_eq!(cancels[0].position, Some(Point::new(12.0, 10.0)));
    assert_eq!(cancels[0].delta, Vec2::ZERO);
    assert_eq!(cancels[0].click_count, 3);
    assert_eq!(cancels[0].modifiers, CTRL);
    assert!(!cancels[0].release_clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().drag_source(), None);
}

#[test]
fn explicit_release_all_cancels_once_without_selection_replay() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    let _ = harness.run_frame(|ui| {
        let id = ui.id("drag");
        ui.captured_domain_drag_gesture(id, FULL, false)
    });

    harness.input_mut().release_pointer_buttons();
    let ((domain, selection), _) = harness.run_frame(|ui| {
        let id = ui.id("drag");
        let domain = ui.captured_domain_drag_gesture(id, FULL, false);
        let selection = ui.captured_selection_gesture(id, FULL, false);
        (domain, selection)
    });
    assert_eq!(domain.actions.len(), 1);
    assert_eq!(domain.actions[0].ordinal, Some(0));
    assert_eq!(domain.actions[0].phase, DomainDragGesturePhase::Cancel);
    assert_eq!(domain.actions[0].position, Some(Point::new(10.0, 10.0)));
    assert_eq!(domain.actions[0].delta, Vec2::ZERO);
    assert_eq!(domain.actions[0].click_count, 2);
    assert!(!domain.actions[0].release_clicked);
    assert!(selection.actions.is_empty());
}

#[test]
fn legacy_and_disabled_pre_resolver_cancellation_reaches_domain_only() {
    let mut legacy = UiTestHarness::new();
    legacy.set_pointer_position(Point::new(10.0, 10.0));
    legacy.pointer_press(MouseButton::Primary);
    legacy.set_click_count(2);
    let _ = legacy.run_frame(|ui| {
        let id = ui.id("drag");
        ui.captured_domain_drag_gesture(id, FULL, false)
    });
    legacy.input_mut().pointer.click_count = 2;
    legacy.input_mut().window_focused = false;
    assert!(legacy.input().events.is_empty());
    let ((selection, domain), _) = legacy.run_frame(|ui| {
        let id = ui.id("drag");
        let selection = ui.captured_selection_gesture(id, FULL, false);
        let domain = ui.captured_domain_drag_gesture(id, FULL, false);
        (selection, domain)
    });
    assert!(selection.actions.is_empty());
    assert_eq!(domain.actions.len(), 1);
    assert_eq!(domain.actions[0].ordinal, None);
    assert_eq!(domain.actions[0].phase, DomainDragGesturePhase::Cancel);
    assert_eq!(domain.actions[0].position, Some(Point::new(10.0, 10.0)));
    assert_eq!(domain.actions[0].click_count, 2);
    assert!(!domain.actions[0].release_clicked);

    let mut disabled = UiTestHarness::new();
    disabled.set_pointer_position(Point::new(10.0, 10.0));
    disabled.pointer_press(MouseButton::Primary);
    let _ = disabled.run_frame(|ui| {
        let id = ui.id("drag");
        ui.captured_domain_drag_gesture(id, FULL, false)
    });
    let cancelled = disabled
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, true)
        })
        .0;
    assert!(cancelled.response.state.disabled);
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, None);
    assert_eq!(cancelled.actions[0].phase, DomainDragGesturePhase::Cancel);
    assert!(!cancelled.actions[0].release_clicked);

    let manual_selection = WidgetId::from_key("manual-selection");
    let mut selection_compat = UiTestHarness::new();
    selection_compat.memory_mut().activate(manual_selection);
    selection_compat.memory_mut().press(manual_selection);
    selection_compat
        .memory_mut()
        .capture_pointer(manual_selection);
    selection_compat.input_mut().pointer.position = Some(Point::new(10.0, 10.0));
    selection_compat.input_mut().window_focused = false;
    let selection_cancel = selection_compat
        .run_frame(|ui| {
            ui.register_id(manual_selection);
            ui.captured_selection_gesture(manual_selection, FULL, false)
        })
        .0;
    assert_eq!(selection_cancel.actions.len(), 1);
    assert_eq!(
        selection_cancel.actions[0].phase,
        kinetik_ui_core::SelectionGesturePhase::Cancel
    );

    let manual_domain = WidgetId::from_key("manual-domain");
    let mut domain_compat = UiTestHarness::new();
    domain_compat.memory_mut().activate(manual_domain);
    domain_compat.memory_mut().press(manual_domain);
    domain_compat.memory_mut().capture_pointer(manual_domain);
    domain_compat.memory_mut().start_drag(manual_domain);
    domain_compat.input_mut().pointer.position = Some(Point::new(10.0, 10.0));
    domain_compat.input_mut().window_focused = false;
    let domain_cancel = domain_compat
        .run_frame(|ui| {
            ui.register_id(manual_domain);
            ui.captured_domain_drag_gesture(manual_domain, FULL, false)
        })
        .0;
    assert_eq!(domain_cancel.actions.len(), 1);
    assert_eq!(
        domain_cancel.actions[0].phase,
        DomainDragGesturePhase::Cancel
    );
}

#[test]
fn conflict_and_clipped_cleanup_are_non_clicking_cancels() {
    let mut conflicted = UiTestHarness::new();
    conflicted.set_pointer_position(Point::new(10.0, 10.0));
    conflicted.pointer_press(MouseButton::Primary);
    let _ = conflicted.run_frame(|ui| {
        let id = ui.id("drag");
        ui.captured_domain_drag_gesture(id, FULL, false)
    });
    conflicted.pointer_release(MouseButton::Primary);
    conflicted.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let cancel = conflicted
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert_eq!(cancel.actions.len(), 1);
    assert_eq!(cancel.actions[0].ordinal, Some(0));
    assert_eq!(cancel.actions[0].phase, DomainDragGesturePhase::Cancel);
    assert!(!cancel.actions[0].release_clicked);

    let clip = ClipId::from_raw(403);
    let clip_rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut clipped = UiTestHarness::new();
    clipped.set_pointer_position(Point::new(10.0, 10.0));
    clipped.pointer_press(MouseButton::Primary);
    let _ = clipped.run_frame(|ui| {
        let id = ui.id("drag");
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: clip_rect,
        });
        let gesture = ui.captured_domain_drag_gesture(id, FULL, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        gesture
    });
    clipped.set_pointer_position(Point::new(50.0, 10.0));
    clipped.pointer_release(MouseButton::Primary);
    let cleanup = clipped
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: clip_rect,
            });
            let gesture = ui.captured_domain_drag_gesture(id, FULL, false);
            ui.push_primitive(Primitive::ClipEnd { id: clip });
            gesture
        })
        .0;
    assert_eq!(cleanup.actions.len(), 1);
    assert_eq!(cleanup.actions[0].ordinal, Some(1));
    assert_eq!(cleanup.actions[0].phase, DomainDragGesturePhase::Cancel);
    assert!(!cleanup.actions[0].release_clicked);
}

#[test]
fn captured_duplicates_return_the_exact_response_without_memory_mutation() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_pointer_position(Point::new(14.0, 10.0));

    let ((first, second, unchanged), _) = harness.run_frame(|ui| {
        let id = ui.id("drag");
        let first = ui.captured_domain_drag_gesture(id, FULL, false);
        let after_first = format!("{:#?}", ui.memory());
        let second = ui.captured_domain_drag_gesture(id, MISS, true);
        let unchanged = after_first == format!("{:#?}", ui.memory());
        (first, second, unchanged)
    });
    assert!(!first.actions.is_empty());
    assert_eq!(second.response, first.response);
    assert!(second.actions.is_empty());
    assert!(unchanged);
}

#[test]
fn ordinary_captured_and_transformed_calls_share_one_exact_response() {
    let mut ordinary_first = UiTestHarness::new();
    ordinary_first.set_pointer_position(Point::new(10.0, 10.0));
    ordinary_first.pointer_press(MouseButton::Primary);
    ordinary_first.set_pointer_position(Point::new(14.0, 10.0));
    let ((ordinary, captured, unchanged), _) = ordinary_first.run_frame(|ui| {
        let id = ui.id("drag");
        let ordinary = {
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(id, FULL, Transform::IDENTITY, input, memory, false)
        };
        let after_first = format!("{:#?}", ui.memory());
        let captured = ui.captured_domain_drag_gesture(id, MISS, true);
        (
            ordinary,
            captured,
            after_first == format!("{:#?}", ui.memory()),
        )
    });
    assert_eq!(captured.response, ordinary);
    assert!(captured.actions.is_empty());
    assert!(unchanged);

    let mut captured_first = UiTestHarness::new();
    captured_first.set_pointer_position(Point::new(10.0, 10.0));
    captured_first.pointer_press(MouseButton::Primary);
    captured_first.set_pointer_position(Point::new(14.0, 10.0));
    let ((captured, ordinary, unchanged), _) = captured_first.run_frame(|ui| {
        let id = ui.id("drag");
        let captured = ui.captured_domain_drag_gesture(id, FULL, false);
        let after_first = format!("{:#?}", ui.memory());
        let ordinary = {
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(id, MISS, Transform::IDENTITY, input, memory, true)
        };
        (
            captured,
            ordinary,
            after_first == format!("{:#?}", ui.memory()),
        )
    });
    assert_eq!(ordinary, captured.response);
    assert!(!captured.actions.is_empty());
    assert!(unchanged);
}

#[test]
fn disabled_first_and_next_frame_reset_are_deterministic() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let (first, _) = harness.run_frame(|ui| {
        let id = ui.id("drag");
        let disabled = {
            let (input, memory) = ui.input_and_memory_mut();
            draggable(id, FULL, input, memory, true)
        };
        let captured = ui.captured_domain_drag_gesture(id, FULL, false);
        (disabled, captured)
    });
    assert!(first.0.state.disabled);
    assert_eq!(first.1.response, first.0);
    assert!(first.1.actions.is_empty());

    harness.pointer_press(MouseButton::Primary);
    let next = harness
        .run_frame(|ui| {
            let id = ui.id("drag");
            ui.captured_domain_drag_gesture(id, FULL, false)
        })
        .0;
    assert!(!next.response.state.disabled);
    assert_eq!(next.actions[0].phase, DomainDragGesturePhase::Press);
}

#[test]
fn unframed_calls_stay_uncached_and_runtime_end_closes_the_cache() {
    let id = WidgetId::from_key("drag");
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let first = draggable(id, FULL, &input, &mut memory, true);
    let second = draggable(id, MISS, &input, &mut memory, false);
    assert!(first.state.disabled);
    assert!(!second.state.disabled);
    assert_eq!(second.rect, MISS);

    let mut harness = UiTestHarness::new();
    let runtime_id = harness.run_frame(|ui| {
        let id = ui.id("drag");
        let gesture = ui.captured_domain_drag_gesture(id, FULL, true);
        assert!(gesture.response.state.disabled);
        id
    });
    let pending = harness.input().clone();
    let after_end = draggable(runtime_id.0, MISS, &pending, harness.memory_mut(), false);
    assert!(!after_end.state.disabled);
    assert_eq!(after_end.rect, MISS);
}

#[test]
fn claims_are_per_owner_and_independent_from_selection() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let ((missed, hit), _) = harness.run_frame(|ui| {
        let missed_id = ui.id("missed");
        let hit_id = ui.id("hit");
        let missed = ui.captured_domain_drag_gesture(missed_id, MISS, false);
        let hit = ui.captured_domain_drag_gesture(hit_id, FULL, false);
        (missed, hit)
    });
    assert!(missed.actions.is_empty());
    assert_eq!(hit.actions[0].phase, DomainDragGesturePhase::Press);

    let mut independent = UiTestHarness::new();
    independent.set_pointer_position(Point::new(10.0, 10.0));
    independent.pointer_press(MouseButton::Primary);
    let (domain, _) = independent.run_frame(|ui| {
        let id = ui.id("shared");
        let selection = ui.captured_selection_gesture(id, MISS, false);
        assert!(selection.actions.is_empty());
        ui.captured_domain_drag_gesture(id, FULL, false)
    });
    assert_eq!(domain.actions[0].phase, DomainDragGesturePhase::Press);

    let mut selection_owner = UiTestHarness::new();
    selection_owner.set_pointer_position(Point::new(10.0, 10.0));
    selection_owner.pointer_press(MouseButton::Primary);
    let _ = selection_owner.run_frame(|ui| {
        let id = ui.id("shared");
        ui.captured_selection_gesture(id, FULL, false)
    });
    selection_owner.pointer_release(MouseButton::Primary);
    let ((domain, selection), _) = selection_owner.run_frame(|ui| {
        let id = ui.id("shared");
        let domain = ui.captured_domain_drag_gesture(id, FULL, false);
        let selection = ui.captured_selection_gesture(id, FULL, false);
        (domain, selection)
    });
    assert!(domain.actions.is_empty());
    assert_eq!(selection.actions.len(), 1);
    assert_eq!(
        selection.actions[0].phase,
        kinetik_ui_core::SelectionGesturePhase::Cancel
    );
}

#[test]
fn captured_actions_do_not_change_planned_or_unplanned_drop_authority() {
    let transform = Transform::scale(Vec2::new(2.0, 2.0));
    let target_rect = Rect::new(0.0, 0.0, 80.0, 80.0);
    for target_first in [false, true] {
        let mut planned = UiTestHarness::new();
        start_crossed_scoped_drag(&mut planned);
        planned.set_pointer_position(Point::new(400.0, 20.0));
        planned
            .input_mut()
            .push_event(UiInputEvent::Text(TextInputEvent::Commit("gap".to_owned())));
        planned.set_modifiers(CTRL);
        planned.set_pointer_position(Point::new(28.0, 20.0));
        planned.pointer_release(MouseButton::Primary);
        let ((source, gesture, drop), _) = planned.run_frame(|ui| {
            let source = ui.id("source");
            let target = ui.id("target");
            ui.resolve_pointer_targets(|plan| {
                plan.with_transform(transform, |plan| {
                    plan.with_clip(FULL, |plan| {
                        plan.target(
                            PointerTarget::new(source, FULL, PointerOrder::new(20))
                                .domain_drag_source(),
                        );
                    });
                });
                plan.target(
                    PointerTarget::new(target, target_rect, PointerOrder::new(30))
                        .ordinary_owner(None)
                        .drop_owner(target),
                );
            })
            .expect("valid transformed clipped captured DomainDrag plan");
            let (gesture, drop) = if target_first {
                let drop = {
                    let (input, memory) = ui.input_and_memory_mut();
                    drop_target(target, target_rect, input, memory, false)
                };
                let gesture = captured_in_source_scope(ui, source);
                (gesture, drop)
            } else {
                let gesture = captured_in_source_scope(ui, source);
                let drop = {
                    let (input, memory) = ui.input_and_memory_mut();
                    drop_target(target, target_rect, input, memory, false)
                };
                (gesture, drop)
            };
            (source, gesture, drop)
        });
        assert!(drop.dropped);
        assert_eq!(drop.source, Some(source));
        assert_eq!(gesture.response.id, source);
        assert_eq!(release_actions(&gesture), vec![(Some(4), false)]);
        assert_eq!(gesture.actions.last().unwrap().modifiers, CTRL);
        assert_eq!(planned.memory().released_drag_source(), Some(source));
    }

    let mut unplanned = UiTestHarness::new();
    start_crossed_scoped_drag(&mut unplanned);
    unplanned.set_pointer_position(Point::new(400.0, 20.0));
    unplanned
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit("gap".to_owned())));
    unplanned.set_modifiers(CTRL);
    unplanned.set_pointer_position(Point::new(28.0, 20.0));
    unplanned.pointer_release(MouseButton::Primary);
    let ((gesture, drop), _) = unplanned.run_frame(|ui| {
        let source = ui.id("source");
        let target = ui.id("target");
        let drop = {
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, target_rect, input, memory, false)
        };
        let gesture = captured_in_source_scope(ui, source);
        (gesture, drop)
    });
    assert!(!drop.dropped);
    assert_eq!(drop.source, None);
    assert_eq!(release_actions(&gesture), vec![(Some(4), false)]);
}
