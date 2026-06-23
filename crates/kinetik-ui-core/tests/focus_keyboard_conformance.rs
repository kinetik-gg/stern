//! Windowless focus traversal, keyboard activation, and shortcut conformance.

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionId, ActionPriority, ActionRouter,
    ActionRoutingContext, FocusTraversal, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    MouseButton, PhysicalKey, PlatformRequest, Point, Rect, SemanticNode, SemanticRole,
    SemanticTree, Shortcut, Ui, UiTestHarness, WidgetId, focusable, pressable,
};

fn ctrl() -> Modifiers {
    Modifiers::new(false, true, false, false)
}

fn super_key() -> Modifiers {
    Modifiers::new(false, false, false, true)
}

fn key_input(key: Key, modifiers: Modifiers) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
    }
}

fn physical_input(
    character: &str,
    physical_key: PhysicalKey,
    modifiers: Modifiers,
) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::with_physical_key(
            Key::Character(character.to_owned()),
            physical_key,
            KeyState::Pressed,
            modifiers,
            false,
        )],
    }
}

fn shortcut_action(id: &str, shortcut: Shortcut) -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(id, id);
    descriptor.shortcut = Some(shortcut);
    descriptor
}

fn ctrl_shortcut(character: &str) -> Shortcut {
    Shortcut::new(ctrl(), Key::Character(character.to_owned()))
}

fn bind_global(router: &mut ActionRouter, id: &str, shortcut: Shortcut) {
    router.bind(ActionBinding::new(
        shortcut_action(id, shortcut),
        ActionContext::Global,
        ActionPriority::Global,
    ));
}

fn ids() -> (WidgetId, WidgetId, WidgetId, WidgetId) {
    (
        WidgetId::from_key("root"),
        WidgetId::from_key("first"),
        WidgetId::from_key("second"),
        WidgetId::from_key("third"),
    )
}

fn focus_tree() -> SemanticTree {
    let (root, first, second, third) = ids();
    let disabled = WidgetId::from_key("disabled");
    let mut disabled_node =
        SemanticNode::new(disabled, SemanticRole::Button, Rect::ZERO).focusable(true);
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO)
            .with_children([second, disabled, first, third]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(disabled_node);
    tree.push(SemanticNode::new(third, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree
}

fn focus_tree_with_non_focusable() -> SemanticTree {
    let (root, first, second, third) = ids();
    let disabled = WidgetId::from_key("disabled");
    let label = WidgetId::from_key("label");
    let mut disabled_node =
        SemanticNode::new(disabled, SemanticRole::Button, Rect::ZERO).focusable(true);
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO)
            .with_children([second, disabled, label, first, third]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(disabled_node);
    tree.push(SemanticNode::new(label, SemanticRole::Label, Rect::ZERO));
    tree.push(SemanticNode::new(third, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree
}

fn emit_tree(ui: &mut Ui<'_>, tree: &SemanticTree) {
    if let Some(root) = tree.root() {
        ui.set_semantic_root(root);
    }
    for node in tree.nodes().iter().cloned() {
        ui.push_semantic_node(node);
    }
}

fn click_focusable(
    harness: &mut UiTestHarness,
    id: WidgetId,
    rect: Rect,
    disabled: bool,
) -> kinetik_ui_core::Response {
    harness.set_pointer_position(Point::new(
        rect.min_x() + rect.width * 0.5,
        rect.min_y() + rect.height * 0.5,
    ));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let (input, memory) = ui.input_and_memory_mut();
        focusable(id, rect, input, memory, disabled)
    });
    harness.pointer_release(MouseButton::Primary);
    harness
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            focusable(id, rect, input, memory, disabled)
        })
        .0
}

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

#[test]
fn focus_keyboard_runtime_text_input_owner_blocks_tab_and_shift_tab_traversal() {
    let (_, _, second, _) = ids();
    let tree = focus_tree();

    for modifiers in [
        Modifiers::default(),
        Modifiers::new(true, false, false, false),
    ] {
        let mut harness = UiTestHarness::new();
        harness.memory_mut().focus(second);
        harness.memory_mut().set_text_input_owner(second);
        harness.set_modifiers(modifiers);
        harness.key_press(Key::Tab);

        let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

        assert_eq!(harness.memory().focused(), Some(second));
        assert_eq!(output.repaint, kinetik_ui_core::RepaintRequest::None);
    }
}

#[test]
fn focus_keyboard_focus_change_retires_stale_text_input_owner() {
    let field = WidgetId::from_key("field");
    let button = WidgetId::from_key("button");
    let mut harness = UiTestHarness::new();

    harness.memory_mut().focus(field);
    harness.memory_mut().set_text_input_owner(field);
    harness.memory_mut().focus(button);

    assert_eq!(harness.memory().focused(), Some(button));
    assert_eq!(harness.memory().text_input_owner(), None);

    let ((), output) = harness.run_frame(|_| {});

    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

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
            &physical_input("z", PhysicalKey::KeyZ, ctrl()),
            ActionRoutingContext::new().with_text_input(field),
        ),
        None
    );
}
