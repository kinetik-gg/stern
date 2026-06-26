//! Windowless conformance tests for text-field widget integration.

use std::time::Duration;

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionRouter,
    ActionRoutingContext, ComponentState, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalKey, PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive,
    Rect, RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Shortcut,
    Size, TextInputEvent, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
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

fn shift() -> Modifiers {
    Modifiers::new(true, false, false, false)
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

fn physical_shortcut_event(character: &str, physical_key: PhysicalKey) -> KeyEvent {
    KeyEvent::with_physical_key(
        Key::Character(character.to_owned()),
        physical_key,
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

fn has_selection_highlight(
    output: &kinetik_ui_core::FrameOutput,
    theme: &kinetik_ui_core::Theme,
) -> bool {
    let selection = theme
        .text_field(ComponentState {
            hovered: false,
            pressed: false,
            focused: true,
            disabled: false,
            selected: false,
        })
        .selection;

    output.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.fill.as_ref() == Some(&selection)
                    && rect.rect.width > 1.0
                    && rect.rect.height > 1.0
        )
    })
}

#[derive(Clone, Copy)]
enum TextWrapperCase {
    TextField,
    MultiLineTextField,
    SearchField,
    NumericInput,
}

impl TextWrapperCase {
    const fn name(self) -> &'static str {
        match self {
            Self::TextField => "TextField",
            Self::MultiLineTextField => "MultiLineTextField",
            Self::SearchField => "SearchField",
            Self::NumericInput => "NumericInput",
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::TextField => "text",
            Self::MultiLineTextField => "multi",
            Self::SearchField => "search",
            Self::NumericInput => "number",
        }
    }

    fn role(self) -> SemanticRole {
        match self {
            Self::TextField | Self::MultiLineTextField | Self::NumericInput => {
                SemanticRole::TextField
            }
            Self::SearchField => SemanticRole::SearchField,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::SearchField => "Search",
            Self::TextField | Self::MultiLineTextField | Self::NumericInput => "Text field",
        }
    }

    const fn text(self) -> &'static str {
        match self {
            Self::TextField => "clip",
            Self::MultiLineTextField => "one\ntwo",
            Self::SearchField => "media",
            Self::NumericInput => "42.5",
        }
    }

    fn rect(self) -> Rect {
        match self {
            Self::MultiLineTextField => Rect::new(0.0, 0.0, 180.0, 80.0),
            Self::TextField | Self::SearchField | Self::NumericInput => {
                Rect::new(0.0, 0.0, 180.0, 24.0)
            }
        }
    }
}

fn render_text_wrapper(case: TextWrapperCase, disabled: bool) -> kinetik_ui_core::FrameOutput {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let id = root_child(case.key());
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new(case.text());
    if disabled {
        memory.focus(id);
        memory.set_text_input_owner(id);
    }

    let mut ui = Ui::new(&input, &mut memory, &theme);
    match case {
        TextWrapperCase::TextField => {
            ui.text_field(case.key(), case.rect(), &mut state, disabled);
        }
        TextWrapperCase::MultiLineTextField => {
            ui.multi_line_text_field(case.key(), case.rect(), &mut state, disabled);
        }
        TextWrapperCase::SearchField => {
            let output = ui.search_field(case.key(), case.rect(), &mut state, disabled);
            assert_eq!(output.query, case.text(), "{}", case.name());
            assert!(!output.empty, "{}", case.name());
        }
        TextWrapperCase::NumericInput => {
            let output = ui.numeric_input(case.key(), case.rect(), &mut state, disabled);
            assert!(output.valid, "{}", case.name());
            assert_eq!(output.value, Some(42.5), "{}", case.name());
        }
    }
    ui.finish_output()
}

fn has_semantic_action(node: &kinetik_ui_core::SemanticNode, kind: &SemanticActionKind) -> bool {
    node.actions.iter().any(|action| action.kind == *kind)
}

