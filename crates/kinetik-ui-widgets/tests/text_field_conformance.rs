//! Windowless conformance tests for text-field widget integration.

use std::time::Duration;

use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionRouter,
    ActionRoutingContext, ComponentState, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalKey, PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive,
    Rect, RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Shortcut,
    Size, TextInputEvent, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
};
use kinetik_ui_text::{TextEditState, TextLayoutStore, TextSelection};
use kinetik_ui_widgets::{
    NumericInputDraft, NumericScrubInputConfig, PathFieldConfig, Ui, VectorComponentLayout,
    VectorScrubInputConfig, classify_numeric_input_draft, multi_line_text_field, numeric_input,
    numeric_scrub_input, path_field, restore_text_draft, text_field, text_field_with_text_layouts,
    vector2_scrub_input, vector3_component_rects, vector3_scrub_input, vector4_scrub_input,
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

fn released_at(x: f32, y: f32) -> UiInput {
    input_at(x, y, false, false, true)
}

fn double_released_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(false, false, true),
            click_count: 2,
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn scrub_drag_at(x: f32, y: f32, delta_x: f32, modifiers: Modifiers) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            delta: Vec2::new(delta_x, 0.0),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        keyboard: KeyboardInput {
            modifiers,
            events: Vec::new(),
        },
        ..UiInput::default()
    }
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

fn assert_f32_slice_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(
            (*actual - *expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }
}

#[path = "text_field_conformance/focus_and_clipboard.rs"]
mod focus_and_clipboard;
#[path = "text_field_conformance/layout_selection_shortcuts.rs"]
mod layout_selection_shortcuts;
#[path = "text_field_conformance/multiline_navigation.rs"]
mod multiline_navigation;
#[path = "text_field_conformance/numeric_and_scrub.rs"]
mod numeric_and_scrub;
#[path = "text_field_conformance/wrappers_and_path.rs"]
mod wrappers_and_path;
