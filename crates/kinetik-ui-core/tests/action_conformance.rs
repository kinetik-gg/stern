//! Windowless action descriptor, queue, invocation, and routing conformance.

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation,
    ActionPriority, ActionQueue, ActionRouter, ActionRoutingContext, ActionSource, ActionState,
    FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalKey, RepaintRequest,
    ScriptedInput, Shortcut, UiTestHarness, WidgetId,
};

fn ctrl() -> Modifiers {
    Modifiers::new(false, true, false, false)
}

fn ctrl_shortcut(character: &str) -> Shortcut {
    Shortcut::new(ctrl(), Key::Character(character.to_owned()))
}

fn keyboard_event(key: Key, modifiers: Modifiers, state: KeyState, repeat: bool) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::new(key, state, modifiers, repeat)],
    }
}

fn key_press(key: Key, modifiers: Modifiers) -> KeyboardInput {
    keyboard_event(key, modifiers, KeyState::Pressed, false)
}

fn key_presses(events: Vec<(Key, Modifiers)>) -> KeyboardInput {
    KeyboardInput {
        modifiers: Modifiers::default(),
        events: events
            .into_iter()
            .map(|(key, modifiers)| KeyEvent::new(key, KeyState::Pressed, modifiers, false))
            .collect(),
    }
}

fn physical_key_press(key: Key, physical_key: PhysicalKey, modifiers: Modifiers) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::with_physical_key(
            key,
            physical_key,
            KeyState::Pressed,
            modifiers,
            false,
        )],
    }
}

fn action_with_shortcut(id: &str, shortcut: Shortcut) -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(id, id);
    descriptor.shortcut = Some(shortcut);
    descriptor
}

fn bind(
    router: &mut ActionRouter,
    descriptor: ActionDescriptor,
    context: ActionContext,
    priority: ActionPriority,
) {
    router.bind(ActionBinding::new(descriptor, context, priority));
}

fn assert_shortcut_routes_to(
    router: &ActionRouter,
    input: &KeyboardInput,
    routing: ActionRoutingContext,
    expected: &str,
) {
    assert_eq!(
        router
            .resolve_shortcut_in_context(input, routing)
            .expect("shortcut action")
            .action_id,
        ActionId::new(expected)
    );
}

fn frame_action_ids(output: &FrameOutput) -> Vec<ActionId> {
    output
        .actions
        .clone()
        .drain()
        .map(|invocation| invocation.action_id)
        .collect()
}

#[test]
fn action_conformance_descriptors_expose_presentation_state() {
    let shortcut = ctrl_shortcut("g");
    let mut descriptor = ActionDescriptor::new("view.grid", "Show Grid");
    descriptor.icon = Some(ActionIcon::new("grid"));
    descriptor.tooltip = Some("Toggle viewport grid".to_owned());
    descriptor.keywords = vec!["guide".to_owned(), "overlay".to_owned()];
    descriptor.shortcut = Some(shortcut.clone());
    descriptor.state = ActionState {
        visible: true,
        enabled: true,
        checked: Some(false),
    };

    assert_eq!(descriptor.id, ActionId::new("view.grid"));
    assert_eq!(descriptor.label, "Show Grid");
    assert_eq!(
        descriptor.icon.as_ref().map(ActionIcon::as_str),
        Some("grid")
    );
    assert_eq!(descriptor.tooltip.as_deref(), Some("Toggle viewport grid"));
    assert_eq!(descriptor.keywords, ["guide", "overlay"]);
    assert_eq!(descriptor.shortcut.as_ref(), Some(&shortcut));
    assert!(descriptor.state.visible);
    assert!(descriptor.state.enabled);
    assert_eq!(descriptor.state.checked, Some(false));
    assert!(descriptor.state.is_checkable());
    assert!(!descriptor.state.is_checked());
    assert!(descriptor.can_invoke());
}

#[test]
fn action_conformance_disabled_and_hidden_descriptors_cannot_invoke() {
    let mut disabled = ActionDescriptor::new("file.save", "Save");
    disabled.state.enabled = false;
    let mut hidden = ActionDescriptor::new("debug.hidden", "Hidden");
    hidden.state.visible = false;

    assert!(!disabled.can_invoke());
    assert!(!hidden.can_invoke());
}