#[test]
fn stage1_text_wrapper_matrix_exposes_semantic_contracts() {
    for case in [
        TextWrapperCase::TextField,
        TextWrapperCase::MultiLineTextField,
        TextWrapperCase::SearchField,
        TextWrapperCase::NumericInput,
    ] {
        let output = render_text_wrapper(case, false);
        let id = root_child(case.key());
        let node = output
            .semantics
            .get(id)
            .unwrap_or_else(|| panic!("{} semantic node", case.name()));

        assert_eq!(node.role, case.role(), "{}", case.name());
        assert_eq!(node.label.as_deref(), Some(case.label()), "{}", case.name());
        assert_eq!(node.bounds, case.rect(), "{}", case.name());
        assert!(node.focusable, "{}", case.name());
        assert!(!node.state.disabled, "{}", case.name());
        assert!(!node.state.focused, "{}", case.name());
        assert!(
            has_semantic_action(node, &SemanticActionKind::Focus),
            "{}",
            case.name()
        );
        assert!(
            has_semantic_action(node, &SemanticActionKind::SetText),
            "{}",
            case.name()
        );
        assert_eq!(
            node.state.value,
            Some(SemanticValue::Text(case.text().to_owned())),
            "{}",
            case.name()
        );
        assert_eq!(node.state.checked, None, "{}", case.name());
        assert!(!node.state.selected, "{}", case.name());

        let disabled_output = render_text_wrapper(case, true);
        let disabled_node = disabled_output
            .semantics
            .get(id)
            .unwrap_or_else(|| panic!("{} disabled semantic node", case.name()));
        assert_eq!(disabled_node.role, case.role(), "{}", case.name());
        assert_eq!(
            disabled_node.label.as_deref(),
            Some(case.label()),
            "{}",
            case.name()
        );
        assert!(disabled_node.state.disabled, "{}", case.name());
        assert!(!disabled_node.state.focused, "{}", case.name());
        assert!(!disabled_node.focusable, "{}", case.name());
        assert!(
            !has_semantic_action(disabled_node, &SemanticActionKind::Focus),
            "{}",
            case.name()
        );
        assert!(
            has_semantic_action(disabled_node, &SemanticActionKind::SetText),
            "{}",
            case.name()
        );
        assert_eq!(
            disabled_node.state.value,
            Some(SemanticValue::Text(case.text().to_owned())),
            "{}",
            case.name()
        );
    }
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
fn ui_text_field_rerendering_current_owner_does_not_restart_platform_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), Some(field));
    assert_eq!(memory.text_input_owner(), Some(field));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));
}

#[test]
fn ui_text_field_clicking_current_owner_does_not_restart_platform_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = pressed_at(8.0, 8.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), Some(field));
    assert_eq!(memory.text_input_owner(), Some(field));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));
}

#[test]
fn ui_text_field_dead_space_click_blurs_and_stops_input() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abc");

    let input = pressed_at(240.0, 120.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", Rect::new(0.0, 0.0, 160.0, 24.0), &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.focused(), None);
    assert_eq!(memory.text_input_owner(), None);
    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
}

#[test]
fn ui_disabled_text_field_cannot_acquire_focus_or_text_ownership() {
    let theme = default_dark_theme();
    let active = root_child("active");
    let disabled = root_child("disabled");
    let mut memory = UiMemory::new();
    memory.focus(active);
    memory.set_text_input_owner(active);
    let mut active_state = TextEditState::new("active");
    let mut disabled_state = TextEditState::new("disabled");

    let input = pressed_at(8.0, 40.0);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field(
        "active",
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut active_state,
        false,
    );
    let disabled_output = ui.text_field(
        "disabled",
        Rect::new(0.0, 32.0, 160.0, 24.0),
        &mut disabled_state,
        true,
    );
    let output = ui.finish_output();

    assert_eq!(
        disabled_output.widget.response.expect("response").id,
        disabled
    );
    assert_eq!(memory.focused(), Some(active));
    assert_eq!(memory.text_input_owner(), Some(active));
    assert!(!output.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput {
            rect: Some(rect),
        } if *rect == Rect::new(0.0, 32.0, 160.0, 24.0)
    )));
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
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
fn focused_text_field_handles_physical_clipboard_shortcuts_with_mismatched_logical_keys() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);

    let mut copy_state = TextEditState::new("abcd");
    copy_state.set_selection(TextSelection::new(1, 3));
    let copy_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("j", PhysicalKey::KeyC)],
        },
        ..UiInput::default()
    };

    let copy = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut copy_state,
        &copy_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!copy.changed);
    assert_eq!(copy_state.text, "abcd");
    assert!(copy.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));

    let mut cut_state = TextEditState::new("abcd");
    cut_state.set_selection(TextSelection::new(1, 3));
    let cut_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("q", PhysicalKey::KeyX)],
        },
        ..UiInput::default()
    };

    let cut = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut cut_state,
        &cut_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(cut.changed);
    assert_eq!(cut_state.text, "ad");
    assert!(cut.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
    }));
    assert!(cut_state.undo());
    assert_eq!(cut_state.text, "abcd");

    let mut paste_state = TextEditState::new("abcd");
    paste_state.set_caret(2);
    let paste_input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![physical_shortcut_event("m", PhysicalKey::KeyV)],
        },
        ..UiInput::default()
    };

    let paste = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut paste_state,
        &paste_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!paste.changed);
    assert_eq!(paste_state.text, "abcd");
    assert!(paste.widget.platform_requests.iter().any(|request| {
        matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
    }));
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_clipboard_shortcuts_and_targeted_text() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: ctrl(),
            events: vec![
                physical_shortcut_event("j", PhysicalKey::KeyC),
                physical_shortcut_event("q", PhysicalKey::KeyX),
                physical_shortcut_event("m", PhysicalKey::KeyV),
            ],
        },
        clipboard_text: vec![kinetik_ui_core::ClipboardText::new(id, "XY")],
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
    unfocused_state.set_selection(TextSelection::new(1, 3));
    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.text, "abcd");
    assert!(
        !unfocused
            .widget
            .platform_requests
            .iter()
            .any(is_clipboard_platform_request)
    );

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
    disabled_state.set_selection(TextSelection::new(1, 3));
    let disabled = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.text, "abcd");
    assert!(
        !disabled
            .widget
            .platform_requests
            .iter()
            .any(is_clipboard_platform_request)
    );
}

