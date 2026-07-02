#![allow(clippy::float_cmp)]
use crate::{
    WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
    WinitTextInputRequest, WinitWindowOps, cursor_to_winit, frame_context_from_winit,
    key_from_winit, modifiers_from_winit, physical_key_from_winit, scale_factor_from_winit,
    viewport_from_winit,
};
use kinetik_ui_core::{
    ClipboardText, CursorShape, FrameOutput, Key, KeyState, Modifiers,
    MouseButton as CoreMouseButton, PhysicalKey, PlatformRequest, Point, Rect, RepaintRequest,
    ScaleFactor, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticTreeError,
    SemanticValue, TextInputEvent, TextRange, TimeInfo, UiInput, Vec2, WidgetId,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{
    Key as WinitKey, KeyCode, ModifiersState, NamedKey, PhysicalKey as WinitPhysicalKey,
};
use winit::window::CursorIcon;

#[derive(Debug, Default, PartialEq)]
struct FakeWindow {
    redraws: usize,
    cursor: Option<CursorIcon>,
    title: Option<String>,
    ime_allowed: Option<bool>,
    ime_rect: Option<Rect>,
}

impl WinitWindowOps for FakeWindow {
    fn request_redraw(&mut self) {
        self.redraws += 1;
    }

    fn set_cursor(&mut self, cursor: CursorIcon) {
        self.cursor = Some(cursor);
    }

    fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_owned());
    }

    fn set_ime_allowed(&mut self, allowed: bool) {
        self.ime_allowed = Some(allowed);
    }

    fn set_ime_cursor_area(&mut self, rect: Rect) {
        self.ime_rect = Some(rect);
    }
}

#[test]
fn viewport_conversion_uses_logical_units() {
    let viewport = viewport_from_winit(PhysicalSize::new(1920, 1080), 2.0);

    assert_eq!(viewport.logical_size.width, 960.0);
    assert_eq!(viewport.logical_size.height, 540.0);
    assert_eq!(viewport.physical_size.width, 1920);
    assert_eq!(viewport.scale_factor, ScaleFactor::new(2.0));
}

#[test]
fn viewport_conversion_sanitizes_invalid_scale_factor() {
    let viewport = viewport_from_winit(PhysicalSize::new(1920, 1080), f64::NAN);

    assert_eq!(viewport.logical_size.width, 1920.0);
    assert_eq!(viewport.logical_size.height, 1080.0);
    assert_eq!(viewport.scale_factor, ScaleFactor::ONE);
    assert_eq!(scale_factor_from_winit(0.0), ScaleFactor::ONE);
}

#[test]
fn frame_context_from_winit_combines_viewport_input_and_time() {
    let input = UiInput {
        window_focused: true,
        ..UiInput::default()
    };
    let time = TimeInfo::new(
        core::time::Duration::from_millis(32),
        core::time::Duration::from_millis(16),
        2,
    );

    let context = frame_context_from_winit(PhysicalSize::new(1280, 720), 2.0, input, time);

    assert_eq!(context.viewport.logical_size.width, 640.0);
    assert!(context.input.window_focused);
    assert_eq!(context.time.frame_index, 2);
}

#[test]
fn frame_clock_reports_delta_and_clamps_backwards_time() {
    let mut clock = WinitFrameClock::new();

    let first = clock.tick(core::time::Duration::from_millis(20));
    let second = clock.tick(core::time::Duration::from_millis(36));
    let backwards = clock.tick(core::time::Duration::from_millis(30));

    assert_eq!(first.delta, core::time::Duration::ZERO);
    assert_eq!(first.frame_index, 0);
    assert_eq!(second.delta, core::time::Duration::from_millis(16));
    assert_eq!(second.frame_index, 1);
    assert_eq!(backwards.delta, core::time::Duration::ZERO);
    assert_eq!(backwards.frame_index, 2);

    clock.reset();
    assert_eq!(
        clock.tick(core::time::Duration::from_millis(1)).frame_index,
        0
    );
}

#[test]
fn pointer_conversion_tracks_position_delta_button_and_wheel() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::new(2.0));

    adapter.pointer_moved(PhysicalPosition::new(20.0, 10.0));
    adapter.pointer_moved(PhysicalPosition::new(24.0, 16.0));
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));

    let input = adapter.input();
    assert_eq!(input.pointer.position.expect("position").x, 12.0);
    assert_eq!(input.pointer.delta.x, 2.0);
    assert!(input.pointer.primary.down);
    assert!(input.pointer.primary.pressed);
    assert_eq!(input.pointer.wheel_delta.y, -1.0);
}

