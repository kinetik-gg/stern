#![allow(clippy::float_cmp)]
use super::{
    ButtonVariant, ComponentState, ControlMetrics, DurationScale, ElevationScale, OpacityScale,
    RadiusScale, SemanticColor, SpacingScale, TextRole, ThemeColors, TypographyScale,
    default_dark_theme,
};
use crate::{Brush, Color, CornerRadius};

#[test]
fn resolves_semantic_colors() {
    let theme = default_dark_theme();

    assert_eq!(
        theme.color(SemanticColor::AccentDefault),
        theme.colors.accent.default
    );
    assert_eq!(
        theme.color(SemanticColor::ContentMuted),
        theme.colors.content.muted
    );
    assert_eq!(
        theme.color(SemanticColor::SurfaceOverlay),
        theme.colors.surface.overlay
    );
}

#[test]
fn default_theme_has_dense_editor_spacing() {
    let theme = default_dark_theme();

    assert_eq!(theme.spacing.md, 8.0);
    assert_eq!(theme.text_size, 12.0);
    assert_eq!(theme.border_width, 1.0);
    assert_eq!(theme.controls.control_height, 28.0);
    assert_eq!(theme.controls.icon_size, 16.0);
    assert_eq!(theme.font(TextRole::Body).family, "Inter");
    assert_eq!(theme.font(TextRole::Label).family, "Inter");
    assert_eq!(theme.font(TextRole::Caption).family, "Inter");
    assert_eq!(theme.font(TextRole::Title).family, "Inter");
    assert_eq!(theme.font(TextRole::Monospace).family, "Geist Mono");
    assert_eq!(theme.font(TextRole::Body).line_height, 17.0);
}

#[test]
fn token_overrides_are_structural_and_predictable() {
    let typography = TypographyScale {
        body: super::FontToken::new("sans-serif", 13.0, 18.0),
        ..default_dark_theme().typography
    };
    let controls = ControlMetrics {
        border_width: 2.0,
        ..default_dark_theme().controls
    };
    let theme = default_dark_theme()
        .with_spacing(SpacingScale::new(1.0, 3.0, 6.0, 9.0, 12.0))
        .with_radii(RadiusScale::from_values(1.0, 2.0, 3.0, 4.0, 999.0))
        .with_typography(typography)
        .with_opacity(OpacityScale {
            hover: 0.2,
            ..default_dark_theme().opacity
        })
        .with_elevation(ElevationScale {
            raised: 3.0,
            ..default_dark_theme().elevation
        })
        .with_duration(DurationScale {
            normal: 180.0,
            ..default_dark_theme().duration
        })
        .with_controls(controls);

    assert_eq!(theme.spacing.xs, 1.0);
    assert_eq!(theme.spacing.md, 6.0);
    assert_eq!(theme.radii.sm, CornerRadius::all(2.0));
    assert_eq!(theme.radius, CornerRadius::all(2.0));
    assert_eq!(theme.text_size, 13.0);
    assert_eq!(theme.opacity.hover, 0.2);
    assert_eq!(theme.elevation.raised, 3.0);
    assert_eq!(theme.duration.normal, 180.0);
    assert_eq!(theme.controls.border_width, 2.0);
    assert_eq!(theme.border_width, 2.0);
    assert_eq!(theme.colors, default_dark_theme().colors);
}

#[test]
fn button_recipe_uses_state_colors() {
    let theme = default_dark_theme();

    let normal = theme.button(ComponentState::default());
    let hovered = theme.button(ComponentState {
        hovered: true,
        ..ComponentState::default()
    });
    let focused = theme.button(ComponentState {
        focused: true,
        ..ComponentState::default()
    });
    let disabled = theme.button(ComponentState {
        disabled: true,
        ..ComponentState::default()
    });
    let primary = theme.button_variant(ButtonVariant::Primary, ComponentState::default());

    assert_eq!(
        normal.background,
        Brush::Solid(theme.colors.surface.control)
    );
    assert_eq!(
        hovered.background,
        Brush::Solid(theme.colors.surface.control_hover)
    );
    assert_eq!(focused.border.brush, Brush::Solid(theme.colors.focus.ring));
    assert_eq!(disabled.foreground, theme.colors.content.disabled);
    assert_eq!(
        primary.background,
        Brush::Solid(theme.colors.accent.default)
    );
    assert_eq!(primary.foreground, theme.colors.content.on_accent);
}

