#![allow(clippy::float_cmp)]
use core::time::Duration;
use std::time::Instant;

use crate::{
    WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
    WinitShellRequest, WinitTextInputRequest, WinitWindowOps, cursor_to_winit,
    frame_context_from_winit, key_from_winit, modifiers_from_winit, physical_key_from_winit,
    scale_factor_from_winit, viewport_from_winit,
};
use stern_core::{
    ClipboardText, CursorShape, FrameOutput, InputWheelDelta, Key, KeyState, Modifiers,
    MouseButton as CoreMouseButton, PhysicalKey, PlatformRequest, Point, Rect, RepaintRequest,
    ScaleFactor, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticTreeError,
    SemanticValue, TextInputEvent, TextRange, TimeInfo, UiInput, UiInputEvent, Vec2, WidgetId,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta};
use winit::keyboard::{
    Key as WinitKey, KeyCode, ModifiersState, NamedKey, PhysicalKey as WinitPhysicalKey,
};
use winit::window::CursorIcon;

#[derive(Debug, PartialEq)]
enum WindowCall {
    Cursor(CursorIcon),
    Title(String),
    ImeAllowed(bool),
    ImeRect(Rect),
}

#[derive(Debug, Default, PartialEq)]
struct FakeWindow {
    calls: Vec<WindowCall>,
    cursor: Option<CursorIcon>,
    title: Option<String>,
    ime_allowed: Option<bool>,
    ime_rect: Option<Rect>,
}

impl WinitWindowOps for FakeWindow {
    fn set_cursor(&mut self, cursor: CursorIcon) {
        self.calls.push(WindowCall::Cursor(cursor));
        self.cursor = Some(cursor);
    }

    fn set_title(&mut self, title: &str) {
        self.calls.push(WindowCall::Title(title.to_owned()));
        self.title = Some(title.to_owned());
    }

    fn set_ime_allowed(&mut self, allowed: bool) {
        self.calls.push(WindowCall::ImeAllowed(allowed));
        self.ime_allowed = Some(allowed);
    }

    fn set_ime_cursor_area(&mut self, rect: Rect) {
        self.calls.push(WindowCall::ImeRect(rect));
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
    assert!(!adapter.ime_enabled());
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

fn pointer_button_counts(adapter: &WinitInputAdapter) -> Vec<u8> {
    adapter
        .input()
        .events
        .iter()
        .filter_map(|event| match event {
            UiInputEvent::PointerButton { click_count, .. } => Some(*click_count),
            _ => None,
        })
        .collect()
}

#[test]
fn automatic_click_sequence_counts_press_and_matching_release_at_inclusive_boundaries() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    let started = Instant::now();
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    adapter.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(10),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1, 1]);

    adapter.begin_frame();
    adapter.pointer_moved(PhysicalPosition::new(14.0, 10.0));
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(500),
    );
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(510),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![2, 2]);
}

#[test]
fn automatic_click_sequence_resets_beyond_time_or_distance_boundaries() {
    let started = Instant::now();
    let mut late = WinitInputAdapter::new(ScaleFactor::ONE);
    late.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    late.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    late.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(10),
    );
    late.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(501),
    );
    assert_eq!(pointer_button_counts(&late), vec![1, 1, 1]);

    let mut far = WinitInputAdapter::new(ScaleFactor::ONE);
    far.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    far.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    far.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(10),
    );
    far.pointer_moved(PhysicalPosition::new(14.01, 10.0));
    far.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(100),
    );
    assert_eq!(pointer_button_counts(&far), vec![1, 1, 1]);
}

