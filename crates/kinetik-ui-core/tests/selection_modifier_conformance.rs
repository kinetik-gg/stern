//! Event-time modifier conformance for captured selection gestures.

use kinetik_ui_core::{
    CapturedSelectionGesture, ClipId, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point,
    PointerButtonState, Primitive, Rect, SelectionGesturePhase, TextInputEvent, UiInputEvent,
    UiTestHarness, Vec2,
};

const FULL: Rect = Rect::new(0.0, 0.0, 160.0, 80.0);
const CTRL: Modifiers = Modifiers::new(false, true, false, false);
const SHIFT: Modifiers = Modifiers::new(true, false, false, false);
const ALT: Modifiers = Modifiers::new(false, false, true, false);

fn run_selection(harness: &mut UiTestHarness) -> CapturedSelectionGesture {
    harness
        .run_frame(|ui| {
            let id = ui.id("selection");
            ui.captured_selection_gesture(id, FULL, false)
        })
        .0
}

fn retain_modifiers(harness: &mut UiTestHarness, modifiers: Modifiers) {
    harness.set_modifiers(modifiers);
    let _ = harness.run_frame(|_| ());
}

#[test]
fn canonical_actions_use_modifiers_from_their_original_ordinals() {
    let mut harness = UiTestHarness::new();
    retain_modifiers(&mut harness, CTRL);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_modifiers(SHIFT);
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let gesture = run_selection(&mut harness);
    assert_eq!(
        gesture
            .actions
            .iter()
            .map(|action| (action.phase, action.modifiers))
            .collect::<Vec<_>>(),
        vec![
            (SelectionGesturePhase::Press, CTRL),
            (SelectionGesturePhase::Move, SHIFT),
            (SelectionGesturePhase::Release, SHIFT),
        ]
    );
}

#[test]
fn key_events_replace_the_running_modifier_state() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            SHIFT,
            false,
        )));
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let gesture = run_selection(&mut harness);
    assert_eq!(gesture.actions[0].modifiers, Modifiers::default());
    assert_eq!(gesture.actions[1].modifiers, SHIFT);
    assert_eq!(gesture.actions[2].modifiers, SHIFT);
}

#[test]
fn pointer_only_canonical_input_does_not_guess_from_the_final_snapshot() {
    let mut harness = UiTestHarness::new();
    harness.input_mut().keyboard.modifiers = SHIFT;
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);

    let gesture = run_selection(&mut harness);
    assert_eq!(gesture.actions.len(), 1);
    assert_eq!(gesture.actions[0].ordinal, Some(1));
    assert_eq!(gesture.actions[0].modifiers, Modifiers::default());
}

#[test]
fn spatial_filtering_keeps_root_modifier_association_across_gaps() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(200.0, 10.0));
    harness.set_modifiers(CTRL);
    harness
        .input_mut()
        .push_event(UiInputEvent::Text(TextInputEvent::Commit("gap".to_owned())));
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_modifiers(SHIFT);
    harness.set_pointer_position(Point::new(12.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let clip = ClipId::from_raw(101);
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
            .map(|action| (action.ordinal, action.modifiers))
            .collect::<Vec<_>>(),
        vec![(Some(4), CTRL), (Some(6), SHIFT), (Some(7), SHIFT)]
    );
}

#[test]
fn legacy_snapshot_modifiers_apply_and_seed_the_next_canonical_frame() {
    let mut harness = UiTestHarness::new();
    let input = harness.input_mut();
    input.keyboard.modifiers = CTRL;
    input.pointer.position = Some(Point::new(10.0, 10.0));
    input.pointer.primary = PointerButtonState::new(true, true, false);
    input.pointer.click_count = 1;
    assert!(input.events.is_empty());

    let pressed = run_selection(&mut harness);
    assert_eq!(pressed.actions.len(), 1);
    assert_eq!(pressed.actions[0].ordinal, None);
    assert_eq!(pressed.actions[0].modifiers, CTRL);

    harness.pointer_release(MouseButton::Primary);
    let released = run_selection(&mut harness);
    assert_eq!(released.actions.len(), 1);
    assert_eq!(released.actions[0].phase, SelectionGesturePhase::Release);
    assert_eq!(released.actions[0].modifiers, CTRL);
}

