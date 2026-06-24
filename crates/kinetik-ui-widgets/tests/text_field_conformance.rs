//! Windowless conformance tests for text-field widget integration.

use std::time::Duration;

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionRouter,
    ActionRoutingContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect, RepaintRequest,
    ScaleFactor, SemanticRole, SemanticValue, Shortcut, Size, TextInputEvent, TimeInfo, UiInput,
    UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use kinetik_ui_text::{TextEditState, TextLayoutStore, TextSelection};
use kinetik_ui_widgets::{
    Ui, multi_line_text_field, numeric_input, text_field, text_field_with_text_layouts,
};

fn root_child(key: &str) -> WidgetId {
    WidgetId::from_key("root").child(key)
}

fn ctrl() -> Modifiers {
    Modifiers::new(false, true, false, false)
}

fn shortcut(character: &str) -> Shortcut {
    Shortcut::new(ctrl(), Key::Character(character.to_owned()))
}

fn shortcut_event(character: &str) -> KeyEvent {
    KeyEvent::new(
        Key::Character(character.to_owned()),
        KeyState::Pressed,
        ctrl(),
        false,
    )
}

fn key_input(key: Key, modifiers: Modifiers) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
    }
}

fn input_at(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressed_at(x: f32, y: f32) -> UiInput {
    input_at(x, y, true, true, false)
}

fn frame_context_at(now: Duration, input: UiInput) -> kinetik_ui_core::FrameContext {
    kinetik_ui_core::FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::ZERO,
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(now, Duration::from_millis(16), 0),
    )
}

fn action_descriptor(id: &str, shortcut: Shortcut) -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(id, id);
    descriptor.shortcut = Some(shortcut);
    descriptor
}

fn bind_global(router: &mut ActionRouter, id: &str, shortcut: Shortcut) {
    router.bind(ActionBinding::new(
        action_descriptor(id, shortcut),
        ActionContext::Global,
        ActionPriority::Global,
    ));
}

fn text_value(output: &kinetik_ui_core::FrameOutput, role: &SemanticRole) -> Option<String> {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == *role)
        .and_then(|node| match &node.state.value {
            Some(SemanticValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
}

#[test]
fn focused_text_field_receives_text_and_unfocused_field_ignores_it() {
    let theme = default_dark_theme();
    let focused = WidgetId::from_key("focused");
    let unfocused = WidgetId::from_key("unfocused");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit(" typed".to_owned())],
        ..UiInput::default()
    };
    let mut memory = UiMemory::new();
    memory.focus(focused);
    memory.set_text_input_owner(focused);
    let mut focused_state = TextEditState::new("focused");
    focused_state.set_caret(focused_state.text.len());
    let mut unfocused_state = TextEditState::new("unfocused");
    unfocused_state.set_caret(unfocused_state.text.len());

    let focused_output = text_field(
        focused,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut focused_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let unfocused_output = text_field(
        unfocused,
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(focused_output.changed);
    assert_eq!(focused_state.text, "focused typed");
    assert!(!unfocused_output.changed);
    assert_eq!(unfocused_state.text, "unfocused");
}

#[test]
fn ui_text_field_focus_handoff_stops_previous_owner_before_starting_new_owner() {
    let theme = default_dark_theme();
    let first = root_child("first");
    let second = root_child("second");
    let mut memory = UiMemory::new();
    memory.focus(first);
    memory.set_text_input_owner(first);
    let mut first_state = TextEditState::new("one");
    let mut second_state = TextEditState::new("two");

    let input = pressed_at(8.0, 40.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field(
        "first",
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut first_state,
        false,
    );
    ui.text_field(
        "second",
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut second_state,
        false,
    );
    let output = ui.finish_output();

    let stop_index = output
        .platform_requests
        .iter()
        .position(|request| matches!(request, PlatformRequest::StopTextInput))
        .expect("previous text input owner stopped");
    let second_start_index = output
        .platform_requests
        .iter()
        .position(|request| {
            matches!(
                request,
                PlatformRequest::StartTextInput { rect: Some(rect) }
                    if *rect == Rect::new(0.0, 32.0, 160.0, 24.0)
            )
        })
        .expect("new text input owner started");

    assert!(stop_index < second_start_index);
    assert_eq!(memory.focused(), Some(second));
    assert_eq!(memory.text_input_owner(), Some(second));
}

#[test]
fn text_field_clipboard_requests_are_targeted_and_targeted_text_is_applied() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));
    let copy_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![shortcut_event("c")],
        },
        ..UiInput::default()
    };

    let copy = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &copy_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(copy.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));

    let paste_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![shortcut_event("v")],
        },
        ..UiInput::default()
    };
    let paste = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &paste_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(paste.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
    }));

    let clipboard_input = UiInput {
        clipboard_text: vec![
            kinetik_ui_core::ClipboardText::new(other, "wrong"),
            kinetik_ui_core::ClipboardText::new(id, "XY"),
        ],
        ..UiInput::default()
    };
    let applied = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &clipboard_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(applied.changed);
    assert_eq!(state.text, "aXYd");
}