#[test]
fn automatic_click_sequence_defines_mismatch_missing_and_backward_transitions() {
    let started = Instant::now();
    let mut unmatched = WinitInputAdapter::new(ScaleFactor::ONE);
    unmatched.mouse_button_at(WinitMouseButton::Left, ElementState::Released, started);
    assert_eq!(pointer_button_counts(&unmatched), vec![0]);

    let mut missing = WinitInputAdapter::new(ScaleFactor::ONE);
    missing.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    missing.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(1),
    );
    missing.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    missing.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(2),
    );
    assert_eq!(pointer_button_counts(&missing), vec![1, 1, 1]);

    let mut backward = WinitInputAdapter::new(ScaleFactor::ONE);
    backward.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    backward.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(10),
    );
    backward.mouse_button_at(WinitMouseButton::Left, ElementState::Released, started);
    backward.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(20),
    );
    assert_eq!(pointer_button_counts(&backward), vec![1, 1, 1]);

    let mut overlapping = WinitInputAdapter::new(ScaleFactor::ONE);
    overlapping.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    overlapping.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    overlapping.mouse_button_at(
        WinitMouseButton::Right,
        ElementState::Pressed,
        started + Duration::from_millis(1),
    );
    overlapping.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(2),
    );
    assert_eq!(pointer_button_counts(&overlapping), vec![1, 1, 0]);

    let mut different = WinitInputAdapter::new(ScaleFactor::ONE);
    different.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    different.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    different.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(1),
    );
    different.mouse_button_at(
        WinitMouseButton::Right,
        ElementState::Pressed,
        started + Duration::from_millis(2),
    );
    different.mouse_button_at(
        WinitMouseButton::Right,
        ElementState::Released,
        started + Duration::from_millis(3),
    );
    assert_eq!(pointer_button_counts(&different), vec![1, 1, 1, 1]);

    let mut duplicate = WinitInputAdapter::new(ScaleFactor::ONE);
    duplicate.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    duplicate.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    duplicate.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(1),
    );
    duplicate.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(2),
    );
    assert_eq!(pointer_button_counts(&duplicate), vec![1, 1, 0]);

    let mut backward_press = WinitInputAdapter::new(ScaleFactor::ONE);
    backward_press.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    backward_press.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(10),
    );
    backward_press.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(11),
    );
    backward_press.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(9),
    );
    assert_eq!(pointer_button_counts(&backward_press), vec![1, 1, 1]);
}

#[test]
fn automatic_click_sequence_resets_for_explicit_input_leave_focus_and_scale_changes() {
    let started = Instant::now();
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    adapter.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(1),
    );
    adapter.set_scale_factor(ScaleFactor::new(f64::NAN));
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(2),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![2]);

    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(3),
    );
    adapter.set_scale_factor(ScaleFactor::new(2.0));
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(4),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1]);

    adapter.mouse_button(WinitMouseButton::Left, ElementState::Released, 9);
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(5),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1]);

    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(6),
    );
    adapter.pointer_left();
    adapter.pointer_moved(PhysicalPosition::new(20.0, 20.0));
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(7),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1]);

    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(8),
    );
    adapter.set_window_focused(false);
    adapter.set_window_focused(true);
    adapter.pointer_moved(PhysicalPosition::new(20.0, 20.0));
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(9),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1]);
}

#[test]
fn scale_change_cannot_seed_click_history_from_stale_logical_pointer_evidence() {
    let started = Instant::now();
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    adapter.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, started);
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(1),
    );

    adapter.set_scale_factor(ScaleFactor::new(2.0));
    assert_eq!(adapter.input().pointer.position, None);
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
    assert!(matches!(
        adapter.input().events.last(),
        Some(UiInputEvent::PointerLeft)
    ));
    adapter.begin_frame();
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(2),
    );
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(3),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1, 1]);

    adapter.begin_frame();
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Pressed,
        started + Duration::from_millis(4),
    );
    adapter.mouse_button_at(
        WinitMouseButton::Left,
        ElementState::Released,
        started + Duration::from_millis(5),
    );
    assert_eq!(pointer_button_counts(&adapter), vec![1, 1]);
}