#[test]
fn action_conformance_queue_drains_fifo() {
    let frame = WidgetId::from_key("frame");
    let panel = WidgetId::from_key("panel");
    let mut queue = ActionQueue::new();
    queue.push(ActionInvocation::new(
        ActionId::new("one"),
        ActionSource::Button,
        ActionContext::Global,
    ));
    queue.invoke(
        ActionId::new("two"),
        ActionSource::Menu,
        ActionContext::Frame(frame),
    );
    queue.push(ActionInvocation::new(
        ActionId::new("three"),
        ActionSource::CommandPalette,
        ActionContext::Panel(panel),
    ));

    assert_eq!(queue.len(), 3);
    assert_eq!(
        queue.pop_front().expect("first").action_id,
        ActionId::new("one")
    );
    assert_eq!(
        queue
            .drain()
            .map(|invocation| (invocation.action_id, invocation.source, invocation.context,))
            .collect::<Vec<_>>(),
        vec![
            (
                ActionId::new("two"),
                ActionSource::Menu,
                ActionContext::Frame(frame),
            ),
            (
                ActionId::new("three"),
                ActionSource::CommandPalette,
                ActionContext::Panel(panel),
            ),
        ]
    );
    assert!(queue.is_empty());
    assert_eq!(queue.drain().count(), 0);
}

#[test]
fn action_conformance_invocations_preserve_source_and_context_snapshots() {
    let action_id = ActionId::new("project.run");
    let frame = WidgetId::from_key("frame");
    let widget = WidgetId::from_key("toolbar.button");
    let mut queue = ActionQueue::new();

    queue.invoke(
        action_id.clone(),
        ActionSource::Button,
        ActionContext::Widget(widget),
    );
    queue.invoke(
        action_id.clone(),
        ActionSource::Menu,
        ActionContext::Frame(frame),
    );
    queue.invoke(
        action_id.clone(),
        ActionSource::CommandPalette,
        ActionContext::Global,
    );

    let invocations = queue.drain().collect::<Vec<_>>();
    assert!(
        invocations
            .iter()
            .all(|invocation| invocation.action_id == action_id)
    );
    assert_eq!(
        invocations
            .iter()
            .map(|invocation| invocation.source)
            .collect::<Vec<_>>(),
        vec![
            ActionSource::Button,
            ActionSource::Menu,
            ActionSource::CommandPalette,
        ]
    );
    assert_eq!(
        invocations
            .iter()
            .map(|invocation| invocation.context.clone())
            .collect::<Vec<_>>(),
        vec![
            ActionContext::Widget(widget),
            ActionContext::Frame(frame),
            ActionContext::Global,
        ]
    );
}

#[test]
fn action_conformance_router_respects_active_context_priority() {
    let frame = WidgetId::from_key("frame");
    let panel = WidgetId::from_key("panel");
    let widget = WidgetId::from_key("button");
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global", ctrl_shortcut("k")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("editor", ctrl_shortcut("k")),
        ActionContext::Editor,
        ActionPriority::Editor,
    );
    bind(
        &mut router,
        action_with_shortcut("frame", ctrl_shortcut("k")),
        ActionContext::Frame(frame),
        ActionPriority::Container,
    );
    bind(
        &mut router,
        action_with_shortcut("panel", ctrl_shortcut("k")),
        ActionContext::Panel(panel),
        ActionPriority::Container,
    );
    bind(
        &mut router,
        action_with_shortcut("widget", ctrl_shortcut("k")),
        ActionContext::Widget(widget),
        ActionPriority::FocusedWidget,
    );
    bind(
        &mut router,
        action_with_shortcut("text", ctrl_shortcut("k")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    );

    let input = key_press(Key::Character("k".to_owned()), ctrl());

    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new()
            .with_editor()
            .with_panel(panel)
            .with_focused_widget(widget)
            .with_text_input(field),
        "text",
    );
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new()
            .with_editor()
            .with_panel(panel)
            .with_focused_widget(widget),
        "widget",
    );
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new().with_panel(panel),
        "panel",
    );
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new().with_frame(frame),
        "frame",
    );
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new().with_editor(),
        "editor",
    );
    assert_eq!(
        router
            .resolve_shortcut(&input)
            .expect("global action")
            .action_id,
        ActionId::new("global")
    );
}

