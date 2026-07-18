//! Retained gradient editor public behavior and composition conformance.

use stern_core::{
    Brush, Color, FrameOutput, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point, Primitive,
    Rect, RepaintRequest, UiInput, UiInputEvent, UiMemory, Vec2, WidgetId, default_dark_theme,
};
use stern_widgets::{
    Ui,
    gradient_editor::{
        GradientEditorConfig, GradientEditorIntent, GradientEditorOutput,
        GradientEditorPrepareError, GradientEditorStop, GradientEditorStopId,
        GradientInterpolationSpace,
    },
};

const ID: WidgetId = WidgetId::from_raw(41);
const BOUNDS: Rect = Rect::new(10.0, 20.0, 320.0, 80.0);

fn id(raw: u64) -> GradientEditorStopId {
    GradientEditorStopId::from_raw(raw)
}

fn stops() -> [GradientEditorStop; 3] {
    [
        GradientEditorStop::new(id(1), 0.0, Color::rgba(1.0, 0.0, 0.0, 1.0)),
        GradientEditorStop::new(id(2), 0.5, Color::rgba(0.0, 1.0, 0.0, 1.0)).removable(true),
        GradientEditorStop::new(id(3), 1.0, Color::rgba(0.0, 0.0, 1.0, 1.0)),
    ]
}

fn run(
    config: GradientEditorConfig<'_>,
    input: &UiInput,
    memory: &mut UiMemory,
) -> Result<(GradientEditorOutput, FrameOutput), GradientEditorPrepareError> {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme);
    let widget = ui.prepare_gradient_editor(config)?;
    let output = ui.gradient_editor(&widget);
    Ok((output, ui.finish_output()))
}

fn pointer_button(position: Point, down: bool) -> UiInputEvent {
    UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down,
        click_count: 1,
        position: Some(position),
    }
}

#[test]
fn gradient_editor_composes_opaque_srgb_ramp_and_declared_space() {
    let stops = stops();
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let widget = ui
        .prepare_gradient_editor(GradientEditorConfig::new(
            ID,
            BOUNDS,
            GradientInterpolationSpace::Srgb,
            &stops,
        ))
        .unwrap();

    assert_eq!(widget.widget_id(), ID);
    assert_eq!(widget.bounds(), BOUNDS);
    assert_eq!(widget.space(), GradientInterpolationSpace::Srgb);
    assert_eq!(widget.stops(), stops);
    assert_eq!(widget.selected_stop(), None);
    assert!(!widget.disabled() && !widget.read_only());

    let output = ui.gradient_editor(&widget);
    let frame = ui.finish_output();
    assert_eq!(output.response.rect, BOUNDS);
    assert!(output.intents.is_empty());
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if matches!(rect.fill, Some(Brush::LinearGradient(_)))
    )));
    assert!(
        frame
            .semantics
            .nodes()
            .iter()
            .any(|node| { node.label.as_deref() == Some("sRGB · Reverse") })
    );
}

#[test]
fn gradient_editor_selects_and_moves_stops_with_event_time_geometry() {
    let stops = stops();
    let mut input = UiInput::default();
    let marker = Point::new(170.0, 62.0);
    let moved = Point::new(246.0, 62.0);
    input.push_event(pointer_button(marker, true));
    input.push_event(UiInputEvent::PointerMoved {
        position: moved,
        delta: Vec2::new(76.0, 0.0),
    });
    let (output, frame) = run(
        GradientEditorConfig::new(ID, BOUNDS, GradientInterpolationSpace::Srgb, &stops),
        &input,
        &mut UiMemory::new(),
    )
    .unwrap();

    assert_eq!(output.intents[0], GradientEditorIntent::SelectStop(id(2)));
    assert_eq!(
        output.intents[1],
        GradientEditorIntent::MoveStop {
            id: id(2),
            position: 0.75,
        }
    );
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);
}

