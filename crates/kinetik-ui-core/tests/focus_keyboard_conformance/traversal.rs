use kinetik_ui_core::{
    FocusTraversal, Key, Modifiers, MouseButton, Point, Rect, ScriptedInput, TextInputEvent,
    TextRange, UiTestHarness, Vec2, WidgetId,
};

use crate::support::{
    click_focusable, ctrl, emit_tree, focus_tree, focus_tree_with_disabled_parent_subtree,
    focus_tree_with_non_focusable, ids,
};

#[test]
fn focus_keyboard_focus_traversal_wraps_forward_and_backward() {
    let (_, first, second, third) = ids();
    let order = vec![second, first, third];

    let from_first = FocusTraversal {
        order: order.clone(),
        focused: Some(first),
    };
    assert_eq!(from_first.next(), Some(third));
    assert_eq!(from_first.previous(), Some(second));

    let from_middle = FocusTraversal {
        order: order.clone(),
        focused: Some(second),
    };
    assert_eq!(from_middle.next(), Some(first));
    assert_eq!(from_middle.previous(), Some(third));

    let from_last = FocusTraversal {
        order: order.clone(),
        focused: Some(third),
    };
    assert_eq!(from_last.next(), Some(second));
    assert_eq!(from_last.previous(), Some(first));

    let no_current = FocusTraversal {
        order,
        focused: None,
    };
    assert_eq!(no_current.next(), Some(second));
    assert_eq!(no_current.previous(), Some(third));
}

#[test]
fn focus_keyboard_focus_traversal_uses_semantic_child_order_and_skips_disabled_focusables() {
    let (_, first, second, third) = ids();
    let tree = focus_tree();
    let traversal = FocusTraversal::from_tree(&tree, Some(second));

    assert_eq!(
        tree.traversal_order()[1..],
        [second, WidgetId::from_key("disabled"), first, third]
    );
    assert_eq!(traversal.order, vec![second, first, third]);
    assert_eq!(traversal.focused, Some(second));
    assert_eq!(traversal.next(), Some(first));
}

#[test]
fn focus_keyboard_focus_traversal_skips_disabled_parent_subtrees() {
    let (_, first, second, third) = ids();
    let disabled_parent = WidgetId::from_key("disabled-parent");
    let disabled_child = WidgetId::from_key("disabled-child");
    let tree = focus_tree_with_disabled_parent_subtree();
    let traversal = FocusTraversal::from_tree(&tree, Some(first));

    assert_eq!(
        tree.traversal_order()[1..],
        [second, disabled_parent, disabled_child, first, third]
    );
    assert_eq!(traversal.order, vec![second, third]);
    assert_eq!(traversal.focused, None);
    assert_eq!(traversal.next(), Some(second));
}

#[test]
fn focus_keyboard_runtime_tab_traverses_forward_from_none_through_middle_and_wraps() {
    let (_, first, second, third) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();

    harness.key_press(Key::Tab);
    let ((), first_output) = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(second));
    assert!(first_output.warnings.is_empty());

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(first));

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(third));

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(second));
}

#[test]
fn focus_keyboard_scripted_input_is_typed_and_frame_based() {
    let mut harness = UiTestHarness::new();
    let delta = std::time::Duration::from_millis(16);

    harness.apply_script([
        ScriptedInput::PointerMove(Point::new(12.0, 24.0)),
        ScriptedInput::PointerDown(MouseButton::Primary),
        ScriptedInput::PointerUp(MouseButton::Primary),
        ScriptedInput::Wheel(Vec2::new(0.0, -3.0)),
        ScriptedInput::key_press(Key::Character("s".to_owned()), ctrl()),
        ScriptedInput::key_release(Key::Character("s".to_owned()), ctrl()),
        ScriptedInput::TextCompositionStart,
        ScriptedInput::TextComposition {
            text: "ka".to_owned(),
            selection: Some(TextRange::new(1, 2)),
        },
        ScriptedInput::TextCommit("か".to_owned()),
        ScriptedInput::TextCompositionEnd,
        ScriptedInput::AdvanceFrame(delta),
    ]);

    assert_eq!(harness.time().delta, delta);
    assert_eq!(harness.time().frame_index, 1);
    assert_eq!(
        harness.input().pointer.position,
        Some(Point::new(12.0, 24.0))
    );
    assert_eq!(harness.input().pointer.wheel_delta, Vec2::new(0.0, -3.0));
    assert!(harness.input().pointer.primary.pressed);
    assert!(harness.input().pointer.primary.released);
    assert_eq!(harness.input().keyboard.modifiers, ctrl());
    assert_eq!(harness.input().keyboard.events.len(), 2);
    assert_eq!(
        harness.input().text_events,
        vec![
            TextInputEvent::CompositionStart,
            TextInputEvent::Composition {
                text: "ka".to_owned(),
                selection: Some(TextRange::new(1, 2)),
            },
            TextInputEvent::Commit("か".to_owned()),
            TextInputEvent::CompositionEnd,
        ]
    );

    let ((), output) = harness.run_frame(|_| {});

    assert!(output.actions.is_empty());
    assert_eq!(
        harness.input().pointer.position,
        Some(Point::new(12.0, 24.0))
    );
    assert_eq!(harness.input().pointer.wheel_delta, Vec2::ZERO);
    assert!(!harness.input().pointer.primary.pressed);
    assert!(!harness.input().pointer.primary.released);
    assert!(harness.input().keyboard.events.is_empty());
    assert!(harness.input().text_events.is_empty());
}