#[test]
fn scale_change_fences_same_frame_old_basis_events_without_releasing_buttons() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.set_window_focused(true);
    adapter.set_modifiers(ModifiersState::SHIFT);
    adapter.pointer_moved(PhysicalPosition::new(12.0, 18.0));
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.set_modifiers(ModifiersState::CONTROL);
    adapter.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        4.0, -6.0,
    )));

    adapter.set_scale_factor(ScaleFactor::new(2.0));

    assert_eq!(
        adapter.input().events,
        vec![
            UiInputEvent::WindowFocusChanged(true),
            UiInputEvent::ModifiersChanged(Modifiers {
                shift: true,
                ..Modifiers::default()
            }),
            UiInputEvent::PointerButton {
                button: CoreMouseButton::Primary,
                down: true,
                click_count: 1,
                position: None,
            },
            UiInputEvent::ModifiersChanged(Modifiers {
                ctrl: true,
                ..Modifiers::default()
            }),
            UiInputEvent::PointerLeft,
        ]
    );
    assert!(adapter.input().pointer.primary.down);
    assert_eq!(adapter.input().pointer.position, None);
    assert_eq!(adapter.input().pointer.delta, Vec2::ZERO);
    assert_eq!(adapter.input().pointer.wheel_delta, Vec2::ZERO);
    assert!(adapter.input().window_focused);
    assert!(
        !adapter
            .input()
            .events
            .iter()
            .any(|event| matches!(event, UiInputEvent::PointerReleaseAll { .. }))
    );
    assert_eq!(adapter.input().validate_event_stream(), Ok(()));
}

#[test]
fn automatic_click_sequence_saturates_at_u8_max() {
    let started = Instant::now();
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.pointer_moved(PhysicalPosition::new(10.0, 10.0));

    for millisecond in 0..260 {
        let at = started + Duration::from_millis(millisecond);
        adapter.mouse_button_at(WinitMouseButton::Left, ElementState::Pressed, at);
        adapter.mouse_button_at(WinitMouseButton::Left, ElementState::Released, at);
    }

    assert_eq!(adapter.input().pointer.click_count, u8::MAX);
}

#[test]
fn physical_pixel_wheels_are_logically_equivalent_at_one_and_two_x_dpi() {
    let mut one_x = WinitInputAdapter::new(ScaleFactor::ONE);
    let mut two_x = WinitInputAdapter::new(ScaleFactor::new(2.0));
    one_x.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        8.0, -4.0,
    )));
    two_x.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        16.0, -8.0,
    )));

    assert_eq!(
        one_x.input().pointer.wheel_delta,
        two_x.input().pointer.wheel_delta
    );
    assert_eq!(one_x.input().pointer.wheel_delta, Vec2::new(8.0, -4.0));
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
    adapter.set_window_focused(true);

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
        TextInputEvent::CompositionEnd
    );
    assert_eq!(
        adapter.input().text_events[3],
        TextInputEvent::Commit("done".to_owned())
    );
    assert_eq!(adapter.input().validate_event_stream(), Ok(()));
}

#[test]
fn ime_availability_is_distinct_from_preedit_composition() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.ime_event(Ime::Enabled);
    adapter.ime_event(Ime::Enabled);
    adapter.ime_event(Ime::Disabled);
    adapter.ime_event(Ime::Disabled);

    assert!(adapter.input().text_events.is_empty());
    assert_eq!(
        adapter.input().events,
        vec![
            UiInputEvent::ImeEnabled(true),
            UiInputEvent::ImeEnabled(true),
            UiInputEvent::ImeEnabled(false),
            UiInputEvent::ImeEnabled(false),
        ]
    );
}

