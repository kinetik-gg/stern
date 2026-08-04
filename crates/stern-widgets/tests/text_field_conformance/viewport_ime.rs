use std::time::Duration;

use stern_core::{
    ClipId, ComponentState, InputWheelDelta, Modifiers, MouseButton, PhysicalSize, PlatformRequest,
    Point, PointerOrder, PointerTarget, Primitive, Rect, RepaintRequest, ScaleFactor, Size,
    TextInputOwnerMode, TextRange, TimeInfo, Transform, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo,
};
use stern_text::{TextComposition, TextEditState, TextLayoutKey, TextLayoutStore, TextStyle};
use stern_widgets::{TextFieldAccess, Ui};

use super::{default_dark_theme, root_child};

const FIELD: Rect = Rect::new(0.0, 0.0, 64.0, 24.0);

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

fn canonical(events: impl IntoIterator<Item = UiInputEvent>) -> UiInput {
    let mut input = UiInput::default();
    for event in events {
        input.push_event(event);
    }
    input
}

fn wheel(delta: InputWheelDelta, position: Point) -> UiInput {
    canonical([UiInputEvent::Wheel {
        delta,
        position: Some(position),
    }])
}

fn start_rect(requests: &[PlatformRequest]) -> Option<Rect> {
    requests.iter().find_map(|request| match request {
        PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
        _ => None,
    })
}

fn update_rect(requests: &[PlatformRequest]) -> Option<Rect> {
    requests.iter().find_map(|request| match request {
        PlatformRequest::UpdateTextInputRect { rect } => Some(*rect),
        _ => None,
    })
}

fn text_transform(output: &stern_core::FrameOutput) -> Transform {
    output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::TransformBegin(transform) => Some(*transform),
            _ => None,
        })
        .expect("text viewport transform")
}

#[test]
fn single_and_multiline_wheel_offsets_are_axis_limited_staged_and_promoted() {
    let theme = default_dark_theme();

    let mut single_memory = UiMemory::new();
    let mut single = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-30.0, -70.0)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut single_memory, &theme);
    ui.text_field("field", FIELD, &mut single, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, 0.0);
    assert_close(text_transform(&output).dy, 0.0);
    assert_eq!(
        single_memory.scroll_offset(root_child("field")),
        Vec2::new(30.0, 0.0)
    );
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut single_memory, &theme);
    ui.text_field("field", FIELD, &mut single, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, -30.0);
    assert_close(text_transform(&output).dy, 0.0);

    let mut multi_memory = UiMemory::new();
    let mut multi = TextEditState::new("one two three four five six seven eight nine ten");
    let rect = Rect::new(0.0, 0.0, 64.0, 36.0);
    let input = wheel(
        InputWheelDelta::Lines(Vec2::new(-2.0, -1.0)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut multi_memory, &theme);
    ui.multi_line_text_field("field", rect, &mut multi, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, 0.0);
    assert_close(text_transform(&output).dy, 0.0);
    assert_close(multi_memory.scroll_offset(root_child("field")).x, 0.0);
    assert_close(multi_memory.scroll_offset(root_child("field")).y, 40.0);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut multi_memory, &theme);
    ui.multi_line_text_field("field", rect, &mut multi, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, 0.0);
    assert_close(text_transform(&output).dy, -40.0);
}

#[test]
fn canonical_wheel_survives_a_conflicted_legacy_projection() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let mut input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-30.0, 0.0)),
        Point::new(8.0, 8.0),
    );
    input.pointer.wheel_delta = Vec2::new(999.0, 0.0);
    assert!(input.validate_event_stream().is_err());

    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.scroll_offset(field), Vec2::new(30.0, 0.0));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
