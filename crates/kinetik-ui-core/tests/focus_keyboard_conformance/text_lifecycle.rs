use kinetik_ui_core::{
    ActionRouter, ActionRoutingContext, FrameWarning, Key, Modifiers, MouseButton, PlatformRequest,
    Point, Rect, ScriptedInput, SemanticNode, SemanticRole, SemanticTree, SemanticTreeError,
    Shortcut, TextInputEvent, TextRange, UiTestHarness, WidgetId,
};

use crate::support::{
    bind_global, ctrl_shortcut, emit_tree, focus_tree, ids, text_owner_tree,
    text_owner_tree_with_disabled_text_field,
};

#[test]
fn focus_keyboard_runtime_tab_from_text_owner_traverses_forward_and_stops_input() {
    let (_, first, second, _) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(second);
    harness.memory_mut().set_text_input_owner(second);
    harness.key_press(Key::Tab);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(first));
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(output.repaint, kinetik_ui_core::RepaintRequest::NextFrame);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_shift_tab_from_text_owner_traverses_backward_and_stops_input() {
    let (_, _, second, third) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(second);
    harness.memory_mut().set_text_input_owner(second);
    harness.set_modifiers(Modifiers::new(true, false, false, false));
    harness.key_press(Key::Tab);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(third));
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(output.repaint, kinetik_ui_core::RepaintRequest::NextFrame);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_escape_clears_text_owner_and_stops_input() {
    let (_, _, second, _) = ids();
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(second);
    harness.memory_mut().set_text_input_owner(second);
    harness.key_press(Key::Escape);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(output.repaint, kinetik_ui_core::RepaintRequest::NextFrame);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_primary_press_outside_text_owner_clears_focus_and_stops_input() {
    let owner = WidgetId::from_key("field");
    let owner_rect = Rect::new(10.0, 10.0, 120.0, 24.0);
    let tree = text_owner_tree(owner, owner_rect);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_pointer_position(Point::new(200.0, 100.0));
    harness.pointer_press(MouseButton::Primary);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_primary_press_inside_text_owner_preserves_focus_and_input() {
    let owner = WidgetId::from_key("field");
    let owner_rect = Rect::new(10.0, 10.0, 120.0, 24.0);
    let tree = text_owner_tree(owner, owner_rect);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.pointer_press(MouseButton::Primary);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(owner));
    assert_eq!(harness.memory().text_input_owner(), Some(owner));
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_primary_press_on_disabled_text_field_preserves_text_owner() {
    let owner = WidgetId::from_key("owner");
    let disabled = WidgetId::from_key("disabled");
    let tree = text_owner_tree_with_disabled_text_field(
        owner,
        Rect::new(10.0, 10.0, 120.0, 24.0),
        disabled,
        Rect::new(10.0, 44.0, 120.0, 24.0),
    );
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_pointer_position(Point::new(20.0, 52.0));
    harness.pointer_press(MouseButton::Primary);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), Some(owner));
    assert_eq!(harness.memory().text_input_owner(), Some(owner));
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_window_focus_loss_clears_text_owner_and_stops_input() {
    let owner = WidgetId::from_key("field");
    let tree = text_owner_tree(owner, Rect::new(10.0, 10.0, 120.0, 24.0));
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_window_focused(false);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_runtime_window_focus_loss_clears_absent_text_owner_and_stops_input() {
    let owner = WidgetId::from_key("field");
    let tree = focus_tree();
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_window_focused(false);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn focus_keyboard_window_focus_loss_clears_text_owner_with_invalid_semantic_tree() {
    let owner = WidgetId::from_key("field");
    let missing = WidgetId::from_key("missing-child");
    let root = WidgetId::from_key("root");
    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([owner, missing]),
    );
    tree.push(
        SemanticNode::new(
            owner,
            SemanticRole::TextField,
            Rect::new(10.0, 10.0, 120.0, 24.0),
        )
        .focusable(true),
    );

    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);
    harness.set_window_focused(false);

    let ((), output) = harness.run_frame(|ui| emit_tree(ui, &tree));

    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
    assert_eq!(
        output.warnings,
        vec![FrameWarning::InvalidSemanticTree {
            error: SemanticTreeError::UnknownChild {
                parent: root,
                child: missing,
            },
        }]
    );
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
fn focus_keyboard_focused_text_widget_starts_input_and_observes_same_frame_commit() {
    let field = WidgetId::from_key("field");
    let rect = Rect::new(12.0, 24.0, 160.0, 20.0);
    let mut harness = UiTestHarness::new();

    let (observed, output) =
        harness.run_scripted_frame([ScriptedInput::TextCommit("hello".to_owned())], |ui| {
            ui.memory_mut().focus(field);
            assert!(ui.start_text_input(field, Some(rect)));
            assert_eq!(ui.memory().text_input_owner(), Some(field));

            let owner = ui.memory().text_input_owner();
            ui.input()
                .text_events
                .iter()
                .filter_map(|event| match event {
                    TextInputEvent::Commit(text) => Some((owner, text.clone())),
                    TextInputEvent::CompositionStart
                    | TextInputEvent::Composition { .. }
                    | TextInputEvent::CompositionEnd => None,
                })
                .collect::<Vec<_>>()
        });

    assert_eq!(observed, vec![(Some(field), "hello".to_owned())]);
    assert_eq!(harness.memory().text_input_owner(), Some(field));
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: Some(rect) }]
    );
}