const PRESERVED_BUTTON_STATES: [(&str, ComponentState); 7] = [
    (
        "normal",
        ComponentState {
            hovered: false,
            pressed: false,
            focused: false,
            disabled: false,
            selected: false,
        },
    ),
    (
        "hovered",
        ComponentState {
            hovered: true,
            pressed: false,
            focused: false,
            disabled: false,
            selected: false,
        },
    ),
    (
        "selected",
        ComponentState {
            hovered: false,
            pressed: false,
            focused: false,
            disabled: false,
            selected: true,
        },
    ),
    (
        "pressed",
        ComponentState {
            hovered: false,
            pressed: true,
            focused: false,
            disabled: false,
            selected: false,
        },
    ),
    (
        "selected and hovered",
        ComponentState {
            hovered: true,
            pressed: false,
            focused: false,
            disabled: false,
            selected: true,
        },
    ),
    (
        "pressed and hovered",
        ComponentState {
            hovered: true,
            pressed: true,
            focused: false,
            disabled: false,
            selected: false,
        },
    ),
    (
        "disabled",
        ComponentState {
            hovered: false,
            pressed: false,
            focused: false,
            disabled: true,
            selected: false,
        },
    ),
];

struct PreservedButtonVariantCase {
    name: &'static str,
    variant: ButtonVariant,
    backgrounds: [Color; 7],
    enabled_foreground: Color,
    border: Color,
}

fn assert_preserved_button_variant(theme: &super::Theme, case: &PreservedButtonVariantCase) {
    for ((state_name, state), expected_background) in PRESERVED_BUTTON_STATES
        .iter()
        .copied()
        .zip(case.backgrounds)
    {
        let recipe = theme.button_variant(case.variant, state);
        let expected_foreground = if state.disabled {
            theme.colors.content.disabled
        } else {
            case.enabled_foreground
        };
        assert_eq!(
            recipe.background,
            Brush::Solid(expected_background),
            "wrong {} {state_name} background",
            case.name
        );
        assert_eq!(
            recipe.foreground, expected_foreground,
            "wrong {} {state_name} foreground",
            case.name
        );
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(case.border),
            "wrong {} {state_name} border",
            case.name
        );
        assert_eq!(recipe.border.width, theme.controls.border_width);
        assert_eq!(recipe.radius, theme.radii.sm);
    }
}

#[test]
fn non_primary_button_variants_preserve_existing_outcomes() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.control = Color::rgb8(1, 2, 3);
    colors.surface.control_hover = Color::rgb8(4, 5, 6);
    colors.surface.control_pressed = Color::rgb8(7, 8, 9);
    colors.surface.control_disabled = Color::rgb8(10, 11, 12);
    colors.status.danger.strong = Color::rgb8(13, 14, 15);
    colors.content.primary = Color::rgb8(16, 17, 18);
    colors.content.on_accent = Color::rgb8(19, 20, 21);
    colors.content.disabled = Color::rgb8(22, 23, 24);
    colors.border.default = Color::rgb8(25, 26, 27);
    colors.border.subtle = Color::rgb8(28, 29, 30);
    let theme = default_dark_theme().with_colors(colors);

    // Danger alpha values are legacy recipe characterization, not status conformance evidence.
    let danger_hover = colors.status.danger.strong.with_alpha(0.92);
    let danger_active = colors.status.danger.strong.with_alpha(0.86);
    let cases = [
        PreservedButtonVariantCase {
            name: "Standard",
            variant: ButtonVariant::Standard,
            backgrounds: [
                colors.surface.control,
                colors.surface.control_hover,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_disabled,
            ],
            enabled_foreground: colors.content.primary,
            border: colors.border.default,
        },
        PreservedButtonVariantCase {
            name: "Ghost",
            variant: ButtonVariant::Ghost,
            backgrounds: [
                Color::TRANSPARENT,
                colors.surface.control_hover,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_pressed,
                colors.surface.control_disabled,
            ],
            enabled_foreground: colors.content.primary,
            border: colors.border.subtle,
        },
        PreservedButtonVariantCase {
            name: "Danger",
            variant: ButtonVariant::Danger,
            backgrounds: [
                colors.status.danger.strong,
                danger_hover,
                danger_active,
                danger_active,
                danger_active,
                danger_active,
                colors.surface.control_disabled,
            ],
            enabled_foreground: colors.content.on_accent,
            border: colors.border.default,
        },
    ];

    for case in &cases {
        assert_preserved_button_variant(&theme, case);
    }
}