#[allow(clippy::too_many_lines)]
fn wheel_routing_honors_plans_hits_fences_finite_input_and_disabled_access() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let other = root_child("other");

    let mut planned_memory = UiMemory::new();
    let mut planned_state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut planned_memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        plan.target(PointerTarget::wheel_only(
            field,
            FIELD,
            PointerOrder::new(1),
        ));
    })
    .expect("valid wheel plan");
    ui.text_field("field", FIELD, &mut planned_state, false);
    let output = ui.finish_output();
    assert_eq!(planned_memory.scroll_offset(field), Vec2::new(10.0, 0.0));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    for blocked_by in [other, field] {
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
        let input = wheel(
            InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
            Point::new(8.0, 8.0),
        );
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.resolve_pointer_targets(|plan| {
            if blocked_by == other {
                plan.target(PointerTarget::wheel_only(
                    other,
                    FIELD,
                    PointerOrder::new(1),
                ));
            } else {
                plan.target(PointerTarget::wheel_only(
                    field,
                    FIELD,
                    PointerOrder::new(1),
                ));
                plan.blocker(FIELD, PointerOrder::new(2));
            }
        })
        .expect("valid blocked wheel plan");
        ui.text_field("field", FIELD, &mut state, false);
        let _ = ui.finish_output();
        assert_eq!(memory.scroll_offset(field), Vec2::ZERO);
    }

    let mut outside_memory = UiMemory::new();
    let mut outside_state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
        Point::new(200.0, 200.0),
    );
    let mut ui = Ui::new(&input, &mut outside_memory, &theme);
    let field_output = ui.text_field("field", FIELD, &mut outside_state, false);
    let _ = ui.finish_output();
    assert!(
        !field_output
            .widget
            .response
            .expect("field response")
            .state
            .hovered
    );
    assert_eq!(outside_memory.scroll_offset(field), Vec2::ZERO);

    for (events, expected) in [
        (
            vec![
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
                UiInputEvent::WindowFocusChanged(false),
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-20.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
            ],
            10.0,
        ),
        (
            vec![
                UiInputEvent::WindowFocusChanged(false),
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-20.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
            ],
            0.0,
        ),
        (
            vec![
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
                UiInputEvent::PointerReleaseAll {
                    position: Some(Point::new(8.0, 8.0)),
                },
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-20.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
            ],
            10.0,
        ),
        (
            vec![
                UiInputEvent::PointerReleaseAll {
                    position: Some(Point::new(8.0, 8.0)),
                },
                UiInputEvent::Wheel {
                    delta: InputWheelDelta::Pixels(Vec2::new(-20.0, 0.0)),
                    position: Some(Point::new(8.0, 8.0)),
                },
            ],
            0.0,
        ),
    ] {
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
        let input = canonical(events);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field("field", FIELD, &mut state, false);
        let _ = ui.finish_output();
        assert_close(memory.scroll_offset(field).x, expected);
    }

    let mut invalid_memory = UiMemory::new();
    let mut invalid_state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(f32::NAN, f32::NEG_INFINITY)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut invalid_memory, &theme);
    ui.text_field("field", FIELD, &mut invalid_state, false);
    let _ = ui.finish_output();
    assert_eq!(invalid_memory.scroll_offset(field), Vec2::ZERO);

    let mut disabled_memory = UiMemory::new();
    let mut disabled_state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-10.0, 0.0)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut disabled_memory, &theme);
    ui.text_field_with_access(
        "field",
        FIELD,
        &mut disabled_state,
        TextFieldAccess::Disabled,
    );
    let _ = ui.finish_output();
    assert_eq!(disabled_memory.scroll_offset(field), Vec2::ZERO);
}

#[test]
fn focused_wheel_candidate_is_kept_when_the_caret_remains_visible() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    state.set_caret(5);
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(-5.0, 0.0)),
        Point::new(8.0, 8.0),
    );

    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let output = ui.finish_output();

    assert_eq!(memory.scroll_offset(field), Vec2::new(5.0, 0.0));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn hidden_initial_caret_stages_reveal_then_starts_ime_on_following_frame() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let hidden = ui.finish_output();
    assert_eq!(start_rect(&hidden.platform_requests), None);
    assert!(memory.scroll_offset(field).x > 0.0);
    assert_eq!(hidden.repaint, RepaintRequest::NextFrame);

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let revealed = ui.finish_output();
    let caret = start_rect(&revealed.platform_requests).expect("revealed caret starts IME");
    assert!(caret.width > 0.0 && caret.height > 0.0);
    assert!(FIELD.contains_rect(caret));
}