#[test]
fn gradient_editor_keyboard_nudges_deletes_and_reverses_with_typed_intents() {
    let stops = stops();
    let config = || {
        GradientEditorConfig::new(ID, BOUNDS, GradientInterpolationSpace::Srgb, &stops)
            .selected_stop(id(2))
            .keyboard_step(0.1)
    };
    let mut memory = UiMemory::new();
    memory.focus(ID);
    let mut keyboard = UiInput::default();
    keyboard.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::ArrowRight,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    keyboard.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Delete,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    let (output, frame) = run(config(), &keyboard, &mut memory).unwrap();
    assert_eq!(
        output.intents,
        vec![
            GradientEditorIntent::MoveStop {
                id: id(2),
                position: 0.6,
            },
            GradientEditorIntent::RemoveStop(id(2)),
        ]
    );
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);

    let reverse = Point::new(260.0, 32.0);
    let mut press = UiInput::default();
    press.push_event(pointer_button(reverse, true));
    let (output, frame) = run(config(), &press, &mut memory).unwrap();
    assert!(output.intents.is_empty());
    assert_eq!(frame.repaint, RepaintRequest::None);
    let mut release = UiInput::default();
    release.push_event(pointer_button(reverse, false));
    let (output, frame) = run(config(), &release, &mut memory).unwrap();
    assert_eq!(output.intents, vec![GradientEditorIntent::Reverse]);
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);
}

#[test]
fn gradient_editor_preserves_stable_equal_position_order() {
    let stops = [
        GradientEditorStop::new(id(1), 0.5, Color::rgba(1.0, 0.0, 0.0, 1.0)),
        GradientEditorStop::new(id(2), 0.5, Color::rgba(0.0, 1.0, 0.0, 1.0)),
        GradientEditorStop::new(id(3), 0.5, Color::rgba(0.0, 0.0, 1.0, 1.0)),
    ];
    let (_, frame) = run(
        GradientEditorConfig::new(ID, BOUNDS, GradientInterpolationSpace::Srgb, &stops),
        &UiInput::default(),
        &mut UiMemory::new(),
    )
    .unwrap();
    let gradient = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) => match rect.fill {
                Some(Brush::LinearGradient(gradient)) => Some(gradient),
                _ => None,
            },
            _ => None,
        });
    let colors = gradient
        .unwrap()
        .stops()
        .iter()
        .map(|stop| stop.color)
        .collect::<Vec<_>>();
    assert_eq!(
        colors,
        stops.iter().map(|stop| stop.color).collect::<Vec<_>>()
    );
}

#[test]
fn gradient_editor_rejects_invalid_and_unsupported_preview_contracts() {
    let valid = stops();
    let error = |space, stops: &[GradientEditorStop], selected| {
        let mut config = GradientEditorConfig::new(ID, BOUNDS, space, stops);
        if let Some(selected) = selected {
            config = config.selected_stop(selected);
        }
        run(config, &UiInput::default(), &mut UiMemory::new()).unwrap_err()
    };
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &valid[..1], None),
        GradientEditorPrepareError::InvalidStopCount
    );
    let duplicate = [
        valid[0],
        GradientEditorStop::new(id(1), 1.0, valid[2].color),
    ];
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &duplicate, None),
        GradientEditorPrepareError::DuplicateStopId
    );
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &valid, Some(id(99))),
        GradientEditorPrepareError::UnknownSelectedStop
    );
    let mut invalid = valid;
    invalid[1].position = f32::NAN;
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &invalid, None),
        GradientEditorPrepareError::InvalidStopPosition
    );
    invalid = valid;
    invalid[1].color.r = f32::NAN;
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &invalid, None),
        GradientEditorPrepareError::InvalidStopColor
    );
    invalid = valid;
    invalid[1].color.a = 0.5;
    assert_eq!(
        error(GradientInterpolationSpace::Srgb, &invalid, None),
        GradientEditorPrepareError::TranslucentPreview
    );
    for space in [
        GradientInterpolationSpace::LinearSrgb,
        GradientInterpolationSpace::DisplayP3,
    ] {
        assert_eq!(
            error(space, &valid, None),
            GradientEditorPrepareError::UnsupportedInterpolationSpace
        );
    }
}

#[test]
fn gradient_editor_disabled_and_read_only_states_do_not_mutate_or_repaint() {
    let stops = stops();
    let original = stops;
    let mut input = UiInput::default();
    input.push_event(pointer_button(Point::new(170.0, 62.0), true));
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(246.0, 62.0),
        delta: Vec2::new(76.0, 0.0),
    });
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Delete,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    for config in [
        GradientEditorConfig::new(ID, BOUNDS, GradientInterpolationSpace::Srgb, &stops)
            .selected_stop(id(2))
            .disabled(true),
        GradientEditorConfig::new(ID, BOUNDS, GradientInterpolationSpace::Srgb, &stops)
            .selected_stop(id(2))
            .read_only(true),
    ] {
        let (output, frame) = run(config, &input, &mut UiMemory::new()).unwrap();
        assert!(output.intents.is_empty());
        assert_eq!(frame.repaint, RepaintRequest::None);
    }
    assert_eq!(stops, original);
}