#[test]
fn preedit_drives_one_composition_and_commit_ends_before_inserting() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.ime_event(Ime::Preedit("first".to_owned(), None));
    adapter.ime_event(Ime::Preedit("second".to_owned(), Some((1, 3))));
    adapter.ime_event(Ime::Preedit(String::new(), None));
    adapter.ime_event(Ime::Preedit(String::new(), None));
    adapter.ime_event(Ime::Commit("plain".to_owned()));
    adapter.ime_event(Ime::Preedit("committed".to_owned(), None));
    adapter.ime_event(Ime::Commit("committed".to_owned()));

    assert_eq!(
        adapter.input().text_events,
        vec![
            TextInputEvent::CompositionStart,
            TextInputEvent::Composition {
                text: "first".to_owned(),
                selection: None,
            },
            TextInputEvent::Composition {
                text: "second".to_owned(),
                selection: Some(TextRange::new(1, 3)),
            },
            TextInputEvent::CompositionEnd,
            TextInputEvent::Commit("plain".to_owned()),
            TextInputEvent::CompositionStart,
            TextInputEvent::Composition {
                text: "committed".to_owned(),
                selection: None,
            },
            TextInputEvent::CompositionEnd,
            TextInputEvent::Commit("committed".to_owned()),
        ]
    );
}

#[test]
fn hardware_key_text_is_source_aware_and_suppressed_only_during_preedit() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);

    adapter.keyboard_event_with_text(
        &WinitKey::Character("a".into()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
        Some("a"),
    );
    adapter.ime_event(Ime::Enabled);
    adapter.keyboard_event_with_text(
        &WinitKey::Character("b".into()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
        Some("b"),
    );
    adapter.ime_event(Ime::Preedit("preedit".to_owned(), None));
    adapter.keyboard_event_with_text(
        &WinitKey::Character("c".into()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
        Some("c"),
    );
    adapter.ime_event(Ime::Preedit(String::new(), None));
    adapter.keyboard_event_with_text(
        &WinitKey::Character("d".into()),
        ElementState::Pressed,
        ModifiersState::empty(),
        true,
        Some("dead-key-output"),
    );
    adapter.keyboard_event_with_text(
        &WinitKey::Character("e".into()),
        ElementState::Released,
        ModifiersState::empty(),
        false,
        Some("e"),
    );

    let keys = &adapter.input().keyboard.events;
    assert_eq!(keys[0].text.as_deref(), Some("a"));
    assert_eq!(keys[1].text.as_deref(), Some("b"));
    assert_eq!(keys[2].text, None);
    assert_eq!(keys[3].text.as_deref(), Some("dead-key-output"));
    assert!(keys[3].repeat);
    assert_eq!(keys[4].text, None);
}

#[test]
fn focus_loss_orders_composition_end_pointer_cleanup_and_focus_before_later_keys() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::ONE);
    adapter.set_window_focused(true);
    adapter.pointer_moved(PhysicalPosition::new(12.0, 18.0));
    adapter.mouse_button(WinitMouseButton::Left, ElementState::Pressed, 1);
    adapter.ime_event(Ime::Preedit("active".to_owned(), None));

    adapter.set_window_focused(false);
    adapter.keyboard_event_with_text(
        &WinitKey::Character("later".into()),
        ElementState::Pressed,
        ModifiersState::empty(),
        false,
        Some("later"),
    );

    let events = &adapter.input().events;
    let loss = events
        .iter()
        .position(|event| matches!(event, UiInputEvent::WindowFocusChanged(false)))
        .expect("focus loss");
    assert!(matches!(
        events[loss - 3],
        UiInputEvent::Text(TextInputEvent::CompositionEnd)
    ));
    assert!(matches!(
        events[loss - 2],
        UiInputEvent::PointerReleaseAll {
            position: Some(Point { x: 12.0, y: 18.0 })
        }
    ));
    assert_eq!(events[loss - 1], UiInputEvent::PointerLeft);
    assert!(matches!(events[loss + 1], UiInputEvent::Key(_)));
    assert_eq!(adapter.input().validate_event_stream(), Ok(()));
}

