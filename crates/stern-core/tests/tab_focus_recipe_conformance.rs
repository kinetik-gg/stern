//! Exact focus-neutral tab recipe conformance.
//!
//! `focused` never changes fill/border/text/indicator on its own — the
//! two-layer focus ring is a separate, additive paint step per
//! `docs/visual-spec/00-language.md` §Focus model, mirroring
//! `button_focus_recipe_conformance.rs`. Per-state background/foreground/
//! border/radius/indicator values match
//! `docs/visual-spec/05-chrome-dock.md` §Frame tab strip (family issue
//! #914): a selected tab merges its fill with the panel body (S2) and is
//! marked only by a `border.strong` top-edge indicator, never a
//! full-perimeter border or an accent color.

#![allow(clippy::float_cmp)]

use stern_core::{
    Brush, Color, ComponentState, CornerRadius, StrokeScale, ThemeColors, default_dark_theme,
};

fn with_focus(mut state: ComponentState, focused: bool) -> ComponentState {
    state.focused = focused;
    state
}

#[test]
#[allow(clippy::too_many_lines)]
fn tab_recipe_is_focus_neutral_and_preserves_selected_state_precedence() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.panel = Color::rgb8(1, 2, 3);
    colors.surface.hover = Color::rgb8(4, 5, 6);
    colors.surface.control_pressed = Color::rgb8(7, 8, 9);
    colors.surface.control_disabled = Color::rgb8(10, 11, 12);
    colors.content.primary = Color::rgb8(13, 14, 15);
    colors.content.disabled = Color::rgb8(16, 17, 18);
    colors.content.muted = Color::rgb8(28, 29, 30);
    colors.border.default = Color::rgb8(19, 20, 21);
    colors.border.strong = Color::rgb8(31, 32, 33);
    colors.focus.ring = Color::rgb8(22, 23, 24);
    colors.accent.default = Color::rgb8(25, 26, 27);
    let strokes = StrokeScale::from_values(0.5, 1.75, 2.5, 3.25, 4.5);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(strokes);
    let expected_radius = CornerRadius {
        top_left: theme.radii.sm.top_left,
        top_right: theme.radii.sm.top_right,
        bottom_left: 0.0,
        bottom_right: 0.0,
    };

    let cases = [
        (
            "default",
            ComponentState::default(),
            Color::TRANSPARENT,
            colors.content.muted,
            None,
        ),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.content.primary,
            None,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.content.primary,
            None,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.panel,
            colors.content.primary,
            Some(colors.border.strong),
        ),
        (
            "hovered-selected",
            ComponentState {
                hovered: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.panel,
            colors.content.primary,
            Some(colors.border.strong),
        ),
        (
            "pressed-selected",
            ComponentState {
                pressed: true,
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.panel,
            colors.content.primary,
            Some(colors.border.strong),
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
            colors.content.disabled,
            None,
        ),
    ];

    for (name, state, expected_background, expected_foreground, expected_indicator) in cases {
        let unfocused = theme.tab(with_focus(state, false));
        let focused = theme.tab(with_focus(state, true));
        assert_eq!(focused, unfocused, "focus-only recipe delta for {name}");

        assert_eq!(
            focused.background,
            Brush::Solid(expected_background),
            "background for {name}"
        );
        assert_eq!(
            focused.foreground, expected_foreground,
            "foreground for {name}"
        );

        // Never a full-perimeter border, and never accent — the selected
        // tab's only mark is the top-edge indicator, asserted below.
        assert_eq!(
            focused.border.width, strokes.default,
            "border width for {name}"
        );
        assert_eq!(
            focused.border.brush,
            Brush::Solid(Color::TRANSPARENT),
            "border brush for {name}"
        );
        assert_ne!(focused.border.brush, Brush::Solid(colors.focus.ring));
        assert_ne!(focused.border.brush, Brush::Solid(colors.accent.default));

        assert_eq!(focused.radius, expected_radius, "radius for {name}");
        assert_ne!(focused.radius, theme.radii.full);

        assert_eq!(
            focused.indicator,
            expected_indicator.map(Brush::Solid),
            "indicator for {name}"
        );
        if let Some(indicator) = focused.indicator {
            assert_ne!(indicator, Brush::Solid(colors.accent.default));
            assert_ne!(indicator, Brush::Solid(colors.focus.ring));
        }
        assert_eq!(focused.indicator_thickness, strokes.emphasis);
    }
}
