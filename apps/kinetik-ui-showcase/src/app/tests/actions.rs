use super::helpers::{
    ACTION_EDITOR_DOCK_JOIN, ACTION_SYSTEMS_DISPATCH, ActionContext, ActionId, ActionInvocation,
    ActionSource, Point, ShowcaseApp, ShowcasePage, click,
};

#[test]
fn clicking_button_changes_action_state() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(70.0, 154.0));

    assert_eq!(app.action_count(), 1);
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
    assert_eq!(app.status, "editor.dock.join via Button (1)");
    assert!(!app.status.contains("Ignored unhandled action"));
}

#[test]
fn explicit_showcase_demo_action_is_counted() {
    let mut app = ShowcaseApp::new();

    assert!(app.invoke_action(ACTION_SYSTEMS_DISPATCH, ActionSource::Button));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.status, "systems.dispatch via Button (1)");
}