fn assert_primary_button_state(
    theme: &super::Theme,
    baseline: &super::ButtonRecipe,
    name: &str,
    state: ComponentState,
    expected_background: Color,
    expected_foreground: Color,
) {
    let recipe = theme.button_variant(ButtonVariant::Primary, state);
    assert_eq!(
        recipe.background,
        Brush::Solid(expected_background),
        "wrong background for {name}"
    );
    assert_eq!(
        recipe.foreground, expected_foreground,
        "wrong foreground for {name}"
    );
    assert_eq!(recipe.radius, baseline.radius, "wrong radius for {name}");
    assert_eq!(
        recipe.border.width, baseline.border.width,
        "wrong border width for {name}"
    );
    assert_eq!(
        recipe.border.brush, baseline.border.brush,
        "wrong border brush for {name}"
    );
}

type PrimaryButtonCase = (&'static str, ComponentState, Color, Color);

fn primary_button_cases(colors: &ThemeColors) -> [PrimaryButtonCase; 8] {
    [
        (
            "normal",
            ComponentState::default(),
            colors.accent.default,
            colors.content.on_accent,
        ),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.accent.hover,
            colors.content.on_accent,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.accent.default,
            colors.content.on_accent,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            colors.accent.pressed,
            colors.content.on_accent,
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
            "selected and hovered",
            ComponentState {
                selected: true,
                hovered: true,
                ..ComponentState::default()
            },
            colors.accent.default,
            colors.content.on_accent,
        ),
        (
            "pressed, selected, and hovered",
            ComponentState {
                pressed: true,
                selected: true,
                hovered: true,
                ..ComponentState::default()
            },
            colors.accent.pressed,
            colors.content.on_accent,
        ),
        (
            "disabled with every active state",
            ComponentState {
                disabled: true,
                pressed: true,
                selected: true,
                hovered: true,
                ..ComponentState::default()
            },
            colors.surface.control_disabled,
            colors.content.disabled,
        ),
    ]
}

#[test]
fn primary_button_uses_exact_accent_roles_and_bounded_state_precedence() {
    let mut colors = ThemeColors::default_dark();
    colors.accent.default = Color::rgb8(1, 2, 3);
    colors.accent.hover = Color::rgb8(4, 5, 6);
    colors.accent.pressed = Color::rgb8(7, 8, 9);
    colors.focus.ring = Color::rgb8(10, 11, 12);
    colors.surface.control_disabled = Color::rgb8(13, 14, 15);
    colors.content.on_accent = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    colors.border.default = Color::rgb8(22, 23, 24);
    let theme = default_dark_theme().with_colors(colors);
    let normal = theme.button_variant(ButtonVariant::Primary, ComponentState::default());

    for (name, state, expected_background, expected_foreground) in primary_button_cases(&colors) {
        assert_primary_button_state(
            &theme,
            &normal,
            name,
            state,
            expected_background,
            expected_foreground,
        );
    }

    let focused_hover = theme.button_variant(
        ButtonVariant::Primary,
        ComponentState {
            focused: true,
            hovered: true,
            ..ComponentState::default()
        },
    );
    assert_eq!(focused_hover.background, Brush::Solid(colors.accent.hover));
    assert_eq!(focused_hover.border.brush, Brush::Solid(colors.focus.ring));
    assert_eq!(focused_hover.border.width, normal.border.width);
    assert_eq!(focused_hover.radius, normal.radius);
    assert_eq!(focused_hover.foreground, normal.foreground);
}

#[test]
fn selected_row_remains_selected_while_hovered() {
    let mut colors = ThemeColors::default_dark();
    colors.selection.background = Color::rgb8(1, 2, 3);
    colors.selection.foreground = Color::rgb8(4, 5, 6);
    colors.surface.hover = Color::rgb8(7, 8, 9);
    colors.content.primary = Color::rgb8(10, 11, 12);
    let theme = default_dark_theme().with_colors(colors);
    let selected_row = theme.row(ComponentState {
        selected: true,
        hovered: true,
        ..ComponentState::default()
    });
    assert_eq!(
        selected_row.background,
        Brush::Solid(colors.selection.background)
    );
    assert_eq!(selected_row.foreground, colors.selection.foreground);
}

