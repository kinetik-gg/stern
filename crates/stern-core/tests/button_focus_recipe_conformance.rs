//! Exact focus-neutral button recipe conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    Brush, ButtonVariant, ComponentState, StrokeScale, ThemeColors, default_dark_theme,
};

fn with_focus(mut state: ComponentState, focused: bool) -> ComponentState {
    state.focused = focused;
    state
}

/// Per `docs/visual-spec/01-buttons.md`, every variant's non-disabled border
/// promotes to `border.strong` (Ghost: from transparent) whenever the
/// control is pressed, hovered, or "chosen" (`selected` without `pressed`,
/// the mode-choice state) — Primary stays transparent throughout, and Danger
/// promotes to its own `status.danger.strong` instead. This mirrors
/// `Theme::button_variant`'s border precedence so this test's real subject —
/// that `focused` never adds a delta of its own — has a correct baseline to
/// compare against per state, instead of one constant for every state.
fn expected_unfocused_border(
    variant: ButtonVariant,
    state: ComponentState,
    colors: &ThemeColors,
) -> stern_core::Color {
    if state.disabled {
        return colors.border.disabled;
    }
    let chosen = state.selected && !state.pressed;
    let emphasized = state.pressed || state.hovered || chosen;
    match variant {
        ButtonVariant::Standard => {
            if emphasized {
                colors.border.strong
            } else {
                colors.border.default
            }
        }
        ButtonVariant::Ghost => {
            if emphasized {
                colors.border.strong
            } else {
                stern_core::Color::TRANSPARENT
            }
        }
        ButtonVariant::Primary => stern_core::Color::TRANSPARENT,
        ButtonVariant::Danger => {
            if state.pressed || state.hovered {
                colors.status.danger.strong
            } else {
                colors.status.danger.border
            }
        }
    }
}

#[test]
fn every_button_variant_is_focus_neutral_across_the_complete_state_matrix() {
    let mut colors = ThemeColors::default_dark();
    colors.border.default = stern_core::Color::rgb8(0x12, 0x34, 0x56);
    colors.border.strong = stern_core::Color::rgb8(0x65, 0x43, 0x21);
    colors.focus.ring = stern_core::Color::rgb8(0xFE, 0x01, 0x7A);
    let strokes = StrokeScale::from_values(0.5, 1.75, 2.5, 3.25, 4.5);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(strokes);
    let states = [
        ComponentState::default(),
        ComponentState {
            hovered: true,
            ..ComponentState::default()
        },
        ComponentState {
            pressed: true,
            ..ComponentState::default()
        },
        ComponentState {
            selected: true,
            ..ComponentState::default()
        },
        ComponentState {
            hovered: true,
            pressed: true,
            selected: true,
            ..ComponentState::default()
        },
        ComponentState {
            disabled: true,
            ..ComponentState::default()
        },
    ];

    for variant in [
        ButtonVariant::Standard,
        ButtonVariant::Primary,
        ButtonVariant::Ghost,
        ButtonVariant::Danger,
    ] {
        for state in states {
            let unfocused = theme.button_variant(variant, with_focus(state, false));
            let focused = theme.button_variant(variant, with_focus(state, true));
            assert_eq!(
                focused, unfocused,
                "focus-only delta for {variant:?} {state:?}"
            );
            assert_eq!(focused.border.width, strokes.default);
            assert_eq!(focused.radius, theme.radii.sm);
            assert_eq!(
                focused.border.brush,
                Brush::Solid(expected_unfocused_border(variant, state, &colors)),
                "wrong border for {variant:?} {state:?}"
            );
            assert_ne!(focused.border.brush, Brush::Solid(colors.focus.ring));
        }
    }
}

#[test]
fn disabled_focused_buttons_keep_disabled_visual_precedence() {
    let theme = default_dark_theme();
    let disabled = ComponentState {
        hovered: true,
        pressed: true,
        disabled: true,
        selected: true,
        ..ComponentState::default()
    };
    for variant in [
        ButtonVariant::Standard,
        ButtonVariant::Primary,
        ButtonVariant::Ghost,
        ButtonVariant::Danger,
    ] {
        let recipe = theme.button_variant(variant, with_focus(disabled, true));
        assert_eq!(
            recipe,
            theme.button_variant(variant, with_focus(disabled, false))
        );
        assert_eq!(
            recipe.background,
            // 01-buttons.md: every variant's disabled fill is S1
            // `surface.application`, not the `surface.control_disabled`
            // tier other families use.
            Brush::Solid(theme.colors.surface.application)
        );
        assert_eq!(recipe.foreground, theme.colors.content.disabled);
    }
}