#[test]
fn action_conformance_modal_context_beats_lower_priority_active_contexts() {
    let modal = WidgetId::from_key("modal");
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global", ctrl_shortcut("k")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("text", ctrl_shortcut("k")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    );
    bind(
        &mut router,
        action_with_shortcut("modal", ctrl_shortcut("k")),
        ActionContext::Modal(modal),
        ActionPriority::Modal,
    );

    let input = key_press(Key::Character("k".to_owned()), ctrl());
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new()
            .with_text_input(field)
            .with_modal(modal),
        "modal",
    );
    assert_shortcut_routes_to(
        &router,
        &input,
        ActionRoutingContext::new().with_text_input(field),
        "text",
    );
}

#[test]
fn action_conformance_router_preserves_first_registered_equal_priority() {
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("first", ctrl_shortcut("p")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("second", ctrl_shortcut("p")),
        ActionContext::Global,
        ActionPriority::Global,
    );

    let invocation = router
        .resolve_shortcut(&key_press(Key::Character("p".to_owned()), ctrl()))
        .expect("shortcut invocation");

    assert_eq!(invocation.action_id, ActionId::new("first"));
}

#[test]
fn action_conformance_scripted_ctrl_s_routes_to_global_frame_action() {
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("file.save", ctrl_shortcut("s")),
        ActionContext::Global,
        ActionPriority::Global,
    );

    let mut harness = UiTestHarness::new();
    let ((), output) = harness.run_scripted_frame_with_action_router(
        [ScriptedInput::key_press(
            Key::Character("s".to_owned()),
            ctrl(),
        )],
        &router,
        ActionRoutingContext::new(),
        |_| {},
    );

    assert_eq!(frame_action_ids(&output), vec![ActionId::new("file.save")]);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert!(harness.input().keyboard.events.is_empty());
}

#[test]
fn action_conformance_scripted_shortcuts_emit_frame_actions_in_event_order() {
    let modal = WidgetId::from_key("modal");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("file.save", ctrl_shortcut("s")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("modal.find", ctrl_shortcut("f")),
        ActionContext::Modal(modal),
        ActionPriority::Modal,
    );

    let mut harness = UiTestHarness::new();
    let ((), output) = harness.run_scripted_frame_with_action_router(
        [
            ScriptedInput::key_press(Key::Character("s".to_owned()), ctrl()),
            ScriptedInput::key_press(Key::Character("f".to_owned()), ctrl()),
        ],
        &router,
        ActionRoutingContext::new().with_modal(modal),
        |_| {},
    );

    assert_eq!(
        frame_action_ids(&output),
        vec![ActionId::new("file.save"), ActionId::new("modal.find")]
    );
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn action_conformance_scripted_text_input_blocks_reserved_global_but_allows_text_binding() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global.select.all", ctrl_shortcut("a")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("text.select.all", ctrl_shortcut("a")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    );
    bind(
        &mut router,
        action_with_shortcut("global.save", ctrl_shortcut("s")),
        ActionContext::Global,
        ActionPriority::Global,
    );

    let mut harness = UiTestHarness::new();
    let routing = ActionRoutingContext::new().with_text_input(field);
    let ((), select_output) = harness.run_scripted_frame_with_action_router(
        [ScriptedInput::key_press(
            Key::Character("a".to_owned()),
            ctrl(),
        )],
        &router,
        routing,
        |_| {},
    );

    assert_eq!(
        frame_action_ids(&select_output),
        vec![ActionId::new("text.select.all")]
    );

    let ((), typing_output) = harness.run_scripted_frame_with_action_router(
        [ScriptedInput::key_press(
            Key::Character("x".to_owned()),
            Modifiers::default(),
        )],
        &router,
        routing,
        |_| {},
    );
    assert!(frame_action_ids(&typing_output).is_empty());

    let ((), save_output) = harness.run_scripted_frame_with_action_router(
        [ScriptedInput::key_press(
            Key::Character("s".to_owned()),
            ctrl(),
        )],
        &router,
        routing,
        |_| {},
    );
    assert_eq!(
        frame_action_ids(&save_output),
        vec![ActionId::new("global.save")]
    );
}