#[test]
fn focus_keyboard_runtime_shift_tab_traverses_backward_from_none_through_middle_and_wraps() {
    let (_, first, second, third) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.set_modifiers(Modifiers::new(true, false, false, false));

    harness.key_press(Key::Tab);
    let ((), first_output) = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(third));
    assert!(first_output.warnings.is_empty());

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(first));

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(second));

    harness.key_press(Key::Tab);
    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(third));
}

#[test]
fn focus_keyboard_runtime_tab_skips_disabled_and_non_focusable_nodes() {
    let (_, first, second, _) = ids();
    let tree = focus_tree_with_non_focusable();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(second);
    harness.key_press(Key::Tab);

    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(first));
}

#[test]
fn focus_keyboard_runtime_tab_skips_disabled_parent_subtree_descendants() {
    let (_, _, second, third) = ids();
    let tree = focus_tree_with_disabled_parent_subtree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(second);
    harness.key_press(Key::Tab);

    let _ = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(third));
}

#[test]
fn focus_keyboard_enabled_focusable_click_acquires_focus_ownership() {
    let id = WidgetId::from_key("button");
    let rect = Rect::new(10.0, 10.0, 40.0, 20.0);
    let mut harness = UiTestHarness::new();

    let response = click_focusable(&mut harness, id, rect, false);

    assert!(response.clicked);
    assert!(response.state.focused);
    assert_eq!(harness.memory().focused(), Some(id));
}

#[test]
fn focus_keyboard_disabled_focus_target_cannot_steal_existing_focus() {
    let focused = WidgetId::from_key("focused");
    let disabled = WidgetId::from_key("disabled");
    let rect = Rect::new(10.0, 10.0, 40.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(focused);

    let response = click_focusable(&mut harness, disabled, rect, true);

    assert!(!response.clicked);
    assert!(response.state.disabled);
    assert!(!response.state.focused);
    assert_eq!(harness.memory().focused(), Some(focused));
}

#[test]
fn focus_keyboard_explicit_focus_clear_leaves_pointer_capture_and_drag_state_untouched() {
    let focused = WidgetId::from_key("focused");
    let pointer_owner = WidgetId::from_key("pointer-owner");
    let drag_owner = WidgetId::from_key("drag-owner");
    let mut harness = UiTestHarness::new();
    let memory = harness.memory_mut();
    memory.focus(focused);
    memory.capture_pointer(pointer_owner);
    memory.start_drag(drag_owner);

    memory.clear_focus();

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().pointer_capture(), Some(pointer_owner));
    assert_eq!(harness.memory().drag_source(), Some(drag_owner));
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn focus_keyboard_runtime_ignores_non_tab_and_released_tab_events() {
    let (_, first, _, _) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(first);
    harness.key_release(Key::Tab);

    let ((), released_output) = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(first));
    assert_eq!(
        released_output.repaint,
        kinetik_ui_core::RepaintRequest::None
    );

    harness.key_press(Key::Enter);
    let ((), non_tab_output) = harness.run_frame(|ui| emit_tree(ui, &tree));
    assert_eq!(harness.memory().focused(), Some(first));
    assert_eq!(
        non_tab_output.repaint,
        kinetik_ui_core::RepaintRequest::None
    );
}