#[test]
fn pointer_and_wheel_conversion_sanitize_non_finite_platform_values() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::new(f64::NAN));

    adapter.pointer_moved(PhysicalPosition::new(f64::INFINITY, f64::NAN));
    adapter.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        f64::NAN,
        f64::INFINITY,
    )));

    let input = adapter.input();
    assert_eq!(input.pointer.position.expect("position").x, 0.0);
    assert_eq!(input.pointer.position.expect("position").y, 0.0);
    assert_eq!(input.pointer.wheel_delta.x, 0.0);
    assert_eq!(input.pointer.wheel_delta.y, 0.0);
}

#[test]
fn pointer_conversion_uses_one_for_invalid_scale_factor() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::new(f64::NAN));

    adapter.pointer_moved(PhysicalPosition::new(24.0, 16.0));

    assert_eq!(
        adapter.input().pointer.position,
        Some(Point::new(24.0, 16.0))
    );
}

#[test]
fn pointer_leave_clears_hover_and_resets_next_move_delta() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.pointer_moved(PhysicalPosition::new(20.0, 10.0));
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.pointer_left();

    assert_eq!(adapter.input().pointer.position, None);
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
    assert!(adapter.input().pointer.primary.down);

    adapter.pointer_moved(PhysicalPosition::new(25.0, 12.0));

    assert_eq!(
        adapter.input().pointer.position,
        Some(Point::new(25.0, 12.0))
    );
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
}

#[test]
fn begin_frame_clears_transient_input() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.text_input("a");

    adapter.begin_frame();

    assert!(adapter.input().pointer.primary.down);
    assert!(!adapter.input().pointer.primary.pressed);
    assert!(adapter.input().text_events.is_empty());
}

#[test]
fn mouse_button_transitions_preserve_same_frame_edges_and_other_buttons() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Released, 1);
    adapter.mouse_button(WinitMouseButton::Other(8), ElementState::Pressed, 1);

    assert!(!adapter.input().pointer.primary.down);
    assert!(adapter.input().pointer.primary.pressed);
    assert!(adapter.input().pointer.primary.released);
    assert!(
        adapter
            .input()
            .pointer
            .button(CoreMouseButton::Other(8))
            .down
    );
}

#[test]
fn losing_window_focus_releases_pressed_buttons() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.begin_frame();
    adapter.set_window_focused(false);

    assert!(!adapter.input().pointer.primary.down);
    assert!(adapter.input().pointer.primary.released);
    assert_eq!(adapter.input().pointer.position, None);
}

#[test]
fn keyboard_conversion_maps_named_and_character_keys() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.keyboard_event(
        &WinitKey::Named(NamedKey::Enter),
        ElementState::Pressed,
        ModifiersState::CONTROL,
        false,
    );
    adapter.keyboard_event(
        &WinitKey::Character("s".into()),
        ElementState::Pressed,
        ModifiersState::CONTROL,
        true,
    );

    assert_eq!(adapter.input().keyboard.events[0].key, Key::Enter);
    assert_eq!(adapter.input().keyboard.events[0].state, KeyState::Pressed);
    assert_eq!(
        adapter.input().keyboard.events[1].key,
        Key::Character("s".to_owned())
    );
    assert!(adapter.input().keyboard.events[1].repeat);
}

#[test]
fn keyboard_conversion_preserves_physical_key() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.keyboard_event_with_physical_key(
        &WinitKey::Character("z".into()),
        &WinitPhysicalKey::Code(KeyCode::KeyY),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
    );

    let event = &adapter.input().keyboard.events[0];
    assert_eq!(event.key, Key::Character("z".to_owned()));
    assert_eq!(event.physical_key, PhysicalKey::KeyY);
}

#[test]
fn ime_events_preserve_lifecycle_and_selection() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.ime_event(Ime::Enabled);
    adapter.ime_event(Ime::Preedit("compose".to_owned(), Some((1, 4))));
    adapter.ime_event(Ime::Commit("done".to_owned()));
    adapter.ime_event(Ime::Disabled);

    assert_eq!(
        adapter.input().text_events[0],
        TextInputEvent::CompositionStart
    );
    assert_eq!(
        adapter.input().text_events[1],
        TextInputEvent::Composition {
            text: "compose".to_owned(),
            selection: Some(TextRange::new(1, 4)),
        }
    );
    assert_eq!(
        adapter.input().text_events[2],
        TextInputEvent::Commit("done".to_owned())
    );
    assert_eq!(
        adapter.input().text_events[3],
        TextInputEvent::CompositionEnd
    );
}

