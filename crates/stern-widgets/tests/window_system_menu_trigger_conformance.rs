//! Windowless conformance for the retained window system-menu trigger.

use stern_core::{
    FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PlatformRequest, Point,
    PointerButtonState, PointerInput, PointerOrder, PointerRoute, PointerRoutes, PointerTarget,
    Rect, Response, SemanticRole, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use stern_widgets::{Ui, chrome::WindowSystemMenuTrigger};

const TRIGGER_ID: WidgetId = WidgetId::from_raw(41);
const TITLEBAR: Rect = Rect::new(8.0, 6.0, 28.0, 24.0);
const REQUEST_POSITION: Point = Point::new(-12.5, -4.0);

fn make_trigger(rect: Rect, position: Point) -> WindowSystemMenuTrigger {
    WindowSystemMenuTrigger::new(
        TRIGGER_ID,
        rect,
        stern_icons_phosphor::regular::LIST,
        position,
    )
}

fn pointer_input(down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(16.0, 12.0)),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn run_frame(
    trigger: &WindowSystemMenuTrigger,
    memory: &mut UiMemory,
    input: &UiInput,
    overlapping_drag: bool,
) -> (
    bool,
    PointerRoutes,
    Option<Response>,
    Option<Response>,
    FrameOutput,
) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme);
    let drag_id = ui.make_id("titlebar-drag");
    let mut declared = false;
    let routes = ui
        .resolve_pointer_targets(|plan| {
            if overlapping_drag {
                plan.target(
                    PointerTarget::new(drag_id, TITLEBAR, PointerOrder::new(10))
                        .domain_drag_source(),
                );
            }
            declared = trigger.declare_pointer_target(plan, PointerOrder::new(100));
        })
        .expect("valid pointer plan");
    let drag = overlapping_drag.then(|| ui.draggable("titlebar-drag", TITLEBAR, false));
    let response = ui.window_system_menu_trigger(trigger);
    (declared, routes, drag, response, ui.finish_output())
}

fn system_menu_positions(frame: &FrameOutput) -> Vec<Point> {
    frame
        .platform_requests
        .iter()
        .filter_map(|request| match request {
            PlatformRequest::ShowWindowSystemMenu { position } => Some(*position),
            _ => None,
        })
        .collect()
}

fn assert_focused_semantics(frame: &FrameOutput) {
    let node = frame
        .semantics
        .get(TRIGGER_ID)
        .expect("system-menu trigger semantics");
    assert_eq!(node.role, SemanticRole::IconButton);
    assert_eq!(node.label.as_deref(), Some("Open window system menu"));
    assert!(node.state.focused);
    assert!(node.focusable);
}

#[test]
fn pointer_and_focused_keyboard_activation_delegate_exactly_once() {
    let trigger = make_trigger(TITLEBAR, REQUEST_POSITION);
    let mut pointer_memory = UiMemory::new();
    let (declared, _, _, pressed, pressed_frame) = run_frame(
        &trigger,
        &mut pointer_memory,
        &pointer_input(true, true, false),
        false,
    );
    assert!(declared);
    assert!(pressed.expect("pointer press").state.pressed);
    assert!(system_menu_positions(&pressed_frame).is_empty());

    let (declared, _, _, clicked, clicked_frame) = run_frame(
        &trigger,
        &mut pointer_memory,
        &pointer_input(false, false, true),
        false,
    );
    assert!(declared);
    assert!(clicked.expect("pointer release").clicked);
    assert_eq!(system_menu_positions(&clicked_frame), [REQUEST_POSITION]);
    assert!(clicked_frame.actions.is_empty());
    assert_focused_semantics(&clicked_frame);

    for key in [Key::Enter, Key::Space] {
        let mut memory = UiMemory::new();
        memory.focus(TRIGGER_ID);
        let (declared, _, _, response, frame) =
            run_frame(&trigger, &mut memory, &key_input(key), false);
        assert!(declared);
        let response = response.expect("keyboard response");
        assert_eq!(response.id, TRIGGER_ID);
        assert!(response.keyboard_activated);
        assert_eq!(system_menu_positions(&frame), [REQUEST_POSITION]);
        assert!(frame.actions.is_empty());
        assert_focused_semantics(&frame);
    }
}

#[test]
fn trigger_wins_titlebar_overlap_and_invalid_inputs_fail_closed() {
    let trigger = make_trigger(TITLEBAR, REQUEST_POSITION);
    let mut overlap_memory = UiMemory::new();
    let (declared, routes, drag, response, _) = run_frame(
        &trigger,
        &mut overlap_memory,
        &pointer_input(true, true, false),
        true,
    );
    assert!(declared);
    assert_eq!(routes.ordinary, PointerRoute::Target(TRIGGER_ID));
    assert!(!drag.expect("lower titlebar drag").state.hovered);
    assert!(response.expect("valid trigger").state.pressed);

    let invalid = [
        make_trigger(Rect::ZERO, REQUEST_POSITION),
        make_trigger(Rect::new(f32::NAN, 6.0, 28.0, 24.0), REQUEST_POSITION),
        make_trigger(Rect::new(8.0, f32::INFINITY, 28.0, 24.0), REQUEST_POSITION),
        make_trigger(Rect::new(8.0, 6.0, f32::NAN, 24.0), REQUEST_POSITION),
        make_trigger(Rect::new(8.0, 6.0, 28.0, f32::INFINITY), REQUEST_POSITION),
        make_trigger(Rect::new(f32::MAX, 6.0, f32::MAX, 24.0), REQUEST_POSITION),
        make_trigger(Rect::new(8.0, f32::MAX, 28.0, f32::MAX), REQUEST_POSITION),
        make_trigger(TITLEBAR, Point::new(f32::NAN, -4.0)),
        make_trigger(TITLEBAR, Point::new(-12.5, f32::INFINITY)),
    ];
    for invalid in invalid {
        let mut memory = UiMemory::new();
        memory.focus(TRIGGER_ID);
        let (declared, routes, _, response, frame) =
            run_frame(&invalid, &mut memory, &key_input(Key::Enter), false);
        assert!(!declared);
        assert_ne!(routes.ordinary, PointerRoute::Target(TRIGGER_ID));
        assert!(response.is_none());
        assert!(frame.primitives.is_empty());
        assert!(frame.semantics.get(TRIGGER_ID).is_none());
        assert!(system_menu_positions(&frame).is_empty());
        assert!(frame.actions.is_empty());
    }
}