fn is_clipboard_platform_request(request: &PlatformRequest) -> bool {
    matches!(
        request,
        PlatformRequest::CopyToClipboard(_) | PlatformRequest::RequestClipboardText { .. }
    )
}

#[test]
fn focused_text_field_handles_backspace_delete_and_replacement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);

    let mut backspace_state = TextEditState::new("aéz");
    backspace_state.set_caret("aé".len());
    let backspace_input = UiInput {
        keyboard: key_input(Key::Backspace, Modifiers::default()),
        ..UiInput::default()
    };
    let backspace = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut backspace_state,
        &backspace_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(backspace.changed);
    assert_eq!(backspace_state.text, "az");
    assert_eq!(backspace_state.caret(), 1);

    let mut delete_state = TextEditState::new("aéz");
    delete_state.set_caret(1);
    let delete_input = UiInput {
        keyboard: key_input(Key::Delete, Modifiers::default()),
        ..UiInput::default()
    };
    let delete = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut delete_state,
        &delete_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(delete.changed);
    assert_eq!(delete_state.text, "az");
    assert_eq!(delete_state.caret(), 1);

    let mut replace_state = TextEditState::new("abcd");
    replace_state.set_selection(TextSelection::new(3, 1));
    let replace_input = UiInput {
        text_events: vec![TextInputEvent::Commit("XY".to_owned())],
        ..UiInput::default()
    };
    let replace = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut replace_state,
        &replace_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(replace.changed);
    assert_eq!(replace_state.text, "aXYd");
    assert_eq!(replace_state.caret(), 3);
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_deletion_and_replacement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        text_events: vec![TextInputEvent::Commit("XY".to_owned())],
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![
                KeyEvent::new(
                    Key::Backspace,
                    KeyState::Pressed,
                    Modifiers::default(),
                    false,
                ),
                KeyEvent::new(Key::Delete, KeyState::Pressed, Modifiers::default(), false),
            ],
        },
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
    unfocused_state.set_selection(TextSelection::new(1, 3));
    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.text, "abcd");
    assert_eq!(unfocused_state.selection, TextSelection::new(1, 3));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
    disabled_state.set_selection(TextSelection::new(1, 3));
    let disabled = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.text, "abcd");
    assert_eq!(disabled_state.selection, TextSelection::new(1, 3));
}