#[test]
fn modifier_conversion_maps_control() {
    assert_eq!(
        modifiers_from_winit(ModifiersState::CONTROL),
        Modifiers::new(false, true, false, false)
    );
}

#[test]
fn adapter_tracks_modifier_only_changes_without_key_events() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.set_modifiers(ModifiersState::SHIFT | ModifiersState::ALT);

    assert_eq!(
        adapter.input().keyboard.modifiers,
        Modifiers::new(true, false, true, false)
    );
    assert!(adapter.input().keyboard.events.is_empty());
}

#[test]
fn key_conversion_maps_arrows_and_functions() {
    assert_eq!(
        key_from_winit(&WinitKey::Named(NamedKey::ArrowLeft)),
        Key::ArrowLeft
    );
    assert_eq!(
        key_from_winit(&WinitKey::Named(NamedKey::F5)),
        Key::Function(5)
    );
    assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::Home)), Key::Home);
    assert_eq!(
        physical_key_from_winit(&WinitPhysicalKey::Code(KeyCode::Digit7)),
        PhysicalKey::Digit(7)
    );
}

#[test]
fn cursor_and_redraw_requests_are_represented() {
    let mut requests = WinitPlatformRequests {
        cursor: CursorShape::Text,
        repaint: RepaintRequest::After(core::time::Duration::from_secs(5)),
        ..WinitPlatformRequests::default()
    };

    requests.request_repaint(RepaintRequest::NextFrame);

    assert_eq!(
        cursor_to_winit(requests.cursor),
        winit::window::CursorIcon::Text
    );
    assert_eq!(
        cursor_to_winit(CursorShape::PointingHand),
        winit::window::CursorIcon::Pointer
    );
    assert_eq!(requests.repaint, RepaintRequest::NextFrame);
}

#[test]
fn frame_output_platform_requests_translate_to_winit_request_data() {
    let mut output = FrameOutput::new();
    let text_rect = Rect::new(10.0, 20.0, 100.0, 24.0);
    let text_target = WidgetId::from_key("field");
    output.request_repaint(RepaintRequest::After(core::time::Duration::from_millis(20)));
    output.push_platform_request(PlatformRequest::SetCursor(CursorShape::Text));
    output.push_platform_request(PlatformRequest::CopyToClipboard("copy".to_owned()));
    output.push_platform_request(PlatformRequest::RequestClipboardText {
        target: text_target,
    });
    output.push_platform_request(PlatformRequest::StartTextInput {
        rect: Some(text_rect),
    });
    output.push_platform_request(PlatformRequest::SetWindowTitle("Kinetik".to_owned()));
    output.push_platform_request(PlatformRequest::OpenUrl("https://example.com".to_owned()));

    let requests = WinitPlatformRequests::from_frame_output(&output);

    assert_eq!(requests.cursor, CursorShape::Text);
    assert_eq!(
        requests.repaint,
        RepaintRequest::After(core::time::Duration::from_millis(20))
    );
    assert_eq!(requests.clipboard_text, Some("copy".to_owned()));
    assert_eq!(requests.request_clipboard_text, Some(text_target));
    assert_eq!(
        requests.text_input,
        Some(WinitTextInputRequest::Start {
            rect: Some(text_rect)
        })
    );
    assert_eq!(requests.window_title, Some("Kinetik".to_owned()));
    assert_eq!(requests.open_urls, vec!["https://example.com".to_owned()]);
}

#[test]
fn frame_output_accessibility_update_preserves_semantic_data() {
    let mut output = FrameOutput::new();
    let root = WidgetId::from_key("root");
    let button = WidgetId::from_key("button");
    let slider = WidgetId::from_key("slider");
    output.push_semantic_node(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button, slider]),
    );
    output.push_semantic_node(
        SemanticNode::new(
            button,
            SemanticRole::Button,
            Rect::new(0.0, 0.0, 80.0, 28.0),
        )
        .focusable(true)
        .with_label("Run")
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Run")),
    );
    let mut slider_node = SemanticNode::new(
        slider,
        SemanticRole::Slider,
        Rect::new(0.0, 32.0, 120.0, 18.0),
    )
    .focusable(true)
    .with_label("Opacity")
    .with_action(SemanticAction::new(
        SemanticActionKind::Increment,
        "Increase",
    ));
    slider_node.state.value = Some(SemanticValue::Number {
        current: 0.5,
        min: 0.0,
        max: 1.0,
    });
    output.push_semantic_node(slider_node);

    let update =
        WinitAccessibilityUpdate::from_frame_output(&output, Some(button)).expect("update");
    let snapshot = update.snapshot;

    assert_eq!(snapshot.root, Some(root));
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![root, button, slider]
    );
    assert_eq!(snapshot.focus_order, vec![button, slider]);
    assert_eq!(snapshot.focused, Some(button));
    assert_eq!(
        snapshot.node(button).expect("button").label.as_deref(),
        Some("Run")
    );
    assert_eq!(
        snapshot.node(slider).expect("slider").state.value,
        Some(SemanticValue::Number {
            current: 0.5,
            min: 0.0,
            max: 1.0,
        })
    );
    assert!(
        snapshot
            .node(slider)
            .expect("slider")
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Increment)
    );
}