#[test]
fn action_conformance_scripted_modal_shortcut_beats_text_widget_and_global() {
    let modal = WidgetId::from_key("modal");
    let field = WidgetId::from_key("field");
    let widget = WidgetId::from_key("widget");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global", ctrl_shortcut("k")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("widget", ctrl_shortcut("k")),
        ActionContext::Widget(widget),
        ActionPriority::FocusedWidget,
    );
    bind(
        &mut router,
        action_with_shortcut("text", ctrl_shortcut("k")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    );
    bind(
        &mut router,
        action_with_shortcut("modal", ctrl_shortcut("k")),
        ActionContext::Modal(modal),
        ActionPriority::Modal,
    );

    let mut harness = UiTestHarness::new();
    let ((), output) = harness.run_scripted_frame_with_action_router(
        [ScriptedInput::key_press(
            Key::Character("k".to_owned()),
            ctrl(),
        )],
        &router,
        ActionRoutingContext::new()
            .with_focused_widget(widget)
            .with_text_input(field)
            .with_modal(modal),
        |_| {},
    );

    assert_eq!(frame_action_ids(&output), vec![ActionId::new("modal")]);
}

#[test]
fn action_conformance_router_ignores_inactive_contextual_shortcuts() {
    let active = WidgetId::from_key("active");
    let inactive = WidgetId::from_key("inactive");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global.active", ctrl_shortcut("r")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("inactive.widget", ctrl_shortcut("r")),
        ActionContext::Widget(inactive),
        ActionPriority::FocusedWidget,
    );
    bind(
        &mut router,
        action_with_shortcut("inactive.text", ctrl_shortcut("r")),
        ActionContext::TextInput(inactive),
        ActionPriority::TextInput,
    );
    bind(
        &mut router,
        action_with_shortcut("inactive.modal", ctrl_shortcut("r")),
        ActionContext::Modal(inactive),
        ActionPriority::Modal,
    );

    let invocation = router
        .resolve_shortcut_in_context(
            &key_press(Key::Character("r".to_owned()), ctrl()),
            ActionRoutingContext::new().with_focused_widget(active),
        )
        .expect("lower-priority active fallback");

    assert_eq!(invocation.action_id, ActionId::new("global.active"));
    assert_eq!(invocation.context, ActionContext::Global);
}

#[test]
fn action_conformance_router_skips_hidden_and_disabled_bindings() {
    let widget = WidgetId::from_key("button");
    let mut hidden = action_with_shortcut("hidden", ctrl_shortcut("h"));
    hidden.state.visible = false;
    let mut disabled = action_with_shortcut("disabled", ctrl_shortcut("h"));
    disabled.state.enabled = false;
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        hidden,
        ActionContext::Widget(widget),
        ActionPriority::FocusedWidget,
    );
    bind(
        &mut router,
        disabled,
        ActionContext::Global,
        ActionPriority::Global,
    );

    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_press(Key::Character("h".to_owned()), ctrl()),
            ActionRoutingContext::new().with_focused_widget(widget),
        ),
        None
    );
}

#[test]
fn action_conformance_router_uses_lower_priority_visible_enabled_fallback() {
    let modal = WidgetId::from_key("modal");
    let widget = WidgetId::from_key("button");
    let mut hidden = action_with_shortcut("hidden", ctrl_shortcut("h"));
    hidden.state.visible = false;
    let mut disabled = action_with_shortcut("disabled", ctrl_shortcut("h"));
    disabled.state.enabled = false;
    let enabled = action_with_shortcut("global.enabled", ctrl_shortcut("h"));
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        disabled,
        ActionContext::Modal(modal),
        ActionPriority::Modal,
    );
    bind(
        &mut router,
        hidden,
        ActionContext::Widget(widget),
        ActionPriority::FocusedWidget,
    );
    bind(
        &mut router,
        enabled,
        ActionContext::Global,
        ActionPriority::Global,
    );

    let invocation = router
        .resolve_shortcut_in_context(
            &key_press(Key::Character("h".to_owned()), ctrl()),
            ActionRoutingContext::new()
                .with_focused_widget(widget)
                .with_modal(modal),
        )
        .expect("fallback invocation");

    assert_eq!(invocation.action_id, ActionId::new("global.enabled"));
    assert_eq!(invocation.source, ActionSource::Shortcut);
    assert_eq!(invocation.context, ActionContext::Global);
}