#[test]
fn targeted_clipboard_text_replaces_existing_selection_with_undo() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let other = WidgetId::from_key("other");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(3, 1));
    let input = UiInput {
        clipboard_text: vec![
            kinetik_ui_core::ClipboardText::new(other, "wrong"),
            kinetik_ui_core::ClipboardText::new(id, "é"),
        ],
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(output.changed);
    assert_eq!(state.text, "aéd");
    assert_eq!(state.caret(), "aé".len());
    assert!(state.undo());
    assert_eq!(state.text, "abcd");
    assert_eq!(state.selection, TextSelection::new(3, 1));
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
fn multi_line_text_field_moves_vertically_between_explicit_lines() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-nav");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(6);

    let up_input = UiInput {
        keyboard: key_input(Key::ArrowUp, Modifiers::default()),
        ..UiInput::default()
    };
    let up = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &up_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!up.changed);
    assert_eq!(state.text, "one\ntwo\nthree");
    assert_eq!(state.caret(), 2);

    let down_input = UiInput {
        keyboard: key_input(Key::ArrowDown, Modifiers::default()),
        ..UiInput::default()
    };
    let down = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &down_input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!down.changed);
    assert_eq!(state.caret(), 6);
}

#[test]
fn multi_line_text_field_extends_selection_with_shift_vertical_navigation() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-shift-nav");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(6);
    let input = UiInput {
        keyboard: key_input(Key::ArrowDown, shift()),
        ..UiInput::default()
    };

    let output = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "one\ntwo\nthree");
    assert_eq!(state.selection, TextSelection::new(6, 10));
}

#[test]
fn multi_line_text_field_home_end_are_line_local() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-home-end");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(5);

    let home_input = UiInput {
        keyboard: key_input(Key::Home, Modifiers::default()),
        ..UiInput::default()
    };
    let home = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &home_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!home.changed);
    assert_eq!(state.caret(), 4);

    state.set_caret(5);
    let end_input = UiInput {
        keyboard: key_input(Key::End, Modifiers::default()),
        ..UiInput::default()
    };
    let end = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &end_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!end.changed);
    assert_eq!(state.caret(), 7);
}

#[test]
fn multi_line_text_field_shift_home_end_extend_to_current_line_edges() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-shift-home-end");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo\nthree");
    state.set_caret(5);

    let shift_home_input = UiInput {
        keyboard: key_input(Key::Home, shift()),
        ..UiInput::default()
    };
    let shift_home = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &shift_home_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!shift_home.changed);
    assert_eq!(state.selection, TextSelection::new(5, 4));

    state.set_caret(5);
    let shift_end_input = UiInput {
        keyboard: key_input(Key::End, shift()),
        ..UiInput::default()
    };
    let shift_end = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &shift_end_input,
        &mut memory,
        &theme,
        false,
    );
    assert!(!shift_end.changed);
    assert_eq!(state.selection, TextSelection::new(5, 7));
}

#[test]
fn unfocused_and_disabled_multi_line_text_fields_ignore_navigation() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi-disabled-nav");
    let input = UiInput {
        keyboard: key_input(Key::ArrowDown, shift()),
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("one\ntwo");
    unfocused_state.set_caret(1);
    let unfocused = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.selection, TextSelection::new(1, 1));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("one\ntwo");
    disabled_state.set_caret(1);
    let disabled = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.selection, TextSelection::new(1, 1));
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
fn focused_text_field_extends_selection_with_shift_movement_without_changing_text() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("abcd");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let output = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "abcd");
    assert_eq!(state.selection, TextSelection::new(4, 3));
}

#[test]
fn unfocused_and_disabled_text_fields_ignore_shift_movement() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("field");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut unfocused_memory = UiMemory::new();
    let mut unfocused_state = TextEditState::new("abcd");
    let unfocused = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut unfocused_state,
        &input,
        &mut unfocused_memory,
        &theme,
        false,
    );
    assert!(!unfocused.changed);
    assert_eq!(unfocused_state.selection, TextSelection::new(4, 4));

    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(id);
    disabled_memory.set_text_input_owner(id);
    let mut disabled_state = TextEditState::new("abcd");
    let disabled = text_field(
        id,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut disabled_state,
        &input,
        &mut disabled_memory,
        &theme,
        true,
    );
    assert!(!disabled.changed);
    assert_eq!(disabled_state.selection, TextSelection::new(4, 4));
}

#[test]
fn ui_text_field_selection_only_movement_requests_repaint_and_highlight() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);
    let mut state = TextEditState::new("abcdef");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui.text_field("field", Rect::new(0.0, 0.0, 180.0, 24.0), &mut state, false);
    let frame = ui.finish_output();

    assert!(!output.changed);
    assert_eq!(state.text, "abcdef");
    assert_eq!(state.selection, TextSelection::new(6, 5));
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);
    assert!(has_selection_highlight(&frame, &theme));
}