#[test]
fn active_hidden_caret_retains_native_rect_until_revealed() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    state.set_caret(0);

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let first = ui.finish_output();
    assert!(start_rect(&first.platform_requests).is_some());

    state.set_caret(state.text.len());
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let hidden = ui.finish_output();
    assert_eq!(update_rect(&hidden.platform_requests), None);
    assert!(!hidden.platform_requests.iter().any(|request| matches!(
        request,
        PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
    )));

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let revealed = ui.finish_output();
    assert!(update_rect(&revealed.platform_requests).is_some());
}

#[test]
fn ime_lifecycle_distinguishes_editable_read_only_and_disabled() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abc");
    state.set_caret(0);

    let mut frame = |access| {
        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field_with_access("field", FIELD, &mut state, access);
        ui.finish_output()
    };

    let editable = frame(TextFieldAccess::Editable);
    assert!(start_rect(&editable.platform_requests).is_some());
    let read_only = frame(TextFieldAccess::ReadOnly);
    assert_eq!(
        read_only
            .platform_requests
            .iter()
            .filter(|request| matches!(request, PlatformRequest::StopTextInput))
            .count(),
        1
    );
    let read_only_again = frame(TextFieldAccess::ReadOnly);
    assert!(
        !read_only_again
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );
    let editable_again = frame(TextFieldAccess::Editable);
    assert!(start_rect(&editable_again.platform_requests).is_some());
    let disabled = frame(TextFieldAccess::Disabled);
    assert_eq!(
        disabled
            .platform_requests
            .iter()
            .filter(|request| matches!(request, PlatformRequest::StopTextInput))
            .count(),
        1
    );

    let mut read_only_memory = UiMemory::new();
    read_only_memory.focus(field);
    read_only_memory.set_text_input_owner_mode(field, TextInputOwnerMode::ReadOnly);
    let mut state = TextEditState::new("abc");
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut read_only_memory, &theme);
    ui.text_field_with_access("field", FIELD, &mut state, TextFieldAccess::Disabled);
    let output = ui.finish_output();
    assert!(
        !output
            .platform_requests
            .contains(&PlatformRequest::StopTextInput)
    );

    let mut hidden_memory = UiMemory::new();
    hidden_memory.focus(field);
    hidden_memory.set_text_input_owner_mode(field, TextInputOwnerMode::ReadOnly);
    let mut hidden_state = TextEditState::new("abcdefghijklmnopqrstuvwxyz");
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut hidden_memory, &theme);
    ui.text_field_with_access("field", FIELD, &mut hidden_state, TextFieldAccess::Editable);
    let hidden = ui.finish_output();
    assert_eq!(start_rect(&hidden.platform_requests), None);
    assert_eq!(hidden.repaint, RepaintRequest::NextFrame);

    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut hidden_memory, &theme);
    ui.text_field_with_access("field", FIELD, &mut hidden_state, TextFieldAccess::Editable);
    let revealed = ui.finish_output();
    assert!(start_rect(&revealed.platform_requests).is_some());
}

#[test]
fn blink_off_caret_still_publishes_native_ime_geometry() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abc");
    state.set_caret(0);
    let context = stern_core::FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::ZERO,
            ScaleFactor::ONE,
        ),
        UiInput::default(),
        TimeInfo::new(Duration::from_millis(750), Duration::from_millis(16), 0),
    );

    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut state, false);
    let output = ui.finish_output();

    assert!(start_rect(&output.platform_requests).is_some());
    let caret_color = theme
        .text_field(ComponentState {
            hovered: false,
            pressed: false,
            focused: true,
            disabled: false,
            selected: false,
        })
        .caret;
    assert!(!output.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.fill == Some(stern_core::Brush::Solid(caret_color))
                && rect.rect.width <= 1.0
    )));
}

