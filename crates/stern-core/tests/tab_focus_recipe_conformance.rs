//! Exact focus-neutral and indicator-free tab recipe conformance.

#![allow(clippy::float_cmp)]

use stern_core::{Brush, Color, ComponentState, StrokeScale, ThemeColors, default_dark_theme};

fn with_focus(mut state: ComponentState, focused: bool) -> ComponentState {
    state.focused = focused;
    state
}

#[test]
fn tab_recipe_is_focus_neutral_indicator_free_and_preserves_state_precedence() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.panel = Color::rgb8(1, 2, 3);
    colors.surface.hover = Color::rgb8(4, 5, 6);
    colors.surface.control_pressed = Color::rgb8(7, 8, 9);
    colors.surface.control_disabled = Color::rgb8(10, 11, 12);
    colors.content.primary = Color::rgb8(13, 14, 15);
    colors.content.disabled = Color::rgb8(16, 17, 18);
    colors.border.default = Color::rgb8(19, 20, 21);
    colors.focus.ring = Color::rgb8(22, 23, 24);
    colors.accent.default = Color::rgb8(25, 26, 27);
    let strokes = StrokeScale::from_values(0.5, 1.75, 2.5, 3.25, 4.5);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(strokes);
    let cases = [
        ("default", ComponentState::default(), colors.surface.panel),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            colors.surface.control_pressed,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.control_pressed,
        ),
        (
            "hovered-selected",
            ComponentState {
                hovered: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.control_pressed,
        ),
        (
            "pressed-selected",
            ComponentState {
                pressed: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.control_pressed,
        ),
        (
            "disabled",
            ComponentState {
                hovered: true,
                pressed: true,
                selected: true,
                disabled: true,
                ..ComponentState::default()
            },
            colors.surface.control_disabled,
        ),
    ];

    for (name, state, expected_background) in cases {
        let unfocused = theme.tab(with_focus(state, false));
        let focused = theme.tab(with_focus(state, true));
        assert_eq!(focused, unfocused, "focus-only recipe delta for {name}");
        assert_eq!(focused.background, Brush::Solid(expected_background));
        assert_eq!(
            focused.foreground,
            if state.disabled {
                colors.content.disabled
            } else {
                colors.content.primary
            }
        );
        assert_eq!(focused.border.width, strokes.default);
        assert_eq!(focused.border.brush, Brush::Solid(colors.border.default));
        assert_ne!(focused.border.brush, Brush::Solid(colors.focus.ring));
        assert_ne!(focused.border.brush, Brush::Solid(colors.accent.default));
        assert_eq!(focused.radius, theme.radii.none);
        assert_eq!(focused.indicator, None);
        assert_eq!(focused.indicator_thickness, strokes.emphasis);
    }
}
