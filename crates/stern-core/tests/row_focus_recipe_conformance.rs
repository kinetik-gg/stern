//! Exact focus-neutral row recipe and selected-color exception conformance.

#![allow(clippy::float_cmp)]

use stern_core::{Brush, Color, ComponentState, StrokeScale, ThemeColors, default_dark_theme};

fn with_focus(mut state: ComponentState, focused: bool) -> ComponentState {
    state.focused = focused;
    state
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

#[test]
#[allow(clippy::too_many_lines)]
fn row_recipe_is_focus_neutral_and_preserves_exact_state_precedence() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.sunken = Color::rgb8(1, 2, 3);
    colors.surface.hover = Color::rgb8(4, 5, 6);
    colors.surface.control_disabled = Color::rgb8(7, 8, 9);
    colors.selection.background = Color::rgb8(10, 11, 12);
    colors.selection.foreground = Color::rgb8(13, 14, 15);
    colors.content.primary = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    colors.border.subtle = Color::rgb8(22, 23, 24);
    colors.focus.ring = Color::rgb8(25, 26, 27);
    colors.accent.default = Color::rgb8(28, 29, 30);
    let strokes = StrokeScale::from_values(0.75, 1.5, 2.25, 3.5, 4.5);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(strokes);
    let cases = [
        (
            "default",
            ComponentState::default(),
            colors.surface.sunken,
            colors.content.primary,
        ),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.content.primary,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            colors.surface.sunken,
            colors.content.primary,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.selection.background,
            colors.selection.foreground,
        ),
        (
            "hovered-selected",
            ComponentState {
                hovered: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.selection.background,
            colors.selection.foreground,
        ),
        (
            "pressed-selected",
            ComponentState {
                pressed: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.selection.background,
            colors.selection.foreground,
        ),
        (
            "disabled",
            ComponentState {
                disabled: true,
                ..ComponentState::default()
            },
            colors.surface.control_disabled,
            colors.content.disabled,
        ),
        (
            "disabled-focused-hovered-pressed-selected",
            ComponentState {
                hovered: true,
                pressed: true,
                focused: true,
                disabled: true,
                selected: true,
            },
            colors.surface.control_disabled,
            colors.content.disabled,
        ),
    ];

    for (name, state, expected_background, expected_foreground) in cases {
        let unfocused = theme.row(with_focus(state, false));
        let focused = theme.row(with_focus(state, true));
        assert_eq!(focused, unfocused, "focus-only recipe delta for {name}");
        assert_eq!(
            focused.background,
            Brush::Solid(expected_background),
            "{name}"
        );
        assert_eq!(focused.foreground, expected_foreground, "{name}");
        assert_eq!(focused.border.width, strokes.hairline, "{name}");
        assert_eq!(
            focused.border.brush,
            Brush::Solid(colors.border.subtle),
            "{name}"
        );
        assert_ne!(
            focused.border.brush,
            Brush::Solid(colors.focus.ring),
            "{name}"
        );
        assert_ne!(
            focused.border.brush,
            Brush::Solid(colors.accent.default),
            "{name}"
        );
        assert_eq!(focused.radius, theme.radii.none, "{name}");
    }
}

#[test]
fn selected_row_states_inventory_the_white_on_blue_product_exception() {
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

    for (name, state) in [
        (
            "selected-only",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
        ),
        (
            "selected-hovered",
            ComponentState {
                hovered: true,
                selected: true,
                ..ComponentState::default()
            },
        ),
        (
            "selected-pressed",
            ComponentState {
                pressed: true,
                selected: true,
                ..ComponentState::default()
            },
        ),
        (
            "selected-focused",
            ComponentState {
                focused: true,
                selected: true,
                ..ComponentState::default()
            },
        ),
        (
            "selected-focused-hovered",
            ComponentState {
                focused: true,
                hovered: true,
                selected: true,
                ..ComponentState::default()
            },
        ),
    ] {
        let recipe = theme.row(state);
        assert_eq!(
            recipe.background,
            Brush::Solid(theme.colors.selection.background),
            "{name}"
        );
        assert_eq!(
            recipe.foreground, theme.colors.selection.foreground,
            "{name}"
        );
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(theme.colors.border.subtle),
            "{name}"
        );
    }
}
