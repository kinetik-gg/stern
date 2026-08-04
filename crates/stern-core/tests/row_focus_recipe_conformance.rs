//! Exact row-recipe focus/selection precedence and selected-color exception
//! conformance.
//!
//! `row_recipe_promotes_fill_on_focus_exactly_like_hover_and_never_elsewhere`
//! was rewritten for family issue #915: `docs/visual-spec/06-collections.md`
//! §Rows explicitly requires "focused (kbd, not selected): S4 + inset focus
//! ring" — a keyboard-focused, otherwise-idle row DOES promote its fill to
//! the hover tier, unlike buttons/tabs where focus never recolors the body.
//! The prior version of this test asserted focus was a no-op recipe delta in
//! every case; that premise contradicted the spec's explicit row-focus row
//! and has been replaced with per-case expected backgrounds. Border, radius,
//! and text color remain untouched by focus in all cases (the two-layer ring
//! itself is a separate, additive paint step per `00-language.md` §Focus
//! model, never baked into the recipe's border field).

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
fn row_recipe_promotes_fill_on_focus_exactly_like_hover_and_never_elsewhere() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.hover = Color::rgb8(4, 5, 6);
    colors.selection.background = Color::rgb8(10, 11, 12);
    colors.selection.foreground = Color::rgb8(13, 14, 15);
    colors.content.secondary = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    colors.border.subtle = Color::rgb8(22, 23, 24);
    colors.focus.ring = Color::rgb8(25, 26, 27);
    colors.accent.default = Color::rgb8(28, 29, 30);
    let strokes = StrokeScale::from_values(0.75, 1.5, 2.25, 3.5, 4.5);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(strokes);

    // (name, state sans focus, unfocused background, focused background, foreground)
    let cases = [
        (
            "default",
            ComponentState::default(),
            Color::TRANSPARENT,
            colors.surface.hover,
            colors.content.secondary,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            Color::TRANSPARENT,
            colors.surface.hover,
            colors.content.secondary,
        ),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.surface.hover,
            colors.content.secondary,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.selection.background,
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
            colors.selection.background,
            colors.selection.foreground,
        ),
        (
            "disabled",
            ComponentState {
                disabled: true,
                ..ComponentState::default()
            },
            Color::TRANSPARENT,
            Color::TRANSPARENT,
            colors.content.disabled,
        ),
        (
            "disabled-hovered-pressed-selected",
            ComponentState {
                hovered: true,
                pressed: true,
                disabled: true,
                selected: true,
                ..ComponentState::default()
            },
            Color::TRANSPARENT,
            Color::TRANSPARENT,
            colors.content.disabled,
        ),
    ];

    for (
        name,
        state,
        expected_unfocused_background,
        expected_focused_background,
        expected_foreground,
    ) in cases
    {
        let unfocused = theme.row(with_focus(state, false));
        let focused = theme.row(with_focus(state, true));

        assert_eq!(
            unfocused.background,
            Brush::Solid(expected_unfocused_background),
            "{name} unfocused"
        );
        assert_eq!(
            focused.background,
            Brush::Solid(expected_focused_background),
            "{name} focused"
        );
        assert_eq!(unfocused.foreground, expected_foreground, "{name}");
        assert_eq!(
            focused.foreground, expected_foreground,
            "{name} focus never changes text"
        );

        // Focus never touches border or radius: the two-layer ring is a
        // separate, additive paint step, not a recipe field.
        assert_eq!(focused.border, unfocused.border, "{name}");
        assert_eq!(focused.radius, unfocused.radius, "{name}");
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
