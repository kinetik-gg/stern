#![allow(clippy::float_cmp)]
use super::{
    ButtonVariant, ComponentState, ControlMetrics, ControlSizeScale, DurationScale, ElevationLevel,
    ElevationScale, FontFamilyRole, HandleSizeScale, IconSizeScale, OpacityScale, RadiusScale,
    RowSizeScale, SemanticColor, SizeScale, SizeToken, SpacingScale, StrokeScale, TextRole,
    TextRoleMetrics, ThemeColors, TypographyScale, default_dark_theme,
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

    assert_eq!(theme.spacing.four, 8.0);
    assert_eq!(theme.text_size, 12.0);
    assert_eq!(theme.border_width, 1.0);
    assert_eq!(theme.strokes.hairline, 1.0);
    assert_eq!(theme.strokes.default, 1.0);
    assert_eq!(theme.strokes.emphasis, 2.0);
    assert_eq!(theme.strokes.focus.primary, 1.0);
    assert_eq!(theme.strokes.focus.separator, 1.0);
    assert_eq!(theme.controls.control_height, 28.0);
    assert_eq!(theme.sizes.icon.md, 16.0);
    assert_eq!(theme.font(TextRole::Body).family, "Inter");
    assert_eq!(theme.font(TextRole::Label).family, "Inter");
    assert_eq!(theme.font(TextRole::Caption).family, "Inter");
    assert_eq!(theme.font(TextRole::Title).family, "Inter");
    assert_eq!(theme.font(TextRole::Monospace).family, "Space Mono");
    assert_eq!(theme.font(TextRole::Body).line_height, 17.0);
}

#[test]
fn default_typography_exposes_exact_semantic_families_and_role_metrics() {
    let theme = default_dark_theme();

    assert_eq!(
        FontFamilyRole::ALL,
        &[
            FontFamilyRole::Ui,
            FontFamilyRole::Brand,
            FontFamilyRole::Mono,
        ]
    );
    assert_eq!(theme.font_family(FontFamilyRole::Ui), "Inter");
    assert_eq!(theme.font_family(FontFamilyRole::Brand), "Space Grotesk");
    assert_eq!(theme.font_family(FontFamilyRole::Mono), "Space Mono");
    assert_ne!(
        theme.font_family(FontFamilyRole::Ui),
        theme.font_family(FontFamilyRole::Brand)
    );
    assert_ne!(
        theme.font_family(FontFamilyRole::Ui),
        theme.font_family(FontFamilyRole::Mono)
    );
    assert_ne!(
        theme.font_family(FontFamilyRole::Brand),
        theme.font_family(FontFamilyRole::Mono)
    );

    let expected = [
        (TextRole::Body, "Inter", 12.0, 17.0),
        (TextRole::Label, "Inter", 12.0, 16.0),
        (TextRole::Caption, "Inter", 11.0, 15.0),
        (TextRole::Title, "Inter", 14.0, 19.0),
        (TextRole::Monospace, "Space Mono", 12.0, 17.0),
    ];
    for (role, family, size, line_height) in expected {
        let token = theme.font(role);
        assert_eq!(token.family, family, "wrong family for {role:?}");
        assert_eq!(token.size, size, "wrong size for {role:?}");
        assert_eq!(
            token.line_height, line_height,
            "wrong line height for {role:?}"
        );
    }
}

fn sentinel_sizes() -> SizeScale {
    SizeScale::new(
        ControlSizeScale::new(101.0, 103.0, 107.0, 109.0),
        RowSizeScale::new(113.0, 127.0),
        131.0,
        137.0,
        139.0,
        IconSizeScale::new(149.0, 151.0, 157.0),
        HandleSizeScale::new(163.0, 167.0),
    )
}

#[test]
fn size_scale_defaults_and_typed_lookup_are_exact() {
    let sizes = default_dark_theme().sizes;
    let expected = [
        20.0, 24.0, 28.0, 32.0, 24.0, 28.0, 28.0, 30.0, 40.0, 12.0, 16.0, 20.0, 1.0, 7.0,
    ];

    assert_eq!(SizeToken::ALL.len(), expected.len());
    for (token, expected) in SizeToken::ALL.iter().copied().zip(expected) {
        assert_eq!(sizes.get(token), expected, "wrong value for {token:?}");
    }
    assert_ne!(sizes.handle.visual, sizes.handle.hit);
}