#[test]
fn frame_output_accessibility_update_reports_invalid_semantics_without_os_services() {
    let mut output = FrameOutput::new();
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");
    output.push_semantic_node(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
    );

    assert_eq!(
        WinitAccessibilityUpdate::from_frame_output(&output, None).expect_err("error"),
        SemanticTreeError::UnknownChild {
            parent: root,
            child: missing,
        }
    );
}

#[test]
fn stop_text_input_overrides_start_request() {
    let mut output = FrameOutput::new();
    output.push_platform_request(PlatformRequest::StartTextInput { rect: None });
    output.push_platform_request(PlatformRequest::StopTextInput);

    let requests = WinitPlatformRequests::from_frame_output(&output);

    assert_eq!(requests.text_input, Some(WinitTextInputRequest::Stop));
}

#[test]
fn platform_requests_apply_window_effects_and_return_shell_work() {
    let text_rect = Rect::new(10.0, 20.0, 100.0, 24.0);
    let text_target = WidgetId::from_key("field");
    let requests = WinitPlatformRequests {
        cursor: CursorShape::Text,
        repaint: RepaintRequest::Continuous,
        clipboard_text: Some("copy".to_owned()),
        request_clipboard_text: Some(text_target),
        text_input: Some(WinitTextInputRequest::Start {
            rect: Some(text_rect),
        }),
        window_title: Some("Kinetik".to_owned()),
        open_urls: vec!["https://example.com".to_owned()],
    };
    let mut window = FakeWindow::default();

    let shell = requests.apply_to_window_ops(&mut window);

    assert_eq!(window.redraws, 1);
    assert_eq!(window.cursor, Some(CursorIcon::Text));
    assert_eq!(window.title, Some("Kinetik".to_owned()));
    assert_eq!(window.ime_allowed, Some(true));
    assert_eq!(window.ime_rect, Some(text_rect));
    assert_eq!(shell.clipboard_text, Some("copy".to_owned()));
    assert_eq!(shell.request_clipboard_text, Some(text_target));
    assert_eq!(shell.open_urls, vec!["https://example.com".to_owned()]);
    assert!(shell.continuous_repaint);
}

#[test]
fn adapter_feeds_targeted_clipboard_text_into_input() {
    let target = WidgetId::from_key("field");
    let mut adapter = WinitInputAdapter::default();

    adapter.clipboard_text(target, "pasted");
    assert_eq!(
        adapter.input().clipboard_text,
        &[ClipboardText::new(target, "pasted")]
    );

    adapter.begin_frame();
    assert!(adapter.input().clipboard_text.is_empty());
}

#[test]
fn platform_text_input_rects_are_sanitized_for_window_ops() {
    let requests = WinitPlatformRequests {
        text_input: Some(WinitTextInputRequest::Start {
            rect: Some(Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN)),
        }),
        ..WinitPlatformRequests::default()
    };
    let mut window = FakeWindow::default();

    let _ = requests.apply_to_window_ops(&mut window);

    assert_eq!(window.ime_allowed, Some(true));
    assert_eq!(window.ime_rect, Some(Rect::new(0.0, 0.0, 0.0, 0.0)));
}

#[test]
fn delayed_repaint_is_returned_to_shell_without_immediate_redraw() {
    let requests = WinitPlatformRequests {
        repaint: RepaintRequest::After(core::time::Duration::from_millis(15)),
        ..WinitPlatformRequests::default()
    };
    let mut window = FakeWindow::default();

    let shell = requests.apply_to_window_ops(&mut window);

    assert_eq!(window.redraws, 0);
    assert_eq!(
        shell.repaint_after,
        Some(core::time::Duration::from_millis(15))
    );
}