#[test]
fn focus_keyboard_same_frame_text_input_handoff_observes_commit_for_new_owner() {
    let old_field = WidgetId::from_key("old-field");
    let new_field = WidgetId::from_key("new-field");
    let rect = Rect::new(8.0, 12.0, 120.0, 18.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(old_field);
    harness.memory_mut().set_text_input_owner(old_field);

    let (observed, output) =
        harness.run_scripted_frame([ScriptedInput::TextCommit("handoff".to_owned())], |ui| {
            ui.memory_mut().focus(new_field);
            assert!(ui.start_text_input(new_field, Some(rect)));

            let owner = ui.memory().text_input_owner();
            ui.input()
                .text_events
                .iter()
                .filter_map(|event| match event {
                    TextInputEvent::Commit(text) => Some((owner, text.clone())),
                    TextInputEvent::CompositionStart
                    | TextInputEvent::Composition { .. }
                    | TextInputEvent::CompositionEnd => None,
                })
                .collect::<Vec<_>>()
        });

    assert_eq!(observed, vec![(Some(new_field), "handoff".to_owned())]);
    assert_eq!(harness.memory().focused(), Some(new_field));
    assert_eq!(harness.memory().text_input_owner(), Some(new_field));
    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::StopTextInput,
            PlatformRequest::StartTextInput { rect: Some(rect) },
        ]
    );
}

#[test]
fn focus_keyboard_starting_text_input_allows_missing_logical_rect() {
    let field = WidgetId::from_key("field");
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(field);

    let (started, output) = harness.run_frame(|ui| ui.start_text_input(field, None));

    assert!(started);
    assert_eq!(harness.memory().text_input_owner(), Some(field));
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: None }]
    );
}

#[test]
fn focus_keyboard_starting_current_text_owner_does_not_churn_platform_requests() {
    let field = WidgetId::from_key("field");
    let rect = Rect::new(12.0, 24.0, 160.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(field);
    harness.memory_mut().set_text_input_owner(field);

    let (started, output) = harness.run_frame(|ui| ui.start_text_input(field, Some(rect)));

    assert!(started);
    assert_eq!(harness.memory().text_input_owner(), Some(field));
    assert!(output.platform_requests.is_empty());
}

#[test]
fn focus_keyboard_text_composition_events_do_not_route_global_shortcut_actions() {
    let field = WidgetId::from_key("field");
    let mut router = ActionRouter::new();
    bind_global(
        &mut router,
        "global.type.k",
        Shortcut::new(Modifiers::default(), Key::Character("k".to_owned())),
    );
    bind_global(&mut router, "global.save", ctrl_shortcut("s"));
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(field);
    harness.memory_mut().set_text_input_owner(field);

    let (text_events, output) = harness.run_scripted_frame_with_action_router(
        [
            ScriptedInput::TextCompositionStart,
            ScriptedInput::TextComposition {
                text: "ka".to_owned(),
                selection: Some(TextRange::new(1, 2)),
            },
            ScriptedInput::TextCommit("か".to_owned()),
            ScriptedInput::TextCompositionEnd,
        ],
        &router,
        ActionRoutingContext::new().with_text_input(field),
        |ui| {
            assert_eq!(ui.memory().text_input_owner(), Some(field));
            ui.input().text_events.clone()
        },
    );

    assert_eq!(
        text_events,
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
    assert!(output.actions.is_empty());
}