#[test]
fn official_focus_loss_cancels_once_and_suspends_until_focus_gain() {
    let mut harness = UiTestHarness::new();
    retain_modifiers(&mut harness, CTRL);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_selection(&mut harness);

    harness.set_window_focused(false);
    harness.set_modifiers(SHIFT);
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            SHIFT,
            false,
        )));
    let cancelled = run_selection(&mut harness);
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(0));
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(cancelled.actions[0].modifiers, CTRL);

    harness.set_window_focused(true);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    harness.pointer_release(MouseButton::Primary);
    let regained = run_selection(&mut harness);
    assert!(
        regained
            .actions
            .iter()
            .all(|action| action.modifiers.is_empty())
    );

    harness.set_modifiers(ALT);
    harness.set_pointer_position(Point::new(11.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let resumed = run_selection(&mut harness);
    assert_eq!(resumed.actions[0].modifiers, ALT);
}

#[test]
fn legacy_focus_loss_reports_snapshot_then_resets_the_canonical_baseline() {
    let mut harness = UiTestHarness::new();
    retain_modifiers(&mut harness, CTRL);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_selection(&mut harness);

    harness.input_mut().window_focused = false;
    harness.input_mut().keyboard.modifiers = SHIFT;
    assert!(harness.input().events.is_empty());
    let cancelled = run_selection(&mut harness);
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, None);
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(cancelled.actions[0].modifiers, SHIFT);

    harness.set_window_focused(true);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let regained = run_selection(&mut harness);
    assert_eq!(regained.actions[0].modifiers, Modifiers::default());
}

#[test]
fn conflicted_cleanup_uses_entry_baseline_without_applying_modifier_changes() {
    let mut harness = UiTestHarness::new();
    retain_modifiers(&mut harness, CTRL);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_selection(&mut harness);

    harness.pointer_release(MouseButton::Primary);
    harness.set_modifiers(SHIFT);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let cancelled = run_selection(&mut harness);
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].phase, SelectionGesturePhase::Cancel);
    assert_eq!(cancelled.actions[0].modifiers, CTRL);

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let next = run_selection(&mut harness);
    assert_eq!(next.actions[0].modifiers, CTRL);
}

#[test]
fn conflicted_focus_loss_still_performs_the_safety_reset() {
    let mut harness = UiTestHarness::new();
    retain_modifiers(&mut harness, CTRL);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = run_selection(&mut harness);

    harness.set_window_focused(false);
    harness.set_modifiers(SHIFT);
    harness.input_mut().pointer.delta = Vec2::new(99.0, 0.0);
    let cancelled = run_selection(&mut harness);
    assert_eq!(cancelled.actions.len(), 1);
    assert_eq!(cancelled.actions[0].ordinal, Some(0));
    assert_eq!(cancelled.actions[0].modifiers, CTRL);

    harness.set_window_focused(true);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let regained = run_selection(&mut harness);
    assert_eq!(regained.actions[0].modifiers, Modifiers::default());
}

#[test]
fn second_same_owner_claim_does_not_replay_modifier_actions() {
    let mut harness = UiTestHarness::new();
    harness.set_modifiers(SHIFT);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);

    let ((first, second), _) = harness.run_frame(|ui| {
        let id = ui.id("selection");
        let first = ui.captured_selection_gesture(id, FULL, false);
        let second = ui.captured_selection_gesture(id, FULL, false);
        (first, second)
    });
    assert_eq!(first.actions[0].modifiers, SHIFT);
    assert!(second.actions.is_empty());
}
