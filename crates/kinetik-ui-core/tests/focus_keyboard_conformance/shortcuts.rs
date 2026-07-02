use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionId, ActionPriority, ActionRouter, ActionRoutingContext,
    Key, KeyEvent, KeyState, Modifiers, PhysicalKey, Rect, Shortcut, UiTestHarness, WidgetId,
    pressable,
};

use crate::support::{
    bind_global, ctrl, ctrl_shortcut, emit_tree, focus_tree, focus_tree_with_non_focusable, ids,
    key_input, physical_input, shortcut_action, super_key,
};

#[test]
fn focus_keyboard_accessibility_snapshot_filters_focused_identity_when_not_focusable() {
    let (_, first, second, third) = ids();
    let disabled = WidgetId::from_key("disabled");
    let label = WidgetId::from_key("label");
    let tree = focus_tree();
    let non_focusable_tree = focus_tree_with_non_focusable();

    let disabled_snapshot = tree
        .accessibility_snapshot(Some(disabled))
        .expect("valid snapshot");
    assert_eq!(disabled_snapshot.focus_order, vec![second, first, third]);
    assert_eq!(disabled_snapshot.focused, None);

    let missing_snapshot = tree
        .accessibility_snapshot(Some(WidgetId::from_key("missing")))
        .expect("valid snapshot");
    assert_eq!(missing_snapshot.focus_order, vec![second, first, third]);
    assert_eq!(missing_snapshot.focused, None);

    let non_focusable_snapshot = non_focusable_tree
        .accessibility_snapshot(Some(label))
        .expect("valid snapshot");
    assert_eq!(
        non_focusable_snapshot.focus_order,
        vec![second, first, third]
    );
    assert_eq!(non_focusable_snapshot.focused, None);
}

#[test]
fn focus_keyboard_frame_output_snapshot_filters_missing_and_non_focusable_focus_ids() {
    let (_, first, second, third) = ids();
    let label = WidgetId::from_key("label");
    let tree = focus_tree_with_non_focusable();
    let mut harness = UiTestHarness::new();
    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    let missing_snapshot = output
        .accessibility_snapshot(Some(WidgetId::from_key("missing")))
        .expect("valid snapshot");
    assert_eq!(missing_snapshot.focus_order, vec![second, first, third]);
    assert_eq!(missing_snapshot.focused, None);

    let non_focusable_snapshot = output
        .accessibility_snapshot(Some(label))
        .expect("valid snapshot");
    assert_eq!(
        non_focusable_snapshot.focus_order,
        vec![second, first, third]
    );
    assert_eq!(non_focusable_snapshot.focused, None);
}

#[test]
fn focus_keyboard_focused_pressable_activates_once_from_enter_and_space() {
    for key in [Key::Enter, Key::Space] {
        let mut harness = UiTestHarness::new();
        let id = WidgetId::from_key("button");
        harness.memory_mut().focus(id);
        harness.key_press(key);

        let first = harness
            .run_frame(|ui| {
                let (input, memory) = ui.input_and_memory_mut();
                pressable(id, Rect::ZERO, input, memory, false)
            })
            .0;
        let idle = harness
            .run_frame(|ui| {
                let (input, memory) = ui.input_and_memory_mut();
                pressable(id, Rect::ZERO, input, memory, false)
            })
            .0;

        assert!(first.clicked);
        assert!(first.keyboard_activated);
        assert!(!idle.clicked);
        assert!(!idle.keyboard_activated);
    }
}

#[test]
fn focus_keyboard_pressable_ignores_unfocused_disabled_release_and_repeat_keyboard_activation() {
    let id = WidgetId::from_key("button");
    let mut harness = UiTestHarness::new();
    harness.key_press(Key::Enter);
    let unfocused = harness
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, Rect::ZERO, input, memory, false)
        })
        .0;
    assert!(!unfocused.clicked);

    let mut disabled = UiTestHarness::new();
    disabled.memory_mut().focus(id);
    disabled.key_press(Key::Enter);
    let disabled_response = disabled
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, Rect::ZERO, input, memory, true)
        })
        .0;
    assert!(!disabled_response.clicked);
    assert!(!disabled_response.keyboard_activated);

    let mut release = UiTestHarness::new();
    release.memory_mut().focus(id);
    release.key_release(Key::Enter);
    let released = release
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, Rect::ZERO, input, memory, false)
        })
        .0;
    assert!(!released.keyboard_activated);

    let mut repeat = UiTestHarness::new();
    repeat.memory_mut().focus(id);
    repeat.input_mut().keyboard.events.push(KeyEvent::new(
        Key::Enter,
        KeyState::Pressed,
        Modifiers::default(),
        true,
    ));
    let repeated = repeat
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            pressable(id, Rect::ZERO, input, memory, false)
        })
        .0;
    assert!(!repeated.keyboard_activated);
}