#[test]
fn nested_transform_and_clip_project_only_positive_caret_intersections() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abc");
    state.set_caret(0);
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.primitive(Primitive::ClipBegin {
        id: ClipId::from_raw(900),
        rect: Rect::new(0.0, 0.0, 20.0, 10.0),
    });
    ui.primitive(Primitive::TransformBegin(Transform::translation(
        Vec2::new(10.0, 20.0),
    )));
    ui.text_field("field", FIELD, &mut state, false);
    ui.primitive(Primitive::TransformEnd);
    ui.primitive(Primitive::ClipEnd {
        id: ClipId::from_raw(900),
    });
    let output = ui.finish_output();

    // The clip is established before the transform, so the transformed field
    // is completely outside it and publishes no zero-area caret.
    assert_eq!(start_rect(&output.platform_requests), None);

    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("abc");
    state.set_caret(0);
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.primitive(Primitive::TransformBegin(Transform::translation(
        Vec2::new(10.0, 20.0),
    )));
    ui.primitive(Primitive::ClipBegin {
        id: ClipId::from_raw(901),
        rect: Rect::new(0.0, 0.0, 20.0, 10.0),
    });
    ui.text_field("field", FIELD, &mut state, false);
    ui.primitive(Primitive::ClipEnd {
        id: ClipId::from_raw(901),
    });
    ui.primitive(Primitive::TransformEnd);
    let output = ui.finish_output();
    // Caret local x is `FIELD.x + recipe.padding_x` (8, docs/visual-spec/
    // 02-fields.md "padding-inline 8", issue #911) before the +10 translation.
    assert_eq!(
        start_rect(&output.platform_requests),
        Some(Rect::new(18.0, 24.0, 1.0, 6.0))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn shaped_geometry_reuses_one_layout_for_hit_selection_paint_caret_and_ime() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    let mut state = TextEditState::new("shaped text");
    state.set_caret(0);
    let mut store = TextLayoutStore::new();
    let recipe = theme.text_field(ComponentState {
        hovered: false,
        pressed: true,
        focused: true,
        disabled: false,
        selected: false,
    });
    let expected = store.layout_id(TextLayoutKey::new(
        state.text.clone(),
        TextStyle::new(
            recipe.font.family,
            recipe.font.size,
            recipe.font.line_height,
        ),
        FIELD.width - recipe.padding_x * 2.0,
        false,
    ));
    let target_offset = 3;
    let registered_layout = store.layout(expected).expect("registered shaped layout");
    let layout_caret = registered_layout.caret_rect(target_offset);
    let content_height = FIELD.height - recipe.padding_y * 2.0;
    let vertical_origin = super::single_line_baseline_offset(registered_layout, content_height);
    let click = Point::new(
        FIELD.x + recipe.padding_x + layout_caret.x + 0.25,
        FIELD.y + recipe.padding_y + vertical_origin + layout_caret.y + layout_caret.height * 0.5,
    );
    let input = canonical([
        UiInputEvent::ModifiersChanged(Modifiers::new(true, false, false, false)),
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(click),
        },
    ]);
    let context = stern_core::FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::ZERO,
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    );

    let mut ui = Ui::begin_frame_with_text_layouts(context, &mut memory, &theme, &mut store);
    ui.text_field("field", FIELD, &mut state, false);
    let output = ui.finish_output();
    let emitted = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => text.layout,
            _ => None,
        })
        .expect("shaped text primitive");
    assert_eq!(emitted, expected);
    assert_eq!(
        state.selection,
        stern_text::TextSelection::new(0, target_offset)
    );

    let expected_selection = store
        .layout(expected)
        .expect("retained shaped layout")
        .selection_rects(0..target_offset)
        .into_iter()
        .map(|rect| {
            rect.translate(Vec2::new(
                FIELD.x + recipe.padding_x,
                FIELD.y + recipe.padding_y + vertical_origin,
            ))
        })
        .collect::<Vec<_>>();
    let painted_selection = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill == Some(recipe.selection) => Some(rect.rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(painted_selection, expected_selection);

    let expected_caret = store
        .layout(expected)
        .expect("retained shaped layout")
        .caret_rect(target_offset)
        .translate(Vec2::new(
            FIELD.x + recipe.padding_x,
            FIELD.y + recipe.padding_y + vertical_origin,
        ));
    let content_rect = Rect::new(
        FIELD.x + recipe.padding_x,
        FIELD.y + recipe.padding_y,
        FIELD.width - recipe.padding_x * 2.0,
        FIELD.height - recipe.padding_y * 2.0,
    );
    assert_eq!(
        start_rect(&output.platform_requests),
        content_rect.intersection(expected_caret)
    );
}