#[test]
fn size_replacement_is_isolated_from_theme_and_control_metrics() {
    let mut baseline = default_dark_theme();
    baseline.colors.surface.application = Color::rgb8(1, 2, 3);
    baseline.spacing = SpacingScale::new(
        173.0, 179.0, 181.0, 191.0, 193.0, 197.0, 199.0, 211.0, 223.0,
    );
    baseline.radii = RadiusScale::from_values(227.0, 229.0, 233.0, 239.0);
    baseline.strokes = StrokeScale::from_values(241.0, 251.0, 257.0, 263.0, 269.0);
    baseline.typography.body.size = 271.0;
    baseline.opacity.hover = 277.0;
    baseline.elevation.low = 281.0;
    baseline.duration.normal = 283.0;
    baseline.controls = ControlMetrics {
        control_height: 293.0,
        compact_control_height: 307.0,
        padding_x: 317.0,
        padding_y: 331.0,
    };
    baseline.radius = CornerRadius::all(337.0);
    baseline.border_width = 347.0;
    baseline.text_size = 349.0;

    let sizes = sentinel_sizes();
    let customized = baseline.with_sizes(sizes);

    assert_eq!(customized.sizes, sizes);
    assert_eq!(customized.colors, baseline.colors);
    assert_eq!(customized.spacing, baseline.spacing);
    assert_eq!(customized.radii, baseline.radii);
    assert_eq!(customized.strokes, baseline.strokes);
    assert_eq!(customized.typography, baseline.typography);
    assert_eq!(customized.opacity, baseline.opacity);
    assert_eq!(customized.elevation, baseline.elevation);
    assert_eq!(customized.duration, baseline.duration);
    assert_eq!(customized.controls, baseline.controls);
    assert_eq!(customized.radius, baseline.radius);
    assert_eq!(customized.border_width, baseline.border_width);
    assert_eq!(customized.text_size, baseline.text_size);

    let respaced = customized.with_spacing(SpacingScale::new(
        353.0, 359.0, 367.0, 373.0, 379.0, 383.0, 389.0, 397.0, 401.0,
    ));
    assert_eq!(respaced.sizes, sizes);
    assert_eq!(respaced.controls, baseline.controls);
}

#[test]
fn control_metrics_defaults_and_customization_remain_independent() {
    let defaults = default_dark_theme();
    assert_eq!(defaults.controls.control_height, 28.0);
    assert_eq!(defaults.controls.compact_control_height, 22.0);
    assert_eq!(defaults.controls.padding_x, 8.0);
    assert_eq!(defaults.controls.padding_y, 4.0);

    let controls = ControlMetrics {
        control_height: 409.0,
        compact_control_height: 419.0,
        padding_x: 433.0,
        padding_y: 439.0,
    };
    let customized = defaults
        .with_controls(controls)
        .with_sizes(sentinel_sizes());
    assert_eq!(customized.controls, controls);
    assert_eq!(customized.sizes, sentinel_sizes());
}

#[test]
fn radius_scale_defaults_and_customization_are_exact() {
    let theme = default_dark_theme();

    assert_eq!(theme.radii.none, CornerRadius::all(0.0));
    assert_eq!(theme.radii.sm, CornerRadius::all(3.0));
    assert_eq!(theme.radii.md, CornerRadius::all(6.0));
    assert_eq!(theme.radii.lg, CornerRadius::all(12.0));
    assert_eq!(theme.radii.full, CornerRadius::all(9999.0));
    assert_eq!(theme.radius, theme.radii.sm);

    let radii = RadiusScale::from_values(4.0, 8.0, 16.0, 2048.0);
    assert_eq!(radii.none, CornerRadius::all(0.0));
    assert_eq!(radii.sm, CornerRadius::all(4.0));
    assert_eq!(radii.md, CornerRadius::all(8.0));
    assert_eq!(radii.lg, CornerRadius::all(16.0));
    assert_eq!(radii.full, CornerRadius::all(2048.0));

    let customized = theme.with_radii(radii);
    assert_eq!(customized.radii, radii);
    assert_eq!(customized.radius, radii.sm);
}

