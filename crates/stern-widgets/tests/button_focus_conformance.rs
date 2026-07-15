//! Exact inward focus conformance for public button-family widget outputs.

use stern_core::{
    ImageId, PathElement, Point, PointerButtonState, PointerInput, Primitive, Rect, UiInput,
    UiMemory, WidgetId, default_dark_theme,
};
use stern_widgets::{
    IconGraphic, IconId, IconLibrary, IconPath, ItemId, PropertyGridAffordanceLayout,
    PropertyGridRow, WidgetOutput, button, icon_button_with_label, icon_button_with_library,
    image_icon_selectable_button_sized, property_grid_row_affordance_controls,
    property_grid_row_affordance_rects,
};

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

fn base_index(primitives: &[Primitive], rect: Rect) -> usize {
    primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == rect && base.stroke.is_some())
        })
        .expect("button base surface")
}

fn annuli(output: &WidgetOutput, rect: Rect) -> [&Primitive; 2] {
    let base = base_index(&output.primitives, rect);
    [&output.primitives[base + 1], &output.primitives[base + 2]]
}

fn assert_focus_pair(
    focused: &WidgetOutput,
    unfocused: &WidgetOutput,
    rect: Rect,
    expected_radius: stern_core::CornerRadius,
) {
    let theme = default_dark_theme();
    let focused_base = base_index(&focused.primitives, rect);
    let unfocused_base = base_index(&unfocused.primitives, rect);
    assert_eq!(
        focused.primitives[focused_base],
        unfocused.primitives[unfocused_base]
    );
    let Primitive::Rect(base) = &focused.primitives[focused_base] else {
        unreachable!()
    };
    assert_eq!(base.radius, expected_radius);
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(
            rect,
            expected_radius,
            base.stroke.expect("neutral boundary").width,
        );
    assert_eq!(focused.primitives[focused_base + 1], expected[0]);
    assert_eq!(focused.primitives[focused_base + 2], expected[1]);
    for (primitive, brush) in [
        (
            &focused.primitives[focused_base + 1],
            theme.focus_ring(true).unwrap().primary.brush,
        ),
        (
            &focused.primitives[focused_base + 2],
            theme.focus_ring(true).unwrap().separator.brush,
        ),
    ] {
        let Primitive::Path(path) = primitive else {
            panic!("inward focus must be a compound path");
        };
        assert_eq!(path.fill, Some(brush));
        assert_eq!(path.stroke, None);
        assert_eq!(path.elements.len(), 20);
        assert!(rect.contains_rect(path_bounds(&path.elements)));
    }
    assert!(
        focused.primitives.len() > focused_base + 3,
        "content follows annuli"
    );

    let mut stripped = focused.primitives.clone();
    stripped.drain(focused_base + 1..focused_base + 3);
    assert_eq!(stripped, unfocused.primitives);
    assert_eq!(
        focused.response.as_ref().map(|response| response.rect),
        unfocused.response.as_ref().map(|response| response.rect)
    );
    assert_eq!(
        focused
            .semantics
            .iter()
            .map(|node| (node.id, node.bounds, node.role.clone()))
            .collect::<Vec<_>>(),
        unfocused
            .semantics
            .iter()
            .map(|node| (node.id, node.bounds, node.role.clone()))
            .collect::<Vec<_>>()
    );
}

fn focused_memory(id: WidgetId) -> UiMemory {
    let mut memory = UiMemory::new();
    memory.focus(id);
    memory
}