#[test]
fn focus_keyboard_text_input_owner_blocks_space_from_pressable_keyboard_activation() {
    let id = WidgetId::from_key("field");
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(id);
    harness.memory_mut().set_text_input_owner(id);
    harness.key_press(Key::Space);
    harness.text_commit(" ");

    let response = harness
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            assert_eq!(input.text_events.len(), 1);
            pressable(id, Rect::ZERO, input, memory, false)
        })
        .0;

    assert!(!response.clicked);
    assert!(!response.keyboard_activated);
}

#[test]
fn focus_keyboard_text_input_blocks_reserved_global_shortcuts() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind_global(
        &mut router,
        "global.type",
        Shortcut::new(Modifiers::default(), Key::Character("x".to_owned())),
    );
    bind_global(
        &mut router,
        "global.space",
        Shortcut::new(Modifiers::default(), Key::Space),
    );
    bind_global(
        &mut router,
        "global.tab",
        Shortcut::new(Modifiers::default(), Key::Tab),
    );
    bind_global(
        &mut router,
        "global.shift.tab",
        Shortcut::new(Modifiers::new(true, false, false, false), Key::Tab),
    );

    for character in ["a", "c", "v", "x", "y", "z"] {
        bind_global(
            &mut router,
            &format!("global.ctrl.{character}"),
            Shortcut::new(ctrl(), Key::Character(character.to_owned())),
        );
        bind_global(
            &mut router,
            &format!("global.super.{character}"),
            Shortcut::new(super_key(), Key::Character(character.to_owned())),
        );
    }

    let routing = ActionRoutingContext::new().with_text_input(field);
    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_input(Key::Character("x".to_owned()), Modifiers::default()),
            routing,
        ),
        None
    );
    assert_eq!(
        router.resolve_shortcut_in_context(&key_input(Key::Space, Modifiers::default()), routing),
        None
    );
    assert_eq!(
        router.resolve_shortcut_in_context(&key_input(Key::Tab, Modifiers::default()), routing),
        None
    );
    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_input(Key::Tab, Modifiers::new(true, false, false, false)),
            routing,
        ),
        None
    );

    for character in ["a", "c", "v", "x", "y", "z"] {
        assert_eq!(
            router.resolve_shortcut_in_context(
                &key_input(Key::Character(character.to_owned()), ctrl()),
                routing,
            ),
            None
        );
        assert_eq!(
            router.resolve_shortcut_in_context(
                &key_input(Key::Character(character.to_owned()), super_key()),
                routing,
            ),
            None
        );
    }
}

#[test]
fn focus_keyboard_text_input_allows_scoped_editing_binding_and_non_reserved_global_shortcut() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind_global(&mut router, "global.select.all", ctrl_shortcut("a"));
    bind_global(&mut router, "file.save", ctrl_shortcut("s"));
    router.bind(ActionBinding::new(
        shortcut_action("text.select.all", ctrl_shortcut("a")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    ));

    let routing = ActionRoutingContext::new().with_text_input(field);
    let text_invocation = router
        .resolve_shortcut_in_context(&key_input(Key::Character("a".to_owned()), ctrl()), routing)
        .expect("text input scoped action");
    assert_eq!(text_invocation.action_id, ActionId::new("text.select.all"));
    assert_eq!(text_invocation.context, ActionContext::TextInput(field));

    let global_invocation = router
        .resolve_shortcut_in_context(&key_input(Key::Character("s".to_owned()), ctrl()), routing)
        .expect("non-reserved global action");
    assert_eq!(global_invocation.action_id, ActionId::new("file.save"));
    assert_eq!(global_invocation.context, ActionContext::Global);
}

#[test]
fn focus_keyboard_physical_shortcut_remains_layout_independent() {
    let modifiers = ctrl();
    let shortcut = Shortcut::physical(modifiers, PhysicalKey::KeyY);

    assert!(shortcut.matches_keyboard(&physical_input("z", PhysicalKey::KeyY, modifiers)));
    assert!(!shortcut.matches_keyboard(&physical_input("y", PhysicalKey::KeyZ, modifiers)));
    assert!(!shortcut.matches_keyboard(&key_input(Key::Character("y".to_owned()), modifiers)));
}

#[test]
fn focus_keyboard_text_input_reservation_still_blocks_global_physical_editing_shortcut() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind_global(
        &mut router,
        "global.undo",
        Shortcut::physical(ctrl(), PhysicalKey::KeyZ),
    );

    assert_eq!(
        router.resolve_shortcut_in_context(
            &physical_input("w", PhysicalKey::KeyZ, ctrl()),
            ActionRoutingContext::new().with_text_input(field),
        ),
        None
    );
}