#[test]
fn line_and_pixel_wheel_events_keep_event_time_positions() {
    let mut adapter = WinitInputAdapter::new(ScaleFactor::new(2.0));
    adapter.set_window_focused(true);
    adapter.pointer_moved(PhysicalPosition::new(10.0, 20.0));
    adapter.mouse_wheel(MouseScrollDelta::LineDelta(1.0, -2.0));
    adapter.pointer_moved(PhysicalPosition::new(30.0, 40.0));
    adapter.mouse_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        8.0, -12.0,
    )));

    let wheels = adapter
        .input()
        .events
        .iter()
        .filter_map(|event| match event {
            UiInputEvent::Wheel { delta, position } => Some((*delta, *position)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        wheels,
        vec![
            (
                InputWheelDelta::Lines(Vec2::new(1.0, -2.0)),
                Some(Point::new(5.0, 10.0)),
            ),
            (
                InputWheelDelta::Pixels(Vec2::new(4.0, -6.0)),
                Some(Point::new(15.0, 20.0)),
            ),
        ]
    );
    assert_eq!(adapter.input().validate_event_stream(), Ok(()));
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
    let mut output = FrameOutput::new();
    output.push_platform_request(PlatformRequest::SetCursor(CursorShape::Text));
    output.request_repaint(RepaintRequest::NextFrame);
    let requests = WinitPlatformRequests::from_frame_output(&output);

    assert_eq!(
        cursor_to_winit(requests.cursor()),
        winit::window::CursorIcon::Text
    );
    assert_eq!(
        cursor_to_winit(CursorShape::PointingHand),
        winit::window::CursorIcon::Pointer
    );
    assert_eq!(requests.repaint(), RepaintRequest::NextFrame);
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
    output.push_platform_request(PlatformRequest::SetWindowTitle("Stern".to_owned()));
    output.push_platform_request(PlatformRequest::OpenUrl("https://example.com".to_owned()));

    let requests = WinitPlatformRequests::from_frame_output(&output);

    assert_eq!(requests.cursor(), CursorShape::Text);
    assert_eq!(
        requests.repaint(),
        RepaintRequest::After(core::time::Duration::from_millis(20))
    );
    assert_eq!(
        requests.text_input(),
        &[WinitTextInputRequest::Start {
            rect: Some(text_rect)
        }]
    );
    assert_eq!(requests.window_title(), Some("Stern"));
    assert_eq!(
        requests.shell().operations(),
        &[
            WinitShellRequest::CopyToClipboard("copy".to_owned()),
            WinitShellRequest::RequestClipboardText {
                target: text_target,
            },
            WinitShellRequest::OpenUrl("https://example.com".to_owned()),
        ]
    );
}

#[test]
fn replacing_frame_output_discards_all_obsolete_work_and_resets_cursor() {
    let target = WidgetId::from_key("field");
    let mut rich = FrameOutput::new();
    rich.request_repaint(RepaintRequest::Continuous);
    rich.push_platform_request(PlatformRequest::SetCursor(CursorShape::Text));
    rich.push_platform_request(PlatformRequest::CopyToClipboard("copy".to_owned()));
    rich.push_platform_request(PlatformRequest::RequestClipboardText { target });
    rich.push_platform_request(PlatformRequest::StartTextInput { rect: None });
    rich.push_platform_request(PlatformRequest::SetWindowTitle("title".to_owned()));
    rich.push_platform_request(PlatformRequest::OpenUrl("https://example.com".to_owned()));
    let mut requests = WinitPlatformRequests::from_frame_output(&rich);

    requests.replace_frame_output(&FrameOutput::new());

    assert_eq!(requests, WinitPlatformRequests::default());
    let mut window = FakeWindow::default();
    let applied = requests.apply_to_window_ops(&mut window);
    assert_eq!(window.cursor, Some(CursorIcon::Default));
    assert_eq!(window.title, None);
    assert_eq!(window.ime_allowed, None);
    assert_eq!(applied.repaint(), RepaintRequest::None);
    assert!(applied.shell().is_empty());
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
fn text_input_requests_preserve_stop_start_and_update_order() {
    let mut output = FrameOutput::new();
    output.push_platform_request(PlatformRequest::StopTextInput);
    output.push_platform_request(PlatformRequest::StartTextInput { rect: None });
    output.push_platform_request(PlatformRequest::UpdateTextInputRect {
        rect: Rect::new(3.0, 4.0, 1.0, 12.0),
    });

    let requests = WinitPlatformRequests::from_frame_output(&output);

    assert_eq!(
        requests.text_input(),
        &[
            WinitTextInputRequest::Stop,
            WinitTextInputRequest::Start { rect: None },
            WinitTextInputRequest::UpdateRect {
                rect: Rect::new(3.0, 4.0, 1.0, 12.0),
            },
        ]
    );
    let mut window = FakeWindow::default();

    let _ = requests.apply_to_window_ops(&mut window);

    assert_eq!(
        window.calls,
        vec![
            WindowCall::Cursor(CursorIcon::Default),
            WindowCall::ImeAllowed(false),
            WindowCall::ImeAllowed(true),
            WindowCall::ImeRect(Rect::new(3.0, 4.0, 1.0, 12.0)),
        ]
    );
}

#[test]
fn platform_requests_apply_window_effects_and_return_shell_work() {
    let text_rect = Rect::new(10.0, 20.0, 100.0, 24.0);
    let text_target = WidgetId::from_key("field");
    let mut output = FrameOutput::new();
    output.request_repaint(RepaintRequest::Continuous);
    output.push_platform_request(PlatformRequest::SetCursor(CursorShape::Text));
    output.push_platform_request(PlatformRequest::CopyToClipboard("copy".to_owned()));
    output.push_platform_request(PlatformRequest::RequestClipboardText {
        target: text_target,
    });
    output.push_platform_request(PlatformRequest::StartTextInput {
        rect: Some(text_rect),
    });
    output.push_platform_request(PlatformRequest::SetWindowTitle("Stern".to_owned()));
    output.push_platform_request(PlatformRequest::OpenUrl("https://example.com".to_owned()));
    let requests = WinitPlatformRequests::from_frame_output(&output);
    let request_debug = format!("{requests:?}");
    assert!(!request_debug.contains("copy"));
    assert!(!request_debug.contains("Stern"));
    assert!(!request_debug.contains("example.com"));
    let mut window = FakeWindow::default();

    let applied = requests.apply_to_window_ops(&mut window);

    assert_eq!(window.cursor, Some(CursorIcon::Text));
    assert_eq!(window.title, Some("Stern".to_owned()));
    assert_eq!(window.ime_allowed, Some(true));
    assert_eq!(window.ime_rect, Some(text_rect));
    assert_eq!(applied.repaint(), RepaintRequest::Continuous);
    assert_eq!(applied.shell().operations().len(), 3);
    assert_eq!(
        window.calls,
        vec![
            WindowCall::Cursor(CursorIcon::Text),
            WindowCall::Title("Stern".to_owned()),
            WindowCall::ImeAllowed(true),
            WindowCall::ImeRect(text_rect),
        ]
    );
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
    let mut output = FrameOutput::new();
    output.push_platform_request(PlatformRequest::StartTextInput {
        rect: Some(Rect::new(2.0, 3.0, 8.0, 12.0)),
    });
    output.push_platform_request(PlatformRequest::UpdateTextInputRect {
        rect: Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN),
    });
    let requests = WinitPlatformRequests::from_frame_output(&output);
    let mut window = FakeWindow::default();

    let _ = requests.apply_to_window_ops(&mut window);

    assert_eq!(window.ime_allowed, Some(true));
    assert_eq!(window.ime_rect, Some(Rect::new(0.0, 0.0, 0.0, 0.0)));
}

#[test]
fn delayed_repaint_is_returned_to_shell_without_immediate_redraw() {
    let mut output = FrameOutput::new();
    output.request_repaint(RepaintRequest::After(core::time::Duration::from_millis(15)));
    let requests = WinitPlatformRequests::from_frame_output(&output);
    let mut window = FakeWindow::default();

    let applied = requests.apply_to_window_ops(&mut window);

    assert_eq!(
        applied.repaint(),
        RepaintRequest::After(core::time::Duration::from_millis(15))
    );
}
