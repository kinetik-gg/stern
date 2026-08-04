//! Exact inward focus and neutral-selection conformance for reusable tabs.

use stern_core::{
    Brush, PathElement, PointerButtonState, PointerInput, Primitive, Rect, SemanticRole, UiInput,
    UiMemory, WidgetId, default_dark_theme,
};
use stern_widgets::{WidgetOutput, tab_button};

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

// A selected tab paints an extra top-edge `border.strong` indicator rect
// immediately after the base surface
// (`docs/visual-spec/05-chrome-dock.md` §Frame tab strip), shifting every
// later primitive index by one.
fn indicator_offset(selected: bool) -> usize {
    usize::from(selected)
}

fn annuli(output: &WidgetOutput, selected: bool) -> [&Primitive; 2] {
    let base = 1 + indicator_offset(selected);
    [&output.primitives[base], &output.primitives[base + 1]]
}

fn assert_focus_pair(focused: &WidgetOutput, unfocused: &WidgetOutput, rect: Rect, selected: bool) {
    let theme = default_dark_theme();
    let offset = indicator_offset(selected);
    assert_eq!(focused.primitives.len(), 4 + offset);
    assert_eq!(unfocused.primitives.len(), 2 + offset);
    assert_eq!(focused.primitives[0], unfocused.primitives[0]);
    let Primitive::Rect(base) = &focused.primitives[0] else {
        panic!("neutral tab base must be first");
    };
    assert_eq!(base.rect, rect);
    let expected_radius = stern_core::CornerRadius {
        top_left: theme.radii.sm.top_left,
        top_right: theme.radii.sm.top_right,
        bottom_left: 0.0,
        bottom_right: 0.0,
    };
    assert_eq!(base.radius, expected_radius);
    assert_eq!(
        base.stroke.expect("neutral tab boundary").brush,
        Brush::Solid(stern_core::Color::TRANSPARENT)
    );
    if selected {
        let Primitive::Rect(indicator) = &focused.primitives[1] else {
            panic!("selected tab indicator must be second");
        };
        assert_eq!(
            indicator.fill,
            Some(Brush::Solid(theme.colors.border.strong))
        );
        assert_eq!(focused.primitives[1], unfocused.primitives[1]);
    }
    let annulus_base = 1 + offset;
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(rect, base.radius, base.stroke.unwrap().width);
    assert_eq!(focused.primitives[annulus_base], expected[0]);
    assert_eq!(focused.primitives[annulus_base + 1], expected[1]);
    for primitive in annuli(focused, selected) {
        let Primitive::Path(path) = primitive else {
            panic!("tab focus must remain a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        assert!(rect.contains_rect(path_bounds(&path.elements)));
    }
    let text_index = annulus_base + 2;
    assert!(matches!(focused.primitives[text_index], Primitive::Text(_)));
    assert_eq!(
        focused.primitives[text_index],
        unfocused.primitives[1 + offset]
    );
    let mut stripped = focused.primitives.clone();
    stripped.drain(annulus_base..annulus_base + 2);
    assert_eq!(stripped, unfocused.primitives);

    let focused_response = focused.response.as_ref().expect("focused response");
    let unfocused_response = unfocused.response.as_ref().expect("unfocused response");
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
    assert_eq!(focused_semantic.id, unfocused_semantic.id);
    assert_eq!(focused_semantic.role, SemanticRole::Tab);
    assert_eq!(focused_semantic.role, unfocused_semantic.role);
    assert_eq!(focused_semantic.bounds, unfocused_semantic.bounds);
    assert_eq!(
        focused_semantic.state.selected,
        unfocused_semantic.state.selected
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn reusable_tabs_use_exact_inward_focus_and_neutral_top_indicator_selection() {
    let theme = default_dark_theme();
    let rect = Rect::new(10.25, 20.5, 74.0, 24.0);
    let id = WidgetId::from_key("tab-focus-basic");
    let input = UiInput::default();

    for selected in [false, true] {
        let focused = tab_button(
            id,
            rect,
            "Timeline",
            selected,
            &input,
            &mut focused_memory(id),
            &theme,
            false,
        );
        let unfocused = tab_button(
            id,
            rect,
            "Timeline",
            selected,
            &input,
            &mut UiMemory::new(),
            &theme,
            false,
        );
        assert_focus_pair(&focused, &unfocused, rect, selected);
        assert_eq!(
            unfocused
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Rect(_)))
                .count(),
            1 + indicator_offset(selected)
        );
        assert!(
            unfocused
                .primitives
                .iter()
                .all(|primitive| !matches!(primitive, Primitive::Path(_)))
        );
    }

    let selected = tab_button(
        id,
        rect,
        "Timeline",
        true,
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    let unselected = tab_button(
        id,
        rect,
        "Timeline",
        false,
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    let Primitive::Rect(selected_base) = &selected.primitives[0] else {
        unreachable!()
    };
    let Primitive::Rect(unselected_base) = &unselected.primitives[0] else {
        unreachable!()
    };
    // Selected tabs "merge" with the panel body they front (S2), unselected
    // tabs carry no fill of their own; neither differs by an accent color.
    assert_eq!(
        selected_base.fill,
        Some(Brush::Solid(theme.colors.surface.panel))
    );
    assert_eq!(
        unselected_base.fill,
        Some(Brush::Solid(stern_core::Color::TRANSPARENT))
    );
    assert_eq!(selected_base.stroke, unselected_base.stroke);
    assert_ne!(
        selected_base.stroke.unwrap().brush,
        Brush::Solid(theme.colors.accent.default)
    );
    assert_ne!(
        selected_base.stroke.unwrap().brush,
        Brush::Solid(theme.colors.focus.ring)
    );

    // The only visible distinguishing mark is a neutral top-edge indicator
    // (never accent-colored), immediately after the base surface.
    let Primitive::Rect(indicator) = &selected.primitives[1] else {
        panic!("selected tab indicator must be second");
    };
    assert_eq!(
        indicator.fill,
        Some(Brush::Solid(theme.colors.border.strong))
    );
    assert_ne!(
        indicator.fill,
        Some(Brush::Solid(theme.colors.accent.default))
    );
    assert_ne!(indicator.fill, Some(Brush::Solid(theme.colors.focus.ring)));

    // Text promotes from muted (idle) to primary (selected) — same content
    // and geometry, only the foreground tier differs.
    let Primitive::Text(selected_text) = &selected.primitives[2] else {
        panic!("selected tab label must follow the indicator");
    };
    let Primitive::Text(unselected_text) = &unselected.primitives[1] else {
        panic!("unselected tab label must follow the base surface");
    };
    assert_eq!(selected_text.text, unselected_text.text);
    assert_eq!(selected_text.origin, unselected_text.origin);
    assert_eq!(
        selected_text.brush,
        Brush::Solid(theme.colors.content.primary)
    );
    assert_eq!(
        unselected_text.brush,
        Brush::Solid(theme.colors.content.muted)
    );
}

#[test]
fn tab_focus_is_state_independent_and_disabled_focus_is_suppressed() {
    let theme = default_dark_theme();
    let rect = Rect::new(3.25, 5.75, 70.0, 24.0);
    let id = WidgetId::from_key("tab-focus-combined");
    let baseline = tab_button(
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
        let output = tab_button(
            id,
            rect,
            "State",
            selected,
            &input,
            &mut focused_memory(id),
            &theme,
            false,
        );
        assert_eq!(annuli(&output, selected), annuli(&baseline, false));
    }

    let disabled = tab_button(
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
    assert!(disabled.response.expect("response").state.focused);
    assert!(
        disabled
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
}

#[test]
fn narrow_and_degenerate_reusable_tabs_keep_finite_contained_focus_and_content() {
    let theme = default_dark_theme();
    for (case, rect) in [
        Rect::new(0.25, 0.75, 3.5, 2.0),
        Rect::new(4.25, 8.75, 0.0, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        let id = WidgetId::from_key(format!("tab-degenerate-{case}"));
        let output = tab_button(
            id,
            rect,
            "T",
            false,
            &UiInput::default(),
            &mut focused_memory(id),
            &theme,
            false,
        );
        assert_eq!(output.primitives.len(), 4);
        for primitive in &output.primitives[1..3] {
            let Primitive::Path(path) = primitive else {
                panic!("focus path");
            };
            assert!(rect.contains_rect(path_bounds(&path.elements)));
            assert!(
                path.elements.iter().all(|element| match *element {
                    PathElement::MoveTo(point) | PathElement::LineTo(point) =>
                        point.x.is_finite() && point.y.is_finite(),
                    PathElement::QuadTo { ctrl, to } =>
                        ctrl.x.is_finite()
                            && ctrl.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::CubicTo { ctrl1, ctrl2, to } =>
                        ctrl1.x.is_finite()
                            && ctrl1.y.is_finite()
                            && ctrl2.x.is_finite()
                            && ctrl2.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::Close => true,
                })
            );
        }
        let Primitive::Text(text) = &output.primitives[3] else {
            panic!("tab label after focus");
        };
        assert!(text.origin.x.is_finite() && text.origin.y.is_finite());
        assert!(text.size.is_finite() && text.line_height.is_finite());
    }
}
