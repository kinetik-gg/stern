//! Public exact stroke token construction conformance.

#![allow(clippy::float_cmp)]

#[test]
fn qualified_core_stroke_types_construct_and_expose_exact_roles() {
    let strokes: stern_core::StrokeScale =
        stern_core::StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let focus: stern_core::FocusStrokeScale = strokes.focus;

    assert_eq!(strokes.hairline, 0.75);
    assert_eq!(strokes.default, 1.25);
    assert_eq!(strokes.emphasis, 2.5);
    assert_eq!(focus.primary, 3.5);
    assert_eq!(focus.separator, 4.5);

    let theme = stern_core::default_dark_theme().with_strokes(strokes);
    assert_eq!(theme.strokes, strokes);
    assert_eq!(theme.border_width, strokes.default);
}