#[test]
fn stroke_scale_defaults_customization_and_legacy_mirror_are_exact() {
    let base = default_dark_theme();
    assert_eq!(base.strokes.hairline, 1.0);
    assert_eq!(base.strokes.default, 1.0);
    assert_eq!(base.strokes.emphasis, 2.0);
    assert_eq!(base.strokes.focus.primary, 1.0);
    assert_eq!(base.strokes.focus.separator, 1.0);
    assert_eq!(base.border_width, base.strokes.default);

    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    assert_eq!(strokes.hairline, 0.75);
    assert_eq!(strokes.default, 1.25);
    assert_eq!(strokes.emphasis, 2.5);
    assert_eq!(strokes.focus.primary, 3.5);
    assert_eq!(strokes.focus.separator, 4.5);

    let customized = base.with_strokes(strokes);
    assert_eq!(customized.strokes, strokes);
    assert_eq!(customized.border_width, strokes.default);
}

#[test]
fn controls_and_legacy_mirror_cannot_mutate_stroke_authority() {
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let controls = ControlMetrics {
        control_height: 31.0,
        compact_control_height: 19.0,
        padding_x: 9.0,
        padding_y: 5.0,
    };
    let mut theme = default_dark_theme()
        .with_strokes(strokes)
        .with_controls(controls);
    assert_eq!(theme.controls, controls);
    assert_eq!(theme.strokes, strokes);
    assert_eq!(theme.border_width, strokes.default);

    theme.border_width = 99.0;
    assert_eq!(theme.strokes, strokes);
    assert_eq!(
        theme.button(ComponentState::default()).border.width,
        strokes.default
    );
    assert_eq!(
        theme.row(ComponentState::default()).border.width,
        strokes.hairline
    );
    assert_eq!(theme.separator().stroke.width, strokes.hairline);
    assert_eq!(
        theme
            .tab(ComponentState {
                selected: true,
                ..ComponentState::default()
            })
            .indicator_thickness,
        strokes.emphasis
    );
}

#[test]
fn canonical_component_recipes_use_radius_roles_by_intent() {
    let theme = default_dark_theme().with_radii(RadiusScale::from_values(4.0, 11.0, 23.0, 777.0));
    let states = [
        ComponentState::default(),
        ComponentState {
            hovered: true,
            ..ComponentState::default()
        },
        ComponentState {
            selected: true,
            focused: true,
            ..ComponentState::default()
        },
        ComponentState {
            disabled: true,
            ..ComponentState::default()
        },
    ];

    for state in states {
        for variant in [
            ButtonVariant::Standard,
            ButtonVariant::Primary,
            ButtonVariant::Ghost,
            ButtonVariant::Danger,
        ] {
            let radius = theme.button_variant(variant, state).radius;
            assert_eq!(radius, theme.radii.sm);
            assert_ne!(radius, theme.radii.full);
        }
        assert_eq!(theme.tab(state).radius, theme.radii.none);
        assert_ne!(theme.tab(state).radius, theme.radii.full);
        assert_eq!(theme.row(state).radius, theme.radii.none);
        assert_eq!(theme.text_field(state).radius, theme.radii.sm);
        assert_ne!(theme.text_field(state).radius, theme.radii.full);
        assert_eq!(theme.checkbox(state).radius, theme.radii.sm);
        assert_eq!(theme.radio_button(state).radius, theme.radii.full);
        assert_eq!(theme.slider(state).radius, theme.radii.full);
    }
}

