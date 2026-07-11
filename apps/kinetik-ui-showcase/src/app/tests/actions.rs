use std::{collections::HashSet, time::Instant};

use super::helpers::{
    ACTION_COMMAND_PALETTE, ACTION_COMPONENTS_RUN, ACTION_EDITOR_DOCK_JOIN,
    ACTION_SYSTEMS_DISPATCH, ACTION_VIEWPORT_GRID, ACTION_WORKSPACE_SAVE, ActionContext, ActionId,
    ActionInvocation, ActionSource, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PlatformRequest, Point, Rect, ShowcaseApp, ShowcaseInput, ShowcasePage, Size, click,
    frame_context, showcase_action_router, showcase_actions,
};
use crate::editor::{ACTION_DOCS, DOCUMENTATION_URL};
use kinetik_ui::{
    core::UiInputEvent,
    platform_winit::{
        WinitInputAdapter, WinitPlatformRequests, WinitRepaintSchedule, WinitRepaintScheduler,
        WinitShellServiceError, WinitShellServices, WinitWindowOps,
    },
};
use winit::window::CursorIcon;

#[derive(Default)]
struct FakeWindow {
    cursors: Vec<CursorIcon>,
    titles: Vec<String>,
    ime_allowed: Vec<bool>,
    ime_rects: Vec<Rect>,
}

impl WinitWindowOps for FakeWindow {
    fn set_cursor(&mut self, cursor: CursorIcon) {
        self.cursors.push(cursor);
    }

    fn set_title(&mut self, title: &str) {
        self.titles.push(title.to_owned());
    }

    fn set_ime_allowed(&mut self, allowed: bool) {
        self.ime_allowed.push(allowed);
    }

    fn set_ime_cursor_area(&mut self, rect: Rect) {
        self.ime_rects.push(rect);
    }
}

#[derive(Default)]
struct FakeShell {
    clipboard_writes: Vec<String>,
    clipboard_read: Option<String>,
    clipboard_reads: usize,
    opened_urls: Vec<String>,
}

impl WinitShellServices for FakeShell {
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), WinitShellServiceError> {
        self.clipboard_writes.push(text.to_owned());
        Ok(())
    }

    fn read_clipboard_text(&mut self) -> Result<String, WinitShellServiceError> {
        self.clipboard_reads += 1;
        self.clipboard_read
            .take()
            .ok_or(WinitShellServiceError::Unavailable)
    }

    fn open_http_url(&mut self, url: &str) -> Result<(), WinitShellServiceError> {
        self.opened_urls.push(url.to_owned());
        Ok(())
    }
}

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

#[test]
fn documentation_action_sources_emit_one_identical_fixed_https_request() {
    for source in [
        ActionSource::Menu,
        ActionSource::Button,
        ActionSource::Shortcut,
    ] {
        let mut app = ShowcaseApp::new();

        assert!(app.invoke_action(ACTION_DOCS, source));
        app.update(&ShowcaseInput::default());

        assert_eq!(
            app.output().platform_requests,
            vec![PlatformRequest::OpenUrl(DOCUMENTATION_URL.to_owned())]
        );
        assert!(DOCUMENTATION_URL.starts_with("https://"));
    }
}

#[test]
fn documentation_f1_shortcut_routes_through_the_same_application_action() {
    let mut app = ShowcaseApp::new();
    let keyboard = KeyboardInput {
        modifiers: Modifiers::default(),
        events: vec![KeyEvent::new(
            Key::Function(1),
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    };

    app.resolve_shortcuts(&keyboard);
    app.update(&ShowcaseInput::default());

    assert_eq!(
        app.output().platform_requests,
        vec![PlatformRequest::OpenUrl(DOCUMENTATION_URL.to_owned())]
    );
}

#[test]
fn about_documentation_click_reaches_fake_winit_url_cursor_and_repaint_backends_once() {
    let mut app = ShowcaseApp::new();
    assert!(app.invoke_action("editor.about.open", ActionSource::Button));
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let documentation = app.editor.about_modal_documentation_rect(viewport).center();

    click(&mut app, documentation);

    assert_eq!(
        app.output().platform_requests,
        vec![PlatformRequest::OpenUrl(DOCUMENTATION_URL.to_owned())]
    );
    let mut window = FakeWindow::default();
    let applied =
        WinitPlatformRequests::from_frame_output(app.output()).apply_to_window_ops(&mut window);
    let (shell, repaint) = applied.into_parts();
    let mut services = FakeShell::default();
    let outcome = shell.execute(&mut services);
    let mut scheduler = WinitRepaintScheduler::new();
    let now = Instant::now();

    assert_eq!(window.cursors, vec![CursorIcon::Default]);
    assert_eq!(services.opened_urls, vec![DOCUMENTATION_URL.to_owned()]);
    assert!(outcome.results().is_empty());
    assert_eq!(
        scheduler.replace_frame_request(repaint, outcome.has_input_response(), now),
        WinitRepaintSchedule::Immediate
    );
    assert!(scheduler.take_redraw_request(now));
    assert!(!scheduler.take_redraw_request(now));
}

#[test]
fn real_showcase_paste_output_reaches_fake_clipboard_ime_and_repaint_backends() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    click(&mut app, Point::new(940.0, 160.0));

    let modifiers = Modifiers::new(false, true, false, false);
    let mut input = super::helpers::UiInput::default();
    input.push_event(UiInputEvent::ModifiersChanged(modifiers));
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Character("v".to_owned()),
        KeyState::Pressed,
        modifiers,
        false,
    )));
    app.update_with_context(frame_context(Size::new(1440.0, 900.0), input));

    assert!(
        app.output()
            .platform_requests
            .iter()
            .any(|request| { matches!(request, PlatformRequest::RequestClipboardText { .. }) })
    );
    assert!(
        app.output()
            .platform_requests
            .iter()
            .any(|request| { matches!(request, PlatformRequest::UpdateTextInputRect { .. }) })
    );

    let mut window = FakeWindow::default();
    let applied =
        WinitPlatformRequests::from_frame_output(app.output()).apply_to_window_ops(&mut window);
    let (shell, repaint) = applied.into_parts();
    let mut services = FakeShell {
        clipboard_read: Some(" from fake clipboard".to_owned()),
        ..FakeShell::default()
    };
    let outcome = shell.execute(&mut services);
    let has_input_response = outcome.has_input_response();
    let now = Instant::now();
    let mut scheduler = WinitRepaintScheduler::new();

    assert_eq!(services.clipboard_reads, 1);
    assert_eq!(services.opened_urls, Vec::<String>::new());
    assert_eq!(window.cursors, vec![CursorIcon::Default]);
    assert_eq!(window.ime_rects.len(), 1);
    assert_eq!(
        scheduler.replace_frame_request(repaint, has_input_response, now),
        WinitRepaintSchedule::Immediate
    );

    let mut adapter = WinitInputAdapter::default();
    adapter.set_window_focused(true);
    adapter.begin_frame();
    assert!(adapter.apply_shell_outcome(outcome).is_empty());
    app.update_with_context(frame_context(
        Size::new(1440.0, 900.0),
        adapter.into_input(),
    ));

    assert!(app.search().ends_with(" from fake clipboard"));
}