#[test]
fn single_line_drops_committed_newlines_while_multi_line_preserves_them() {
    let theme = default_dark_theme();
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("a\r\nb\nc".to_owned())],
        ..UiInput::default()
    };

    let single = WidgetId::from_key("single");
    let mut single_memory = UiMemory::new();
    single_memory.focus(single);
    single_memory.set_text_input_owner(single);
    let mut single_state = TextEditState::new("");
    let single_output = text_field(
        single,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut single_state,
        &input,
        &mut single_memory,
        &theme,
        false,
    );

    let multi = WidgetId::from_key("multi");
    let mut multi_memory = UiMemory::new();
    multi_memory.focus(multi);
    multi_memory.set_text_input_owner(multi);
    let mut multi_state = TextEditState::new("");
    let multi_output = multi_line_text_field(
        multi,
        Rect::new(0.0, 0.0, 160.0, 80.0),
        &mut multi_state,
        &input,
        &mut multi_memory,
        &theme,
        false,
    );

    assert!(single_output.changed);
    assert_eq!(single_state.text, "abc");
    assert!(multi_output.changed);
    assert_eq!(multi_state.text, "a\r\nb\nc");
    assert_eq!(multi_output.visible_lines, 3);
}

#[test]
fn multi_line_enter_inserts_newline_but_single_line_enter_does_not() {
    let theme = default_dark_theme();
    let input = UiInput {
        keyboard: key_input(Key::Enter, Modifiers::default()),
        ..UiInput::default()
    };

    let single = WidgetId::from_key("single-enter");
    let mut single_memory = UiMemory::new();
    single_memory.focus(single);
    single_memory.set_text_input_owner(single);
    let mut single_state = TextEditState::new("one");
    let single_output = text_field(
        single,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut single_state,
        &input,
        &mut single_memory,
        &theme,
        false,
    );

    let multi = WidgetId::from_key("multi-enter");
    let mut multi_memory = UiMemory::new();
    multi_memory.focus(multi);
    multi_memory.set_text_input_owner(multi);
    let mut multi_state = TextEditState::new("one");
    multi_state.set_caret(multi_state.text.len());
    let multi_output = multi_line_text_field(
        multi,
        Rect::new(0.0, 0.0, 160.0, 80.0),
        &mut multi_state,
        &input,
        &mut multi_memory,
        &theme,
        false,
    );

    assert!(!single_output.changed);
    assert_eq!(single_state.text, "one");
    assert!(multi_output.changed);
    assert_eq!(multi_state.text, "one\n");
}