#[test]
fn search_and_numeric_fields_extend_selection_through_widget_flow() {
    let theme = default_dark_theme();
    let search = root_child("search");
    let number = root_child("number");
    let input = UiInput {
        keyboard: key_input(Key::ArrowLeft, shift()),
        ..UiInput::default()
    };

    let mut search_memory = UiMemory::new();
    search_memory.focus(search);
    search_memory.set_text_input_owner(search);
    let mut search_state = TextEditState::new("media");
    let mut ui = Ui::new(&input, &mut search_memory, &theme);
    let search_output = ui.search_field(
        "search",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut search_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!search_output.field.changed);
    assert_eq!(search_state.selection, TextSelection::new(5, 4));

    let mut numeric_memory = UiMemory::new();
    numeric_memory.focus(number);
    numeric_memory.set_text_input_owner(number);
    let mut numeric_state = TextEditState::new("42.5");
    let mut ui = Ui::new(&input, &mut numeric_memory, &theme);
    let numeric_output = ui.numeric_input(
        "number",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut numeric_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!numeric_output.field.changed);
    assert_eq!(numeric_state.selection, TextSelection::new(4, 3));
}

#[test]
fn single_line_search_and_numeric_navigation_remains_buffer_local() {
    let theme = default_dark_theme();
    let field = WidgetId::from_key("single-nav");
    let mut field_memory = UiMemory::new();
    field_memory.focus(field);
    field_memory.set_text_input_owner(field);
    let mut field_state = TextEditState::new("one\ntwo");
    field_state.set_caret(5);

    let home = text_field(
        field,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut field_state,
        &UiInput {
            keyboard: key_input(Key::Home, Modifiers::default()),
            ..UiInput::default()
        },
        &mut field_memory,
        &theme,
        false,
    );
    assert!(!home.changed);
    assert_eq!(field_state.caret(), 0);

    field_state.set_caret(5);
    let arrow_down = text_field(
        field,
        Rect::new(0.0, 0.0, 160.0, 24.0),
        &mut field_state,
        &UiInput {
            keyboard: key_input(Key::ArrowDown, Modifiers::default()),
            ..UiInput::default()
        },
        &mut field_memory,
        &theme,
        false,
    );
    assert!(!arrow_down.changed);
    assert_eq!(field_state.caret(), 5);

    let search = root_child("search-nav");
    let mut search_memory = UiMemory::new();
    search_memory.focus(search);
    search_memory.set_text_input_owner(search);
    let mut search_state = TextEditState::new("media");
    search_state.set_caret(2);
    let search_input = UiInput {
        keyboard: key_input(Key::End, Modifiers::default()),
        ..UiInput::default()
    };
    let mut ui = Ui::new(&search_input, &mut search_memory, &theme);
    let search_output = ui.search_field(
        "search-nav",
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut search_state,
        false,
    );
    let _ = ui.finish_output();
    assert!(!search_output.field.changed);
    assert_eq!(search_state.caret(), 5);

    let number = WidgetId::from_key("number-nav");
    let mut number_memory = UiMemory::new();
    number_memory.focus(number);
    number_memory.set_text_input_owner(number);
    let mut number_state = TextEditState::new("42.5");
    number_state.set_caret(2);
    let number_output = numeric_input(
        number,
        Rect::new(0.0, 0.0, 180.0, 24.0),
        &mut number_state,
        &UiInput {
            keyboard: key_input(Key::ArrowUp, Modifiers::default()),
            ..UiInput::default()
        },
        &mut number_memory,
        &theme,
        false,
    );
    assert!(!number_output.field.changed);
    assert_eq!(number_state.caret(), 2);
}

#[test]
fn multi_line_text_field_shift_end_extends_to_current_line_end() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("multi");
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory.set_text_input_owner(id);
    let mut state = TextEditState::new("one\ntwo");
    state.set_caret(4);
    let input = UiInput {
        keyboard: key_input(Key::End, shift()),
        ..UiInput::default()
    };

    let output = multi_line_text_field(
        id,
        Rect::new(0.0, 0.0, 180.0, 80.0),
        &mut state,
        &input,
        &mut memory,
        &theme,
        false,
    );

    assert!(!output.changed);
    assert_eq!(state.text, "one\ntwo");
    assert_eq!(state.selection, TextSelection::new(4, 7));
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