#[test]
fn preedit_hits_map_to_model_and_all_selection_end_forms_are_clamped() {
    let theme = default_dark_theme();
    let click = |x| {
        canonical([UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(Point::new(x, 8.0)),
        }])
    };
    for (x, expected) in [(4.0, 0), (18.0, 1), (60.0, 2)] {
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("ab");
        state.set_caret(1);
        state.composition = Some(TextComposition::new("XY", None));
        let input = click(x);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field("field", FIELD, &mut state, false);
        let _ = ui.finish_output();
        assert_eq!(state.caret(), expected);
    }

    let caret_x = |selection| {
        let mut memory = UiMemory::new();
        memory.focus(root_child("field"));
        let mut state = TextEditState::new("ab");
        state.set_caret(1);
        state.composition = Some(TextComposition {
            text: "éx".to_owned(),
            selection,
        });
        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.text_field("field", FIELD, &mut state, false);
        let output = ui.finish_output();
        start_rect(&output.platform_requests)
            .expect("visible clamped preedit caret")
            .x
    };
    let absent = caret_x(None);
    let oversized_end = caret_x(Some(TextRange {
        start: 0,
        end: usize::MAX,
    }));
    let reversed = caret_x(Some(TextRange { start: 3, end: 0 }));
    let inside_utf8 = caret_x(Some(TextRange { start: 0, end: 1 }));
    let after_scalar = caret_x(Some(TextRange { start: 0, end: 2 }));

    assert_close(absent, oversized_end);
    assert_close(reversed, inside_utf8);
    assert!(reversed < after_scalar);
    assert!(after_scalar < absent);
}

#[test]
fn content_shrink_repairs_retained_offset_and_caret_reveal_overrides_wheel() {
    let theme = default_dark_theme();
    let field = root_child("field");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(field, Vec2::new(f32::NAN, f32::INFINITY));
    let mut short = TextEditState::new("x");
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut short, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, 0.0);
    assert_eq!(memory.scroll_offset(field), Vec2::ZERO);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    memory.set_scroll_offset(field, Vec2::new(500.0, 90.0));
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.text_field("field", FIELD, &mut short, false);
    let output = ui.finish_output();
    assert_close(text_transform(&output).dx, 0.0);
    assert_eq!(memory.scroll_offset(field), Vec2::ZERO);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let long_text = "abcdefghijklmnopqrstuvwxyz";
    let mut baseline_memory = UiMemory::new();
    baseline_memory.focus(field);
    let mut baseline = TextEditState::new(long_text);
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut baseline_memory, &theme);
    ui.text_field("field", FIELD, &mut baseline, false);
    let _ = ui.finish_output();

    let mut wheel_memory = UiMemory::new();
    wheel_memory.focus(field);
    let mut with_wheel = TextEditState::new(long_text);
    let input = wheel(
        InputWheelDelta::Pixels(Vec2::new(200.0, 0.0)),
        Point::new(8.0, 8.0),
    );
    let mut ui = Ui::new(&input, &mut wheel_memory, &theme);
    ui.text_field("field", FIELD, &mut with_wheel, false);
    let _ = ui.finish_output();
    assert_eq!(
        wheel_memory.scroll_offset(field),
        baseline_memory.scroll_offset(field)
    );
}
