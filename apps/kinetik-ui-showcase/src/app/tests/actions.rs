use std::collections::HashSet;

use super::helpers::{
    ACTION_COMMAND_PALETTE, ACTION_COMPONENTS_RUN, ACTION_EDITOR_DOCK_JOIN,
    ACTION_SYSTEMS_DISPATCH, ACTION_VIEWPORT_GRID, ACTION_WORKSPACE_SAVE, ActionContext, ActionId,
    ActionInvocation, ActionSource, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point,
    ShowcaseApp, ShowcasePage, click, showcase_action_router, showcase_actions,
};

#[test]
fn clicking_button_changes_action_state() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(70.0, 154.0));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.component_action_count, 1);
    assert_eq!(app.status, "Component demo counter: 1");
}

#[test]
fn unknown_action_invocation_is_not_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new("showcase.unknown"),
        ActionSource::Button,
        ActionContext::Global,
    );

    app.handle_action_invocation(&invocation);

    assert_eq!(app.action_count(), 0);
    assert_eq!(
        app.status,
        "Ignored unhandled action showcase.unknown via Button"
    );
}

#[test]
fn unhandled_editor_action_invocation_is_not_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new("editor.unknown"),
        ActionSource::Button,
        ActionContext::Editor,
    );

    assert!(!app.handle_action_invocation(&invocation));

    assert_eq!(app.action_count(), 0);
    assert_eq!(
        app.status,
        "Ignored unhandled action editor.unknown via Button"
    );
}

#[test]
fn editor_rendered_dock_action_invocation_is_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new(ACTION_EDITOR_DOCK_JOIN),
        ActionSource::Button,
        ActionContext::Editor,
    );

    assert!(app.handle_action_invocation(&invocation));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.status, "Ready");
    assert!(!app.status.contains("Ignored unhandled action"));
}

#[test]
fn explicit_showcase_demo_action_is_counted() {
    let mut app = ShowcaseApp::new();

    assert!(app.invoke_action(ACTION_SYSTEMS_DISPATCH, ActionSource::Button));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.systems_dispatch_count, 1);
    assert_eq!(app.status, "Systems dispatches: 1");
}

#[test]
fn showcase_action_truth_system_descriptors_are_unique_and_truthful() {
    let actions = showcase_actions();
    let ids = actions
        .iter()
        .map(|action| action.id.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(ids.len(), actions.len());
    assert_eq!(actions.len(), 3);
    for action in actions {
        if action.id.as_str() == ACTION_WORKSPACE_SAVE {
            assert!(action.can_invoke());
            assert_eq!(action.label, "Save Workspace");
        } else {
            assert!(!action.can_invoke());
            assert!(action.label.ends_with(" (Experimental)"));
            assert_eq!(action.shortcut, None);
        }
    }
}

#[test]
fn showcase_action_truth_gallery_actions_mutate_dedicated_state() {
    let mut app = ShowcaseApp::new();

    assert!(app.invoke_action(ACTION_COMPONENTS_RUN, ActionSource::Button));
    assert_eq!(app.component_action_count, 1);
    assert_eq!(app.systems_dispatch_count, 0);

    assert!(app.invoke_action(ACTION_SYSTEMS_DISPATCH, ActionSource::Button));
    assert_eq!(app.component_action_count, 1);
    assert_eq!(app.systems_dispatch_count, 1);

    let expected = app.capture_workspace_snapshot();
    assert!(app.invoke_action(ACTION_WORKSPACE_SAVE, ActionSource::Menu));
    assert_eq!(app.workspace_snapshot.as_ref(), Some(&expected));
    assert_eq!(app.status, "Workspace snapshot captured in memory");
    assert_eq!(app.action_count(), 3);
}

#[test]
fn showcase_action_truth_disabled_system_actions_cannot_reach_handler() {
    let mut app = ShowcaseApp::new();

    for action_id in [ACTION_COMMAND_PALETTE, ACTION_VIEWPORT_GRID] {
        assert!(!app.invoke_action(action_id, ActionSource::CommandPalette));
    }

    assert_eq!(app.action_count(), 0);
    assert_eq!(app.workspace_snapshot, None);
}

#[test]
fn showcase_action_truth_router_has_no_unfinished_shortcuts() {
    let modifiers = Modifiers::new(false, true, false, false);
    let keyboard = KeyboardInput {
        modifiers,
        events: ["s", "b", "p"]
            .into_iter()
            .map(|key| {
                KeyEvent::new(
                    Key::Character(key.to_owned()),
                    KeyState::Pressed,
                    modifiers,
                    false,
                )
            })
            .collect(),
    };

    assert!(
        showcase_action_router(true)
            .resolve_shortcuts(&keyboard)
            .is_empty()
    );
}

#[test]
fn showcase_action_truth_play_shortcut_respects_running_state() {
    let mut app = ShowcaseApp::new();
    let play = KeyboardInput {
        modifiers: Modifiers::default(),
        events: vec![KeyEvent::new(
            Key::Function(5),
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    };

    app.resolve_shortcuts(&play);

    assert!(app.editor.is_running());
    assert_eq!(app.action_count(), 1);

    app.resolve_shortcuts(&play);

    assert!(app.editor.is_running());
    assert_eq!(app.action_count(), 1);

    let grid = KeyboardInput {
        modifiers: Modifiers::default(),
        events: vec![KeyEvent::new(
            Key::Character("g".to_owned()),
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    };
    app.resolve_shortcuts(&grid);

    assert_eq!(app.action_count(), 2);
}