#[test]
fn action_conformance_text_input_reservation_blocks_only_reserved_global_shortcuts() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global.select.all", ctrl_shortcut("a")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("global.save", ctrl_shortcut("s")),
        ActionContext::Global,
        ActionPriority::Global,
    );
    bind(
        &mut router,
        action_with_shortcut("text.select.all", ctrl_shortcut("a")),
        ActionContext::TextInput(field),
        ActionPriority::TextInput,
    );
    bind(
        &mut router,
        action_with_shortcut(
            "global.type.x",
            Shortcut::new(Modifiers::default(), Key::Character("x".to_owned())),
        ),
        ActionContext::Global,
        ActionPriority::Global,
    );

    let routing = ActionRoutingContext::new().with_text_input(field);
    assert_eq!(
        router
            .resolve_shortcut_in_context(
                &key_press(Key::Character("a".to_owned()), ctrl()),
                routing,
            )
            .expect("text-scoped action")
            .action_id,
        ActionId::new("text.select.all")
    );
    assert_eq!(
        router
            .resolve_shortcut_in_context(
                &key_press(Key::Character("s".to_owned()), ctrl()),
                routing,
            )
            .expect("non-reserved global action")
            .action_id,
        ActionId::new("global.save")
    );
    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_press(Key::Character("x".to_owned()), Modifiers::default(),),
            routing,
        ),
        None
    );
    assert_eq!(
        router
            .resolve_shortcut_in_context(
                &key_presses(vec![
                    (Key::Character("x".to_owned()), Modifiers::default(),),
                    (Key::Character("s".to_owned()), ctrl()),
                ]),
                routing,
            )
            .expect("mixed non-reserved global action")
            .action_id,
        ActionId::new("global.save")
    );
}

#[test]
fn action_conformance_physical_shortcuts_remain_layout_independent() {
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut(
            "physical.undo",
            Shortcut::physical(ctrl(), PhysicalKey::KeyZ),
        ),
        ActionContext::Global,
        ActionPriority::Global,
    );

    assert_eq!(
        router
            .resolve_shortcut(&physical_key_press(
                Key::Character("y".to_owned()),
                PhysicalKey::KeyZ,
                ctrl(),
            ))
            .expect("physical action")
            .action_id,
        ActionId::new("physical.undo")
    );
    assert_eq!(
        router.resolve_shortcut(&physical_key_press(
            Key::Character("z".to_owned()),
            PhysicalKey::KeyY,
            ctrl(),
        )),
        None
    );
    assert_eq!(
        router.resolve_shortcut(&key_press(Key::Character("z".to_owned()), ctrl())),
        None
    );
}

#[test]
fn action_conformance_text_input_reserves_physical_editing_shortcuts_across_layouts() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("global.undo", Shortcut::physical(ctrl(), PhysicalKey::KeyZ)),
        ActionContext::Global,
        ActionPriority::Global,
    );

    assert_eq!(
        router.resolve_shortcut_in_context(
            &physical_key_press(Key::Character("w".to_owned()), PhysicalKey::KeyZ, ctrl(),),
            ActionRoutingContext::new().with_text_input(field),
        ),
        None
    );
}

#[test]
fn action_conformance_shortcut_routing_ignores_releases_and_repeats() {
    let mut router = ActionRouter::new();
    bind(
        &mut router,
        action_with_shortcut("file.save", ctrl_shortcut("s")),
        ActionContext::Global,
        ActionPriority::Global,
    );

    assert_eq!(
        router.resolve_shortcut(&keyboard_event(
            Key::Character("s".to_owned()),
            ctrl(),
            KeyState::Released,
            false,
        )),
        None
    );
    assert_eq!(
        router.resolve_shortcut(&keyboard_event(
            Key::Character("s".to_owned()),
            ctrl(),
            KeyState::Pressed,
            true,
        )),
        None
    );
}