#[test]
#[allow(clippy::too_many_lines)]
fn labeled_bitmap_and_vector_or_missing_icon_buttons_share_exact_inward_surfaces() {
    let theme = default_dark_theme();
    let rect = Rect::new(10.25, 20.5, 38.0, 24.0);
    let input = UiInput::default();

    let label_id = WidgetId::from_key("focus-label");
    let focused = button(
        label_id,
        rect,
        "Apply",
        &input,
        &mut focused_memory(label_id),
        &theme,
        false,
    );
    let unfocused = button(
        label_id,
        rect,
        "Apply",
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert_focus_pair(&focused, &unfocused, rect, theme.radii.sm);

    let image_id = WidgetId::from_key("focus-image");
    let focused_image = image_icon_selectable_button_sized(
        image_id,
        rect,
        ImageId::from_raw(7),
        "Image",
        true,
        14.0,
        &input,
        &mut focused_memory(image_id),
        &theme,
        false,
    );
    let unfocused_image = image_icon_selectable_button_sized(
        image_id,
        rect,
        ImageId::from_raw(7),
        "Image",
        true,
        14.0,
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert_focus_pair(&focused_image, &unfocused_image, rect, theme.radii.sm);
    assert!(matches!(
        focused_image.primitives.last(),
        Some(Primitive::Image(_))
    ));

    let icon = IconId::from_raw(11);
    let mut library = IconLibrary::new();
    library.register(
        icon,
        IconGraphic::new(
            Rect::new(0.0, 0.0, 16.0, 16.0),
            vec![IconPath::filled([
                PathElement::MoveTo(Point::new(2.0, 2.0)),
                PathElement::LineTo(Point::new(14.0, 8.0)),
                PathElement::LineTo(Point::new(2.0, 14.0)),
                PathElement::Close,
            ])],
        ),
    );
    let valid_id = WidgetId::from_key("focus-vector");
    let focused_valid = icon_button_with_library(
        valid_id,
        rect,
        icon,
        "Vector",
        &library,
        &input,
        &mut focused_memory(valid_id),
        &theme,
        false,
    );
    let unfocused_valid = icon_button_with_library(
        valid_id,
        rect,
        icon,
        "Vector",
        &library,
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert_focus_pair(&focused_valid, &unfocused_valid, rect, theme.radii.sm);

    let missing_id = WidgetId::from_key("focus-missing");
    let focused_missing = icon_button_with_label(
        missing_id,
        rect,
        IconId::from_raw(99),
        "Missing",
        &input,
        &mut focused_memory(missing_id),
        &theme,
        false,
    );
    let unfocused_missing = icon_button_with_label(
        missing_id,
        rect,
        IconId::from_raw(99),
        "Missing",
        &input,
        &mut UiMemory::new(),
        &theme,
        false,
    );
    assert_focus_pair(&focused_missing, &unfocused_missing, rect, theme.radii.sm);
}

#[test]
fn focus_annuli_are_independent_from_hover_press_and_selection_and_disabled_suppresses_them() {
    let theme = default_dark_theme();
    let rect = Rect::new(3.25, 5.75, 40.0, 24.0);
    let id = WidgetId::from_key("combined-focus");
    let baseline = button(
        id,
        rect,
        "State",
        &UiInput::default(),
        &mut focused_memory(id),
        &theme,
        false,
    );
    for input in [hovering(rect), pressing(rect)] {
        let output = button(
            id,
            rect,
            "State",
            &input,
            &mut focused_memory(id),
            &theme,
            false,
        );
        assert_eq!(annuli(&output, rect), annuli(&baseline, rect));
    }
    let selected = image_icon_selectable_button_sized(
        id,
        rect,
        ImageId::from_raw(3),
        "Selected",
        true,
        12.0,
        &UiInput::default(),
        &mut focused_memory(id),
        &theme,
        false,
    );
    assert_eq!(annuli(&selected, rect), annuli(&baseline, rect));

    let disabled = button(
        id,
        rect,
        "Disabled",
        &UiInput::default(),
        &mut focused_memory(id),
        &theme,
        true,
    );
    assert_eq!(disabled.primitives.len(), 2);
    assert!(!disabled.response.expect("response").state.focused);
    assert!(
        disabled
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
}

#[test]
fn eighteen_unit_property_affordance_preserves_radius_content_and_contained_annuli() {
    let theme = default_dark_theme();
    let root = WidgetId::from_key("exposure-affordance");
    let row = PropertyGridRow::property(ItemId::from_raw(42), "Exposure", 0)
        .with_resettable(true, false)
        .with_keyframeable(true, false);
    let rects = property_grid_row_affordance_rects(
        &row,
        Rect::new(0.25, 0.75, 96.0, 20.0),
        PropertyGridAffordanceLayout::new(18.0, 4.0),
    );
    let reset_rect = rects.reset_rect.expect("reset rect");
    assert!((reset_rect.width - 18.0).abs() <= f32::EPSILON);
    assert!((reset_rect.height - 18.0).abs() <= f32::EPSILON);
    let focused = property_grid_row_affordance_controls(
        root,
        &row,
        rects,
        &UiInput::default(),
        &mut focused_memory(root.child("reset")),
        &theme,
    );
    let unfocused = property_grid_row_affordance_controls(
        root,
        &row,
        rects,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
    );
    assert_focus_pair(
        &focused.widget,
        &unfocused.widget,
        reset_rect,
        stern_core::CornerRadius::all(3.0),
    );
    let base = base_index(&focused.widget.primitives, reset_rect);
    let Primitive::Text(glyph) = &focused.widget.primitives[base + 3] else {
        panic!("affordance glyph must follow both annuli");
    };
    assert_eq!(glyph.text, "R");
    assert!(glyph.origin.x.is_finite() && glyph.origin.y.is_finite());
    assert!(glyph.size.is_finite() && glyph.line_height.is_finite());
}
