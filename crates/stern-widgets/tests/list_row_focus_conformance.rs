//! Exact inward focus and selection-exception conformance for reusable list rows.

#![allow(clippy::float_cmp)]

use stern_core::{
    Brush, Color, CursorShape, PathElement, PlatformRequest, PointerButtonState, PointerInput,
    Primitive, Rect, SemanticRole, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use stern_widgets::{WidgetOutput, list_row};

fn hovering(rect: Rect) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(rect.center()),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressing(rect: Rect) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(rect.center()),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn focused_memory(id: WidgetId) -> UiMemory {
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory
}

fn path_bounds(elements: &[PathElement]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in elements.iter().flat_map(|element| match *element {
        PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
        PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
        PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
        PathElement::Close => Vec::new(),
    }) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn annuli(output: &WidgetOutput) -> [&Primitive; 2] {
    [&output.primitives[1], &output.primitives[2]]
}

fn without_annuli(output: &WidgetOutput) -> Vec<Primitive> {
    output
        .primitives
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 1 | 2))
        .map(|(_, primitive)| primitive.clone())
        .collect()
}

fn linear_channel(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast_ratio(foreground: Color, background: Color) -> f32 {
    let luminance = |color: Color| {
        0.2126 * linear_channel(color.r)
            + 0.7152 * linear_channel(color.g)
            + 0.0722 * linear_channel(color.b)
    };
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn assert_exact_focus_pair(focused: &WidgetOutput, unfocused: &WidgetOutput, rect: Rect) {
    let theme = default_dark_theme();
    assert_eq!(focused.primitives.len(), 4);
    assert_eq!(unfocused.primitives.len(), 2);
    assert_eq!(focused.primitives[0], unfocused.primitives[0]);
    let Primitive::Rect(base) = &focused.primitives[0] else {
        panic!("neutral list-row base must be first");
    };
    assert_eq!(base.rect, rect);
    assert_eq!(base.radius, theme.radii.none);
    assert_eq!(
        base.stroke.expect("neutral row boundary").brush,
        Brush::Solid(theme.colors.border.subtle)
    );
    assert_eq!(
        base.stroke.expect("neutral row boundary").width,
        theme.strokes.hairline
    );
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(rect, base.radius, base.stroke.unwrap().width);
    assert_eq!(focused.primitives[1], expected[0]);
    assert_eq!(focused.primitives[2], expected[1]);
    for primitive in annuli(focused) {
        let Primitive::Path(path) = primitive else {
            panic!("list-row focus must remain a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        assert!(rect.contains_rect(path_bounds(&path.elements)));
    }
    assert!(matches!(focused.primitives[3], Primitive::Text(_)));
    assert_eq!(focused.primitives[3], unfocused.primitives[1]);
    assert_eq!(without_annuli(focused), unfocused.primitives);

    let focused_response = focused.response.as_ref().expect("focused response");
    let unfocused_response = unfocused.response.as_ref().expect("unfocused response");
    assert_eq!(focused_response.id, unfocused_response.id);
    assert_eq!(focused_response.rect, unfocused_response.rect);
    assert_eq!(
        focused_response.state.selected,
        unfocused_response.state.selected
    );
    assert_eq!(
        focused_response.state.disabled,
        unfocused_response.state.disabled
    );
    let focused_semantic = &focused.semantics[0];
    let unfocused_semantic = &unfocused.semantics[0];
    assert_eq!(focused_semantic.role, SemanticRole::ListItem);
    assert_eq!(focused_semantic.id, unfocused_semantic.id);
    assert_eq!(focused_semantic.bounds, unfocused_semantic.bounds);
    assert_eq!(focused_semantic.label, unfocused_semantic.label);
    assert_eq!(
        focused_semantic.state.selected,
        unfocused_semantic.state.selected
    );
}

#[test]
fn reusable_rows_use_exact_inward_focus_without_changing_base_content_or_geometry() {
    let theme = default_dark_theme();
    let rect = Rect::new(10.25, 20.5, 142.0, 24.0);
    let id = WidgetId::from_key("list-row-focus-basic");

    for selected in [false, true] {
        let focused = list_row(
            id,
            rect,
            "Timeline",
            selected,
            &UiInput::default(),
            &mut focused_memory(id),
            &theme,
            false,
        );
        let unfocused = list_row(
            id,
            rect,
            "Timeline",
            selected,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
            false,
        );
        assert_exact_focus_pair(&focused, &unfocused, rect);
    }
}

#[test]
fn reusable_row_focus_is_state_independent_and_disabled_focus_is_suppressed() {
    let theme = default_dark_theme();
    let rect = Rect::new(3.25, 5.75, 132.0, 24.0);
    let id = WidgetId::from_key("list-row-focus-combined");
    let baseline = list_row(
        id,
        rect,
        "State",
        false,
        &UiInput::default(),
        &mut focused_memory(id),
        &theme,
        false,
    );
    for (selected, input) in [
        (false, hovering(rect)),
        (false, pressing(rect)),
        (true, UiInput::default()),
        (true, hovering(rect)),
        (true, pressing(rect)),
    ] {
        let output = list_row(
            id,
            rect,
            "State",
            selected,
            &input,
            &mut focused_memory(id),
            &theme,
            false,
        );
        assert_eq!(annuli(&output), annuli(&baseline));
        assert_eq!(output.response.as_ref().expect("response").rect, rect);
        assert_eq!(output.semantics[0].bounds, rect);
    }

    let disabled = list_row(
        id,
        rect,
        "Disabled",
        true,
        &UiInput::default(),
        &mut focused_memory(id),
        &theme,
        true,
    );
    assert_eq!(disabled.primitives.len(), 2);
    assert!(disabled.response.as_ref().expect("response").state.focused);
    assert!(!disabled.semantics[0].focusable);
    assert!(!disabled.semantics[0].state.focused);
    assert!(
        disabled
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
}

#[test]
fn reusable_selected_rows_enumerate_the_white_on_blue_contrast_exception() {
    let theme = default_dark_theme();
    assert_eq!(
        theme.colors.selection.background,
        Color::rgb8(0x0C, 0x8C, 0xE9)
    );
    assert_eq!(theme.colors.selection.foreground, Color::WHITE);
    let ratio = contrast_ratio(
        theme.colors.selection.foreground,
        theme.colors.selection.background,
    );
    assert!((ratio - 3.53).abs() < 0.01);
    assert!(
        ratio < 4.5,
        "known exception is not AA normal-text compliance"
    );

    let rect = Rect::new(4.5, 8.25, 120.0, 24.0);
    let id = WidgetId::from_key("list-row-selection-exception");
    for (name, input, focused) in [
        ("selected-only", UiInput::default(), false),
        ("selected-hovered", hovering(rect), false),
        ("selected-pressed", pressing(rect), false),
        ("selected-focused", UiInput::default(), true),
        ("selected-focused-hovered", hovering(rect), true),
    ] {
        let mut memory = if focused {
            focused_memory(id)
        } else {
            UiMemory::new()
        };
        let output = list_row(id, rect, name, true, &input, &mut memory, &theme, false);
        let Primitive::Rect(base) = &output.primitives[0] else {
            panic!("selected row base");
        };
        assert_eq!(
            base.fill,
            Some(Brush::Solid(theme.colors.selection.background)),
            "{name}"
        );
        assert_eq!(
            base.stroke.expect("neutral row boundary").brush,
            Brush::Solid(theme.colors.border.subtle),
            "{name}"
        );
        let text_index = if output.response.as_ref().expect("response").state.focused {
            3
        } else {
            1
        };
        let Primitive::Text(text) = &output.primitives[text_index] else {
            panic!("selected row text");
        };
        assert_eq!(
            text.brush,
            Brush::Solid(theme.colors.selection.foreground),
            "{name}"
        );
    }
}

#[test]
fn reusable_row_click_preserves_same_frame_selection_cursor_and_label_geometry() {
    let theme = default_dark_theme();
    let rect = Rect::new(2.25, 4.75, 126.0, 24.0);
    let id = WidgetId::from_key("list-row-same-frame");
    let mut memory = UiMemory::new();
    let press = pressing(rect);
    let _ = list_row(id, rect, "Asset", false, &press, &mut memory, &theme, false);
    let release = UiInput {
        pointer: PointerInput {
            position: Some(rect.center()),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let output = list_row(
        id,
        rect,
        "Asset",
        false,
        &release,
        &mut memory,
        &theme,
        false,
    );
    let response = output.response.as_ref().expect("response");
    assert!(response.clicked);
    assert!(response.state.selected);
    assert!(!response.state.focused);
    assert_eq!(response.rect, rect);
    assert!(output.semantics[0].state.selected);
    assert!(!output.semantics[0].state.focused);
    assert_eq!(output.semantics[0].bounds, rect);
    assert_eq!(output.semantics[0].label.as_deref(), Some("Asset"));
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
    let Primitive::Text(text) = &output.primitives[1] else {
        panic!("same-frame selected label");
    };
    assert!(text.origin.x.is_finite() && text.origin.y.is_finite());
    assert!(text.size.is_finite() && text.line_height.is_finite());
}