#[test]
fn component_recipes_cover_common_states() {
    let theme = default_dark_theme();
    let selected = ComponentState {
        selected: true,
        ..ComponentState::default()
    };
    let focused = ComponentState {
        focused: true,
        ..ComponentState::default()
    };

    assert_eq!(
        theme.tab(selected).indicator,
        Some(Brush::Solid(theme.colors.accent.default))
    );
    assert_eq!(
        theme.row(selected).background,
        Brush::Solid(theme.colors.selection.background)
    );
    assert_eq!(
        theme.row(selected).foreground,
        theme.colors.selection.foreground
    );
    assert_eq!(
        theme.checkbox(selected).fill,
        Brush::Solid(theme.colors.accent.default)
    );
    assert_eq!(
        theme.toggle(selected).track,
        Brush::Solid(theme.colors.accent.default)
    );
    assert_eq!(
        theme.slider(focused).border.brush,
        Brush::Solid(theme.colors.focus.ring)
    );
    assert_eq!(
        theme.text_field(focused).border.brush,
        Brush::Solid(theme.colors.border.focused)
    );
    assert!(theme.panel().shadow.is_some());
}

#[test]
fn recipe_lookups_follow_independently_overridden_semantic_paths() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.control = Color::rgb8(1, 2, 3);
    colors.surface.control_hover = Color::rgb8(4, 5, 6);
    colors.content.primary = Color::rgb8(7, 8, 9);
    colors.content.on_accent = Color::rgb8(10, 11, 12);
    colors.border.subtle = Color::rgb8(13, 14, 15);
    colors.selection.background = Color::rgb8(16, 17, 18);
    colors.focus.ring = Color::rgb8(19, 20, 21);
    colors.accent.default = Color::rgb8(22, 23, 24);
    colors.border.focused = Color::rgb8(25, 26, 27);
    colors.selection.foreground = Color::rgb8(28, 29, 30);
    let theme = default_dark_theme().with_colors(colors);

    assert_eq!(
        theme.button(ComponentState::default()).background,
        Brush::Solid(colors.surface.control)
    );
    assert_eq!(
        theme
            .button(ComponentState {
                hovered: true,
                ..ComponentState::default()
            })
            .background,
        Brush::Solid(colors.surface.control_hover)
    );
    assert_eq!(
        theme.label(TextRole::Body, false).foreground,
        colors.content.primary
    );
    assert_eq!(
        theme
            .button_variant(ButtonVariant::Primary, ComponentState::default())
            .foreground,
        colors.content.on_accent
    );
    assert_eq!(
        theme
            .button_variant(ButtonVariant::Ghost, ComponentState::default())
            .border
            .brush,
        Brush::Solid(colors.border.subtle)
    );
    assert_eq!(
        theme
            .row(ComponentState {
                selected: true,
                ..ComponentState::default()
            })
            .background,
        Brush::Solid(colors.selection.background)
    );
    assert_eq!(
        theme
            .row(ComponentState {
                selected: true,
                ..ComponentState::default()
            })
            .foreground,
        colors.selection.foreground
    );
    assert_eq!(
        theme
            .button(ComponentState {
                focused: true,
                ..ComponentState::default()
            })
            .border
            .brush,
        Brush::Solid(colors.focus.ring)
    );
    assert_eq!(
        theme
            .button_variant(ButtonVariant::Primary, ComponentState::default())
            .background,
        Brush::Solid(colors.accent.default)
    );
    assert_eq!(
        theme
            .text_field(ComponentState {
                focused: true,
                ..ComponentState::default()
            })
            .border
            .brush,
        Brush::Solid(colors.border.focused)
    );
}

#[test]
fn elevation_shadow_materializes_shadow_primitives() {
    let theme = default_dark_theme();
    let shadow = theme
        .elevation_shadow(theme.elevation.overlay, theme.radii.md.top_left)
        .expect("overlay elevation casts a shadow");
    let primitive = shadow.primitive(crate::Rect::new(0.0, 0.0, 20.0, 10.0));

    assert_eq!(primitive.rect, crate::Rect::new(0.0, 0.0, 20.0, 10.0));
    assert!(primitive.blur_radius > theme.elevation.overlay);
    assert_eq!(primitive.radius, theme.radii.md.top_left);
}

#[test]
fn active_selection_uses_blue_accent_family() {
    let theme = default_dark_theme();

    assert_eq!(
        theme.colors.accent.default,
        theme.colors.selection.background
    );
    assert!(theme.colors.accent.default.b > theme.colors.accent.default.r);
    assert!(theme.colors.accent.default.b > theme.colors.accent.default.g);
}

#[test]
fn transparent_color_remains_available() {
    assert_eq!(Color::TRANSPARENT.a, 0.0);
}
