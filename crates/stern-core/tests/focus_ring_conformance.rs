//! Exact public focus-ring recipe and geometry conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    Brush, Color, CornerRadius, FocusRingRecipe, Primitive, Rect, StrokeScale, ThemeColors,
    default_dark_theme,
};

#[test]
fn focus_ring_visibility_and_default_tokens_are_exact() {
    let theme = default_dark_theme();

    assert_eq!(theme.focus_ring(false), None);
    let recipe: FocusRingRecipe = theme.focus_ring(true).expect("visible focus ring");
    assert_eq!(recipe.primary.width, 1.0);
    assert_eq!(
        recipe.primary.brush,
        Brush::Solid(Color::rgb8(0x4D, 0xB2, 0xFF))
    );
    assert_eq!(recipe.separator.width, 1.0);
    assert_eq!(
        recipe.separator.brush,
        Brush::Solid(Color::rgb8(0x0B, 0x0B, 0x0B))
    );
}

#[test]
fn focus_ring_uses_distinct_sentinel_tokens_and_exact_nonuniform_geometry() {
    let mut colors = ThemeColors::default_dark();
    colors.focus.indicator = Color::rgb8(0x12, 0x34, 0x56);
    colors.focus.separator = Color::rgb8(0xA1, 0xB2, 0xC3);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(StrokeScale::from_values(0.5, 1.5, 2.5, 3.5, 4.5));
    let recipe = theme.focus_ring(true).expect("visible focus ring");

    assert_eq!(recipe.primary.width, 3.5);
    assert_eq!(
        recipe.primary.brush,
        Brush::Solid(Color::rgb8(0x12, 0x34, 0x56))
    );
    assert_eq!(recipe.separator.width, 4.5);
    assert_eq!(
        recipe.separator.brush,
        Brush::Solid(Color::rgb8(0xA1, 0xB2, 0xC3))
    );

    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let radius = CornerRadius {
        top_left: 1.0,
        top_right: 2.0,
        bottom_right: 3.0,
        bottom_left: 4.0,
    };
    let [primary, separator] = recipe.primitives(rect, radius);
    let Primitive::Rect(primary) = primary else {
        panic!("expected primary rectangle");
    };
    let Primitive::Rect(separator) = separator else {
        panic!("expected separator rectangle");
    };

    assert_eq!(primary.rect, Rect::new(2.0, 12.0, 46.0, 56.0));
    assert_eq!(primary.fill, Some(recipe.primary.brush));
    assert_eq!(primary.stroke, None);
    assert_eq!(
        primary.radius,
        CornerRadius {
            top_left: 9.0,
            top_right: 10.0,
            bottom_right: 11.0,
            bottom_left: 12.0,
        }
    );
    assert_eq!(separator.rect, Rect::new(5.5, 15.5, 39.0, 49.0));
    assert_eq!(separator.fill, Some(recipe.separator.brush));
    assert_eq!(separator.stroke, None);
    assert_eq!(
        separator.radius,
        CornerRadius {
            top_left: 5.5,
            top_right: 6.5,
            bottom_right: 7.5,
            bottom_left: 8.5,
        }
    );
}

#[test]
fn focused_choice_and_slider_recipes_retain_neutral_base_borders() {
    use stern_core::ComponentState;

    let theme = default_dark_theme();
    let focused = ComponentState {
        focused: true,
        ..ComponentState::default()
    };
    let unfocused = ComponentState::default();

    assert_eq!(
        theme.checkbox(focused).border,
        theme.checkbox(unfocused).border
    );
    assert_eq!(
        theme.radio_button(focused).border,
        theme.radio_button(unfocused).border
    );
    assert_eq!(theme.toggle(focused).border, theme.toggle(unfocused).border);
    assert_eq!(theme.slider(focused).border, theme.slider(unfocused).border);
    assert_eq!(
        theme.checkbox(focused).border.brush,
        Brush::Solid(theme.colors.border.default)
    );
}
