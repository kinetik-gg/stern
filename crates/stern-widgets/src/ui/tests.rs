use std::time::Duration;

use super::Ui;
use stern_core::{
    ActionContext, ActionDescriptor, ActionSource, Brush, Color, CursorShape, FrameContext,
    FrameOutput, FrameWarning, IconId, ImageId, Insets, Key, KeyEvent, KeyState, KeyboardInput,
    Modifiers, PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive,
    Rect, RepaintRequest, ScaleFactor, SemanticNode, SemanticRole, Size, TextInputEvent,
    TextPrimitive, TextRange, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_text::{TextEditState, TextLayoutKey, TextLayoutStore, TextStyle};

mod core;
mod core_accessibility;
mod core_behavior;
mod core_platform;
mod core_values;
mod layout;
mod text;

fn pressed_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn held_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn released_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn input_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn scrolled_at(x: f32, y: f32, wheel_delta: Vec2) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            wheel_delta,
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn committed_text(text: &str) -> UiInput {
    UiInput {
        text_events: vec![TextInputEvent::Commit(text.to_owned())],
        ..UiInput::default()
    }
}

fn pressed_key(key: Key) -> UiInput {
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

fn frame_context() -> FrameContext {
    frame_context_at(Duration::from_millis(32))
}

fn frame_context_at(now: Duration) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(1280.0, 720.0),
            PhysicalSize::new(2560, 1440),
            ScaleFactor::new(2.0),
        ),
        UiInput::default(),
        TimeInfo::new(now, Duration::from_millis(16), 2),
    )
}

fn text_field_has_caret(output: &FrameOutput) -> bool {
    output.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(rect)
                if (rect.rect.width - 1.0).abs() < f32::EPSILON
                    && rect.rect.height > 8.0
        )
    })
}