#[test]
fn token_overrides_are_structural_and_predictable() {
    let typography = TypographyScale {
        body: TextRoleMetrics::new(13.0, 18.0),
        ..default_dark_theme().typography
    };
    let controls = ControlMetrics {
        padding_x: 10.0,
        ..default_dark_theme().controls
    };
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let theme = default_dark_theme()
        .with_spacing(SpacingScale::new(
            1.0, 3.0, 6.0, 9.0, 12.0, 15.0, 18.0, 21.0, 24.0,
        ))
        .with_radii(RadiusScale::from_values(2.0, 3.0, 4.0, 999.0))
        .with_typography(typography)
        .with_opacity(OpacityScale {
            hover: 0.2,
            ..default_dark_theme().opacity
        })
        .with_elevation(ElevationScale {
            low: 3.0,
            ..default_dark_theme().elevation
        })
        .with_duration(DurationScale {
            normal: 180.0,
            ..default_dark_theme().duration
        })
        .with_strokes(strokes)
        .with_controls(controls);

    assert_eq!(theme.spacing.zero, 1.0);
    assert_eq!(theme.spacing.two, 6.0);
    assert_eq!(theme.radii.sm, CornerRadius::all(2.0));
    assert_eq!(theme.radius, CornerRadius::all(2.0));
    assert_eq!(theme.text_size, 13.0);
    assert_eq!(theme.opacity.hover, 0.2);
    assert_eq!(theme.elevation.low, 3.0);
    assert_eq!(theme.duration.normal, 180.0);
    assert_eq!(theme.controls.padding_x, 10.0);
    assert_eq!(theme.strokes, strokes);
    assert_eq!(theme.border_width, strokes.default);
    assert_eq!(theme.colors, default_dark_theme().colors);
}

#[test]
fn elevation_scale_defaults_and_typed_lookup_are_exact() {
    let theme = default_dark_theme();

    assert_eq!(theme.elevation.none, 0.0);
    assert_eq!(theme.elevation.low, 1.0);
    assert_eq!(theme.elevation.medium, 2.0);
    assert_eq!(theme.elevation.high, 3.0);
    assert_eq!(theme.elevation.get(ElevationLevel::None), 0.0);
    assert_eq!(theme.elevation.get(ElevationLevel::Low), 1.0);
    assert_eq!(theme.elevation.get(ElevationLevel::Medium), 2.0);
    assert_eq!(theme.elevation.get(ElevationLevel::High), 3.0);

    let customized = theme.with_elevation(ElevationScale::new(10.0, 20.0, 30.0, 40.0));
    assert_eq!(customized.elevation.get(ElevationLevel::None), 10.0);
    assert_eq!(customized.elevation.get(ElevationLevel::Low), 20.0);
    assert_eq!(customized.elevation.get(ElevationLevel::Medium), 30.0);
    assert_eq!(customized.elevation.get(ElevationLevel::High), 40.0);
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
    assert_eq!(
        focused.border.brush,
        Brush::Solid(theme.colors.border.default)
    );
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
        assert_eq!(recipe.border.width, theme.strokes.default);
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
    assert_eq!(
        focused_hover.border.brush,
        Brush::Solid(colors.border.default)
    );
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

    assert_eq!(theme.tab(selected).indicator, None);
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
        Brush::Solid(theme.colors.border.default)
    );
    assert_eq!(
        theme.text_field(focused).border.brush,
        Brush::Solid(theme.colors.border.focused)
    );
    assert!(theme.panel().shadow.is_none());
}

#[test]
fn selection_indicator_recipe_size_is_exact_across_component_states() {
    let theme = default_dark_theme();
    let states = [
        ComponentState::default(),
        ComponentState {
            hovered: true,
            ..ComponentState::default()
        },
        ComponentState {
            focused: true,
            ..ComponentState::default()
        },
        ComponentState {
            selected: true,
            ..ComponentState::default()
        },
        ComponentState {
            hovered: true,
            focused: true,
            selected: true,
            ..ComponentState::default()
        },
        ComponentState {
            disabled: true,
            ..ComponentState::default()
        },
        ComponentState {
            hovered: true,
            focused: true,
            disabled: true,
            selected: true,
            ..ComponentState::default()
        },
    ];

    for state in states {
        let checkbox = theme.checkbox(state);
        let radio = theme.radio_button(state);
        assert_eq!(checkbox.size, 14.0, "wrong checkbox size for {state:?}");
        assert_eq!(radio.size, 14.0, "wrong radio size for {state:?}");
        assert_eq!(radio.size, checkbox.size);
    }
}

