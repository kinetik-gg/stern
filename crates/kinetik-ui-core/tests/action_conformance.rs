//! Windowless action descriptor, queue, invocation, and routing conformance.

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation,
    ActionPriority, ActionQueue, ActionRouter, ActionRoutingContext, ActionSource, ActionState,
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalKey, Shortcut, WidgetId,
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
    let text_routing = ActionRoutingContext::new()
        .with_panel(panel)
        .with_focused_widget(widget)
        .with_text_input(field);
    let widget_routing = ActionRoutingContext::new()
        .with_panel(panel)
        .with_focused_widget(widget);
    let panel_routing = ActionRoutingContext::new().with_panel(panel);

    assert_eq!(
        router
            .resolve_shortcut_in_context(&input, text_routing)
            .expect("text action")
            .action_id,
        ActionId::new("text")
    );
    assert_eq!(
        router
            .resolve_shortcut_in_context(&input, widget_routing)
            .expect("widget action")
            .action_id,
        ActionId::new("widget")
    );
    assert_eq!(
        router
            .resolve_shortcut_in_context(&input, panel_routing)
            .expect("panel action")
            .action_id,
        ActionId::new("panel")
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
fn action_conformance_router_ignores_inactive_contextual_shortcuts() {
    let active = WidgetId::from_key("active");
    let inactive = WidgetId::from_key("inactive");
    let mut router = ActionRouter::new();
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

    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_press(Key::Character("r".to_owned()), ctrl()),
            ActionRoutingContext::new().with_focused_widget(active),
        ),
        None
    );
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
    let widget = WidgetId::from_key("button");
    let mut hidden = action_with_shortcut("hidden", ctrl_shortcut("h"));
    hidden.state.visible = false;
    let enabled = action_with_shortcut("global.enabled", ctrl_shortcut("h"));
    let mut router = ActionRouter::new();
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
            ActionRoutingContext::new().with_focused_widget(widget),
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