#[test]
fn numeric_input_distinguishes_valid_invalid_and_empty_states() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();

    let mut valid_state = TextEditState::new("42.5");
    let valid = numeric_input(
        WidgetId::from_key("valid"),
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut valid_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(valid.valid);
    assert_eq!(valid.value, Some(42.5));

    let mut invalid_state = TextEditState::new("42 px");
    let invalid = numeric_input(
        WidgetId::from_key("invalid"),
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut invalid_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(!invalid.valid);
    assert_eq!(invalid.value, None);

    let mut empty_state = TextEditState::new("  ");
    let empty = numeric_input(
        WidgetId::from_key("empty"),
        Rect::new(0.0, 64.0, 160.0, 24.0),
        &mut empty_state,
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    assert!(empty.valid);
    assert_eq!(empty.value, None);
}

#[test]
fn search_field_exposes_search_semantics_and_current_query() {
    let theme = default_dark_theme();
    let id = root_child("search");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("media");

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let search = ui.search_field(
        "search",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut state,
        false,
    );
    let output = ui.finish_output();

    assert_eq!(search.query, "media");
    assert!(!search.empty);
    assert_eq!(
        text_value(&output, &SemanticRole::SearchField).as_deref(),
        Some("media")
    );
    assert!(output.semantics.nodes().iter().any(|node| {
        node.id == id && node.role == SemanticRole::SearchField && node.state.focused
    }));
}

#[test]
fn ui_text_field_uses_layout_store_for_text_and_caret_blink_repaint() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abcdef");
    let mut text_layouts = TextLayoutStore::new();

    let mut ui = Ui::begin_frame_with_text_layouts(
        frame_context_at(Duration::from_millis(0), UiInput::default()),
        &mut memory,
        &theme,
        &mut text_layouts,
    );
    ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 28.0), &mut state, false);
    let output = ui.finish_output();

    assert!(!text_layouts.is_empty());
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "abcdef" && text.layout.is_some())
    }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect.width <= 1.0 && rect.rect.height > 1.0)
    }));
    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_millis(500))
    );
}

#[test]
fn focused_text_field_reserves_typing_and_editing_shortcuts_from_global_actions() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("x".to_owned())],
        ..UiInput::default()
    };

    let mut ui = Ui::new(&input, &mut memory, &theme);
    let text = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 24.0), &mut state, false);
    let _ = ui.finish_output();
    assert!(text.changed);
    assert_eq!(memory.text_input_owner(), Some(field));

    let mut router = ActionRouter::new();
    bind_global(
        &mut router,
        "global.type.x",
        Shortcut::new(Modifiers::default(), Key::Character("x".to_owned())),
    );
    bind_global(&mut router, "global.select.all", shortcut("a"));
    bind_global(&mut router, "global.cut", shortcut("x"));
    bind_global(&mut router, "global.copy", shortcut("c"));
    bind_global(&mut router, "global.paste", shortcut("v"));

    let routing = ActionRoutingContext::new().with_text_input(field);
    assert_eq!(
        router.resolve_shortcut_in_context(
            &key_input(Key::Character("x".to_owned()), Modifiers::default()),
            routing,
        ),
        None
    );
    for character in ["a", "c", "v", "x"] {
        assert_eq!(
            router.resolve_shortcut_in_context(
                &key_input(Key::Character(character.to_owned()), ctrl()),
                routing,
            ),
            None
        );
    }
}

#[test]
fn pointer_press_uses_shaped_layout_store_for_caret_hit_placement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("hit-field");
    let rect = Rect::new(0.0, 0.0, 220.0, 28.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abcdef");
    let mut text_layouts = TextLayoutStore::new();
    let input = pressed_at(rect.max_x() - 2.0, 14.0);

    let output = text_field_with_text_layouts(
        id,
        rect,
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
        Some(&mut text_layouts),
    );

    assert_eq!(state.caret(), state.text.len());
    assert_eq!(memory.focused(), Some(id));
    assert!(!text_layouts.is_empty());
    assert!(
        output
            .widget
            .response
            .as_ref()
            .is_some_and(|response| response.state.focused)
    );
}