#[test]
fn canonical_recipes_route_distinct_stroke_roles_without_focused_state_width_changes() {
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let theme = default_dark_theme().with_strokes(strokes);
    let unfocused = ComponentState::default();
    let focused = ComponentState {
        focused: true,
        ..ComponentState::default()
    };

    for state in [unfocused, focused] {
        assert_eq!(theme.button(state).border.width, strokes.default);
        assert_eq!(theme.tab(state).border.width, strokes.default);
        assert_eq!(theme.checkbox(state).border.width, strokes.default);
        assert_eq!(theme.radio_button(state).border.width, strokes.default);
        assert_eq!(theme.toggle(state).border.width, strokes.default);
        assert_eq!(theme.slider(state).border.width, strokes.default);
        assert_eq!(theme.text_field(state).border.width, strokes.default);
        assert_eq!(theme.panel().border.width, strokes.default);
        assert_eq!(theme.row(state).border.width, strokes.hairline);
    }

    assert_eq!(theme.separator().stroke.width, strokes.hairline);
    assert_eq!(
        theme
            .tab(ComponentState {
                selected: true,
                ..ComponentState::default()
            })
            .indicator_thickness,
        strokes.emphasis
    );
    assert_eq!(theme.strokes.focus.primary, 3.5);
    assert_eq!(theme.strokes.focus.separator, 4.5);
}

#[test]
fn passive_panel_recipe_stays_flat_under_nonzero_elevation() {
    let background = Color::rgb8(1, 2, 3);
    let border = Color::rgb8(4, 5, 6);
    let border_width = 2.75;
    let radius = CornerRadius::all(5.5);
    let base = default_dark_theme();
    let mut colors = ThemeColors::default_dark();
    colors.surface.panel_raised = background;
    colors.border.default = border;
    let theme = base
        .with_colors(colors)
        .with_strokes(StrokeScale::from_values(1.0, border_width, 2.0, 1.0, 1.0))
        .with_radii(RadiusScale::from_values(5.5, 7.0, 9.0, 99.0))
        .with_elevation(ElevationScale {
            low: 37.0,
            ..base.elevation
        });

    let recipe = theme.panel();

    assert_eq!(recipe.background, Brush::Solid(background));
    assert_eq!(recipe.border.brush, Brush::Solid(border));
    assert_eq!(recipe.border.width, border_width);
    assert_eq!(recipe.radius, radius);
    assert_eq!(recipe.shadow, None);
    assert!(
        theme
            .elevation_shadow(ElevationLevel::Low, radius.top_left)
            .is_some(),
        "positive elevation tokens must still resolve shadows for elevated consumers"
    );
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
    colors.border.default = Color::rgb8(31, 32, 33);
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
        Brush::Solid(colors.border.default)
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
fn elevation_shadow_recipes_are_exact_and_preserve_shape_radius() {
    let theme = default_dark_theme();
    let rect = crate::Rect::new(0.0, 0.0, 20.0, 10.0);

    assert_eq!(theme.elevation_shadow(ElevationLevel::None, 7.0), None);
    for (level, offset_y, blur_radius, alpha) in [
        (ElevationLevel::Low, 2.0, 6.0, 0.32),
        (ElevationLevel::Medium, 6.0, 18.0, 0.42),
        (ElevationLevel::High, 12.0, 36.0, 0.52),
    ] {
        let shadow = theme
            .elevation_shadow(level, 7.0)
            .expect("visible elevation casts a shadow");
        assert_eq!(shadow.offset, crate::Vec2::new(0.0, offset_y));
        assert_eq!(shadow.blur_radius, blur_radius);
        assert_eq!(shadow.spread, 0.0);
        assert_eq!(shadow.radius, 7.0);
        assert_eq!(shadow.color, Color::rgba(0.0, 0.0, 0.0, alpha));

        let primitive = shadow.primitive(rect);
        assert_eq!(primitive.rect, rect);
        assert_eq!(primitive.offset, crate::Vec2::new(0.0, offset_y));
        assert_eq!(primitive.blur_radius, blur_radius);
        assert_eq!(primitive.spread, 0.0);
        assert_eq!(primitive.radius, 7.0);
        assert_eq!(primitive.color, Color::rgba(0.0, 0.0, 0.0, alpha));
    }

    let clamped = theme
        .elevation_shadow(ElevationLevel::Medium, -7.0)
        .expect("visible elevation casts a shadow");
    assert_eq!(clamped.offset, crate::Vec2::new(0.0, 6.0));
    assert_eq!(clamped.blur_radius, 18.0);
    assert_eq!(clamped.spread, 0.0);
    assert_eq!(clamped.radius, 0.0);
    assert_eq!(clamped.color, Color::rgba(0.0, 0.0, 0.0, 0.42));
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
