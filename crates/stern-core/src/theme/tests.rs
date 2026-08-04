#![allow(clippy::float_cmp)]
use super::{
    ButtonVariant, ComponentState, ControlMetrics, ControlSizeScale, DurationScale, ElevationLevel,
    ElevationScale, FontFamilyRole, HandleSizeScale, IconSizeScale, OpacityScale,
    OverlaySurfaceTier, RadiusScale, RadiusToken, RowSizeScale, SemanticColor, SizeScale,
    SizeToken, SpacingRole, SpacingScale, SpacingStep, StrokeScale, TextRole, TextRoleMetrics,
    ThemeColors, TypographyScale, default_dark_theme, generated_tokens,
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

/// `docs/visual-spec/04-overlays.md` (family issue #913): "Scrim:
/// `overlay.scrim` `#0B0B0B` at 38% opacity" — the default theme previously
/// carried an unconformed 55%.
#[test]
fn default_theme_modal_scrim_opacity_matches_visual_spec() {
    let theme = default_dark_theme();
    assert_eq!(theme.opacity.overlay_scrim, 0.38);
    assert_eq!(theme.colors.overlay.scrim, Color::rgb8(0x0B, 0x0B, 0x0B));
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

/// The seven button interaction-state combinations distinct enough to need
/// individually asserted recipe output. "Chosen" (`selected` without
/// `pressed`) is the mode-choice state documented in
/// `docs/visual-spec/01-buttons.md` §Icon button / 00-language.md
/// §Selection-vs-hover doctrine (selectable icon button "chosen" mode); a
/// transient `pressed` always takes precedence over it.
const BUTTON_VARIANT_STATES: [(&str, ComponentState); 7] = [
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
        "chosen (selected, not pressed)",
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
        "chosen and hovered",
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

struct ButtonVariantCase {
    name: &'static str,
    variant: ButtonVariant,
    backgrounds: [Color; 7],
    foregrounds: [Color; 7],
    borders: [Color; 7],
}

fn assert_button_variant_case(theme: &super::Theme, case: &ButtonVariantCase) {
    for (index, (state_name, state)) in BUTTON_VARIANT_STATES.iter().copied().enumerate() {
        let recipe = theme.button_variant(case.variant, state);
        assert_eq!(
            recipe.background,
            Brush::Solid(case.backgrounds[index]),
            "wrong {} {state_name} background",
            case.name
        );
        assert_eq!(
            recipe.foreground, case.foregrounds[index],
            "wrong {} {state_name} foreground",
            case.name
        );
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(case.borders[index]),
            "wrong {} {state_name} border",
            case.name
        );
        assert_eq!(recipe.border.width, theme.strokes.default);
        assert_eq!(recipe.radius, theme.radii.sm);
    }
}

/// Pins `theme.button_variant` outputs to `docs/visual-spec/01-buttons.md`'s
/// per-state Standard/Ghost/Danger tables (family issue #910). Primary is
/// covered separately below since its accent-role precedence predates this
/// issue and is left intact (01-buttons.md's Primary table has no "selected"
/// row; the DS never exercises a selectable primary button).
#[test]
#[allow(clippy::too_many_lines)]
fn button_variants_match_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.control = Color::rgb8(1, 2, 3);
    colors.surface.control_hover = Color::rgb8(4, 5, 6);
    colors.surface.control_pressed = Color::rgb8(7, 8, 9);
    colors.surface.application = Color::rgb8(10, 11, 12);
    colors.status.danger.strong = Color::rgb8(13, 14, 15);
    colors.content.primary = Color::rgb8(16, 17, 18);
    colors.content.on_accent = Color::rgb8(19, 20, 21);
    colors.content.disabled = Color::rgb8(22, 23, 24);
    colors.border.default = Color::rgb8(25, 26, 27);
    colors.border.strong = Color::rgb8(31, 32, 33);
    colors.border.disabled = Color::rgb8(34, 35, 36);
    colors.content.secondary = Color::rgb8(37, 38, 39);
    colors.status.danger.surface = Color::rgb8(40, 41, 42);
    colors.status.danger.border = Color::rgb8(43, 44, 45);
    colors.status.danger.foreground = Color::rgb8(46, 47, 48);
    let theme = default_dark_theme().with_colors(colors);

    // Standard and Ghost: idle/hover/pressed/chosen/disabled per
    // 01-buttons.md's default-variant table; chosen fill/text match idle
    // (S3, secondary) with the border alone promoted to `border.strong`,
    // and (per doctrine) always take precedence over hover for fill/text.
    let neutral_foregrounds = [
        colors.content.secondary, // normal
        colors.content.primary,   // hovered
        colors.content.secondary, // chosen
        colors.content.primary,   // pressed
        colors.content.secondary, // chosen and hovered
        colors.content.primary,   // pressed and hovered
        colors.content.disabled,  // disabled
    ];
    let neutral_borders = [
        colors.border.default,
        colors.border.strong,
        colors.border.strong,
        colors.border.strong,
        colors.border.strong,
        colors.border.strong,
        colors.border.disabled,
    ];

    let cases = [
        ButtonVariantCase {
            name: "Standard",
            variant: ButtonVariant::Standard,
            backgrounds: [
                colors.surface.control,
                colors.surface.control_hover,
                colors.surface.control,
                colors.surface.control_pressed,
                colors.surface.control,
                colors.surface.control_pressed,
                colors.surface.application,
            ],
            foregrounds: neutral_foregrounds,
            borders: neutral_borders,
        },
        ButtonVariantCase {
            name: "Ghost",
            variant: ButtonVariant::Ghost,
            backgrounds: [
                Color::TRANSPARENT,
                colors.surface.control_hover,
                colors.surface.control,
                colors.surface.control_pressed,
                colors.surface.control,
                colors.surface.control_pressed,
                colors.surface.application,
            ],
            foregrounds: neutral_foregrounds,
            borders: [
                Color::TRANSPARENT,
                colors.border.strong,
                colors.border.strong,
                colors.border.strong,
                colors.border.strong,
                colors.border.strong,
                colors.border.disabled,
            ],
        },
        ButtonVariantCase {
            name: "Danger",
            variant: ButtonVariant::Danger,
            backgrounds: [
                colors.status.danger.surface,
                colors.status.danger.surface,
                colors.status.danger.surface,
                colors.status.danger.surface,
                colors.status.danger.surface,
                colors.status.danger.surface,
                colors.surface.application,
            ],
            foregrounds: [
                colors.status.danger.foreground, // normal
                colors.status.danger.foreground, // hovered
                colors.status.danger.foreground, // chosen (not a DS concept; inert)
                colors.content.on_accent,        // pressed
                colors.status.danger.foreground, // chosen and hovered
                colors.content.on_accent,        // pressed and hovered
                colors.content.disabled,         // disabled
            ],
            borders: [
                colors.status.danger.border,
                colors.status.danger.strong,
                colors.status.danger.border,
                colors.status.danger.strong,
                colors.status.danger.strong,
                colors.status.danger.strong,
                colors.border.disabled,
            ],
        },
    ];

    for case in &cases {
        assert_button_variant_case(&theme, case);
    }
}

/// Pins `Theme::text_field` outputs to `docs/visual-spec/02-fields.md`'s
/// single-line field state table (family issue #911). Fill never varies
/// across idle/hovered/focused (only disabled diverges) — fields read as
/// wells, buttons as raised. `focused` and `hovered` both resolve the border
/// to `border.strong`; unlike buttons, `focused` alone (without hover) does
/// change the border relative to idle, because 02-fields.md's own table
/// specifies that promotion explicitly (the two-layer ring, asserted
/// separately below, is still a purely additive layer on top of it — the
/// border itself never becomes the ring color `border.focused`).
/// `read-only` and `invalid` are in the spec's state table but are not
/// resolved here; see `KNOWN-GAPS.md`.
#[test]
fn text_field_matches_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.control = Color::rgb8(1, 2, 3);
    colors.surface.control_disabled = Color::rgb8(4, 5, 6);
    colors.content.primary = Color::rgb8(7, 8, 9);
    colors.content.disabled = Color::rgb8(10, 11, 12);
    colors.border.default = Color::rgb8(13, 14, 15);
    colors.border.strong = Color::rgb8(16, 17, 18);
    colors.border.disabled = Color::rgb8(19, 20, 21);
    colors.selection.background = Color::rgb8(22, 23, 24);
    colors.focus.ring = Color::rgb8(25, 26, 27);
    let theme = default_dark_theme().with_colors(colors);

    let states = [
        ("idle", ComponentState::default(), colors.border.default),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.border.strong,
        ),
        (
            "focused",
            ComponentState {
                focused: true,
                ..ComponentState::default()
            },
            colors.border.strong,
        ),
        (
            "focused and hovered",
            ComponentState {
                focused: true,
                hovered: true,
                ..ComponentState::default()
            },
            colors.border.strong,
        ),
        (
            "disabled",
            ComponentState {
                disabled: true,
                ..ComponentState::default()
            },
            colors.border.disabled,
        ),
    ];

    for (name, state, expected_border) in states {
        let recipe = theme.text_field(state);
        let expected_background = if state.disabled {
            colors.surface.control_disabled
        } else {
            colors.surface.control
        };
        let expected_foreground = if state.disabled {
            colors.content.disabled
        } else {
            colors.content.primary
        };
        assert_eq!(
            recipe.background,
            Brush::Solid(expected_background),
            "wrong {name} background"
        );
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(expected_border),
            "wrong {name} border"
        );
        assert_eq!(
            recipe.foreground, expected_foreground,
            "wrong {name} foreground"
        );
        // Caret and selection are constant across every state per
        // 02-fields.md's Caret note and Selection-highlight sentence: caret
        // is always `focus.ring`, selection highlight is always
        // `selection.background` at full opacity (no alpha blend).
        assert_eq!(recipe.caret, colors.focus.ring, "wrong {name} caret");
        assert_eq!(
            recipe.selection,
            Brush::Solid(colors.selection.background),
            "wrong {name} selection"
        );
        // "padding-inline 8": the field's per-side horizontal inset equals
        // `controls.padding_x` directly (not halved).
        assert_eq!(
            recipe.padding_x, theme.controls.padding_x,
            "wrong {name} padding_x"
        );
    }
}

fn assert_primary_button_state(
    theme: &super::Theme,
    baseline: &super::ButtonRecipe,
    name: &str,
    state: ComponentState,
    expected_background: Color,
    expected_foreground: Color,
    expected_border: Color,
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
        recipe.border.brush,
        Brush::Solid(expected_border),
        "wrong border brush for {name}"
    );
}

// name, state, expected background, expected foreground, expected border.
// 01-buttons.md's Primary table: border is "none (transparent)" for every
// enabled state, disabled falls back to S1 `surface.application` fill /
// `border.disabled` (same disabled rule every variant shares).
type PrimaryButtonCase = (&'static str, ComponentState, Color, Color, Color);

fn primary_button_cases(colors: &ThemeColors) -> [PrimaryButtonCase; 8] {
    [
        (
            "normal",
            ComponentState::default(),
            colors.accent.default,
            colors.content.on_accent,
            Color::TRANSPARENT,
        ),
        (
            "hovered",
            ComponentState {
                hovered: true,
                ..ComponentState::default()
            },
            colors.accent.hover,
            colors.content.on_accent,
            Color::TRANSPARENT,
        ),
        (
            "selected",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.accent.default,
            colors.content.on_accent,
            Color::TRANSPARENT,
        ),
        (
            "pressed",
            ComponentState {
                pressed: true,
                ..ComponentState::default()
            },
            colors.accent.pressed,
            colors.content.on_accent,
            Color::TRANSPARENT,
        ),
        (
            "disabled",
            ComponentState {
                disabled: true,
                ..ComponentState::default()
            },
            colors.surface.application,
            colors.content.disabled,
            colors.border.disabled,
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
            Color::TRANSPARENT,
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
            Color::TRANSPARENT,
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
            colors.surface.application,
            colors.content.disabled,
            colors.border.disabled,
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
    colors.surface.application = Color::rgb8(13, 14, 15);
    colors.content.on_accent = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    colors.border.disabled = Color::rgb8(22, 23, 24);
    let theme = default_dark_theme().with_colors(colors);
    let normal = theme.button_variant(ButtonVariant::Primary, ComponentState::default());

    for (name, state, expected_background, expected_foreground, expected_border) in
        primary_button_cases(&colors)
    {
        assert_primary_button_state(
            &theme,
            &normal,
            name,
            state,
            expected_background,
            expected_foreground,
            expected_border,
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
    assert_eq!(focused_hover.border.brush, Brush::Solid(Color::TRANSPARENT));
    assert_eq!(focused_hover.border.width, normal.border.width);
    assert_eq!(focused_hover.radius, normal.radius);
    assert_eq!(focused_hover.foreground, normal.foreground);
}

/// Pins `theme.overlay_surface` outputs to `docs/visual-spec/04-overlays.md`
/// (family issue #913): `Menu` (menu/context-menu/dropdown/popover) and
/// `Tooltip` share `surface.overlay`/`border.default`, differing only in
/// radius (`md` vs `sm`); `Panel` (modal/command-palette) sits one tier
/// deeper on `surface.panel` with `border.strong`.
#[test]
fn overlay_surface_matches_visual_spec_tiers() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.overlay = Color::rgb8(1, 2, 3);
    colors.surface.panel = Color::rgb8(4, 5, 6);
    colors.border.default = Color::rgb8(7, 8, 9);
    colors.border.strong = Color::rgb8(10, 11, 12);
    let theme = default_dark_theme().with_colors(colors);

    let menu = theme.overlay_surface(OverlaySurfaceTier::Menu);
    assert_eq!(menu.background, Brush::Solid(colors.surface.overlay));
    assert_eq!(menu.border.brush, Brush::Solid(colors.border.default));
    assert_eq!(menu.border.width, theme.strokes.default);
    assert_eq!(menu.radius, theme.radii.md);

    let tooltip = theme.overlay_surface(OverlaySurfaceTier::Tooltip);
    assert_eq!(tooltip.background, menu.background);
    assert_eq!(tooltip.border, menu.border);
    assert_eq!(tooltip.radius, theme.radii.sm);
    assert_ne!(tooltip.radius, menu.radius);

    let panel = theme.overlay_surface(OverlaySurfaceTier::Panel);
    assert_eq!(panel.background, Brush::Solid(colors.surface.panel));
    assert_eq!(panel.border.brush, Brush::Solid(colors.border.strong));
    assert_eq!(panel.border.width, theme.strokes.default);
    assert_eq!(panel.radius, theme.radii.md);
}

/// Pins `theme.overlay_item` outputs to `docs/visual-spec/04-overlays.md`'s
/// Menu "Item state" table (family issue #913). Rest is transparent with
/// secondary text (not `Theme::row`'s S0-sunken/primary-text default);
/// `selected` here is the keyboard-highlight sense, not data selection, so
/// it maps to the same neutral highlight as `hovered` rather than the accent
/// selection brush; `focused` alone (no mouse hover) also promotes to the
/// highlight, since menu items combine the focus ring with the hover fill
/// (00-language.md §Focus model) — unlike `Theme::row`, which is focus
/// neutral. The border is always transparent (menu items have no per-row
/// border) and the radius is `radius.sm`.
#[test]
fn overlay_item_matches_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.hover = Color::rgb8(1, 2, 3);
    colors.content.primary = Color::rgb8(4, 5, 6);
    colors.content.secondary = Color::rgb8(7, 8, 9);
    colors.content.disabled = Color::rgb8(10, 11, 12);
    colors.selection.background = Color::rgb8(13, 14, 15);
    colors.selection.foreground = Color::rgb8(16, 17, 18);
    let theme = default_dark_theme().with_colors(colors);

    let cases = [
        (
            "rest",
            ComponentState::default(),
            Color::TRANSPARENT,
            colors.content.secondary,
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
            "focused (kbd, no hover)",
            ComponentState {
                focused: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.content.primary,
        ),
        (
            "selected (keyboard highlight, not data selection)",
            ComponentState {
                selected: true,
                ..ComponentState::default()
            },
            colors.surface.hover,
            colors.content.primary,
        ),
        (
            "disabled",
            ComponentState {
                disabled: true,
                ..ComponentState::default()
            },
            Color::TRANSPARENT,
            colors.content.disabled,
        ),
        (
            "disabled takes precedence over hover/focused/selected",
            ComponentState {
                disabled: true,
                hovered: true,
                focused: true,
                selected: true,
                ..ComponentState::default()
            },
            Color::TRANSPARENT,
            colors.content.disabled,
        ),
    ];

    for (name, state, expected_background, expected_foreground) in cases {
        let recipe = theme.overlay_item(state);
        assert_eq!(
            recipe.background,
            Brush::Solid(expected_background),
            "{name} background"
        );
        assert_eq!(recipe.foreground, expected_foreground, "{name} foreground");
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(Color::TRANSPARENT),
            "{name} border"
        );
        assert_eq!(recipe.border.width, theme.strokes.default);
        assert_eq!(recipe.radius, theme.radii.sm);
        assert_ne!(
            recipe.background,
            Brush::Solid(colors.selection.background),
            "{name} must never use the accent selection brush"
        );
    }
}

/// Pins `theme.command_palette_item` to `docs/visual-spec/04-overlays.md`'s
/// Command palette section (family issue #913): the active
/// (`state.selected`) item is the one place in this family where selection
/// genuinely is data selection (00-language.md's explicit "this IS data
/// selection" exception) and takes the accent brush; every other state
/// falls back to the same neutral treatment `Theme::overlay_item` uses.
#[test]
fn command_palette_item_is_the_one_accent_selection_exception() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.hover = Color::rgb8(1, 2, 3);
    colors.content.primary = Color::rgb8(4, 5, 6);
    colors.content.secondary = Color::rgb8(7, 8, 9);
    colors.content.disabled = Color::rgb8(10, 11, 12);
    colors.selection.background = Color::rgb8(13, 14, 15);
    colors.selection.foreground = Color::rgb8(16, 17, 18);
    let theme = default_dark_theme().with_colors(colors);

    let active = theme.command_palette_item(ComponentState {
        selected: true,
        ..ComponentState::default()
    });
    assert_eq!(active.background, Brush::Solid(colors.selection.background));
    assert_eq!(active.foreground, colors.selection.foreground);

    let active_hovered = theme.command_palette_item(ComponentState {
        selected: true,
        hovered: true,
        ..ComponentState::default()
    });
    assert_eq!(active_hovered, active, "active precedence beats hover too");

    let hovered_only = theme.command_palette_item(ComponentState {
        hovered: true,
        ..ComponentState::default()
    });
    assert_eq!(
        hovered_only,
        theme.overlay_item(ComponentState {
            hovered: true,
            ..ComponentState::default()
        })
    );

    let rest = theme.command_palette_item(ComponentState::default());
    assert_eq!(rest, theme.overlay_item(ComponentState::default()));

    let disabled_selected = theme.command_palette_item(ComponentState {
        selected: true,
        disabled: true,
        ..ComponentState::default()
    });
    assert_eq!(
        disabled_selected,
        theme.overlay_item(ComponentState {
            selected: true,
            disabled: true,
            ..ComponentState::default()
        }),
        "disabled beats even the active/selected accent"
    );
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
        Brush::Solid(theme.colors.accent.subtle)
    );
    assert_eq!(
        theme.slider(focused).border.brush,
        Brush::Solid(Color::TRANSPARENT)
    );
    assert_eq!(
        // docs/visual-spec/02-fields.md: focused resolves to `border.strong`,
        // the same tier as hovered — the ring is a separate additive layer,
        // not a border-color change (00-language.md §Focus model).
        theme.text_field(focused).border.brush,
        Brush::Solid(theme.colors.border.strong)
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

/// Pins `theme.checkbox`/`theme.radio_button` outputs to
/// `docs/visual-spec/03-choice-sliders-tabs.md`'s Checkbox/Radio tables
/// (family issue #912). Hover is asserted equal to its non-hovered
/// counterpart rather than pinned to a third color: that section's Checkbox
/// hover rule only promotes the (separately painted) label, leaving box
/// fill/border unchanged, and the Radio section inherits the same "same
/// row/pattern as checkbox" hover behavior.
#[test]
fn checkbox_and_radio_match_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.application = Color::rgb8(1, 2, 3);
    colors.accent.default = Color::rgb8(4, 5, 6);
    colors.border.strong = Color::rgb8(7, 8, 9);
    colors.border.disabled = Color::rgb8(10, 11, 12);
    colors.content.on_accent = Color::rgb8(13, 14, 15);
    colors.content.disabled = Color::rgb8(16, 17, 18);
    colors.content.primary = Color::rgb8(19, 20, 21);
    let theme = default_dark_theme().with_colors(colors);

    let unchecked = ComponentState::default();
    let checked = ComponentState {
        selected: true,
        ..ComponentState::default()
    };
    let hovered_unchecked = ComponentState {
        hovered: true,
        ..ComponentState::default()
    };
    let disabled_unchecked = ComponentState {
        disabled: true,
        ..ComponentState::default()
    };
    let disabled_checked = ComponentState {
        disabled: true,
        selected: true,
        ..ComponentState::default()
    };

    // Checkbox: checked fills accent and drops its border to transparent;
    // disabled always reads via S1 fill / `border.disabled`, overriding
    // checked, and hover changes nothing on the box itself.
    let check_unchecked = theme.checkbox(unchecked);
    assert_eq!(
        check_unchecked.fill,
        Brush::Solid(colors.surface.application)
    );
    assert_eq!(
        check_unchecked.border.brush,
        Brush::Solid(colors.border.strong)
    );

    let check_checked = theme.checkbox(checked);
    assert_eq!(check_checked.fill, Brush::Solid(colors.accent.default));
    assert_eq!(check_checked.border.brush, Brush::Solid(Color::TRANSPARENT));
    assert_eq!(check_checked.mark, colors.content.on_accent);

    let check_hovered = theme.checkbox(hovered_unchecked);
    assert_eq!(check_hovered.fill, check_unchecked.fill, "hover fill");
    assert_eq!(check_hovered.border, check_unchecked.border, "hover border");

    let check_disabled_unchecked = theme.checkbox(disabled_unchecked);
    assert_eq!(
        check_disabled_unchecked.fill,
        Brush::Solid(colors.surface.application)
    );
    assert_eq!(
        check_disabled_unchecked.border.brush,
        Brush::Solid(colors.border.disabled)
    );

    let check_disabled_checked = theme.checkbox(disabled_checked);
    assert_eq!(
        check_disabled_checked.fill,
        Brush::Solid(colors.surface.application),
        "disabled checkbox fill stays S1 even when checked"
    );
    assert_eq!(
        check_disabled_checked.border.brush,
        Brush::Solid(colors.border.disabled)
    );
    assert_eq!(check_disabled_checked.mark, colors.content.disabled);

    // Radio: fill/border are neutral (constant) across checked/unchecked —
    // only the dot color (`mark`) reacts to `selected`.
    for state in [unchecked, checked, hovered_unchecked] {
        let radio = theme.radio_button(state);
        assert_eq!(radio.fill, Brush::Solid(colors.surface.application));
        assert_eq!(radio.border.brush, Brush::Solid(colors.border.strong));
    }
    assert_eq!(theme.radio_button(checked).mark, colors.content.primary);

    for state in [disabled_unchecked, disabled_checked] {
        let radio = theme.radio_button(state);
        assert_eq!(radio.fill, Brush::Solid(colors.surface.application));
        assert_eq!(radio.border.brush, Brush::Solid(colors.border.disabled));
    }
    assert_eq!(
        theme.radio_button(disabled_checked).mark,
        colors.content.disabled
    );
}

/// Pins `theme.toggle` outputs to
/// `docs/visual-spec/03-choice-sliders-tabs.md` §Switch's off/on/disabled
/// rows (family issue #912). That table has no separate hover row, so
/// hover must not change the track or knob color.
#[test]
fn toggle_matches_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.border.default = Color::rgb8(1, 2, 3);
    colors.border.strong = Color::rgb8(4, 5, 6);
    colors.border.disabled = Color::rgb8(7, 8, 9);
    colors.accent.subtle = Color::rgb8(10, 11, 12);
    colors.focus.indicator = Color::rgb8(13, 14, 15);
    colors.content.muted = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    let theme = default_dark_theme().with_colors(colors);

    let off = ComponentState::default();
    let on = ComponentState {
        selected: true,
        ..ComponentState::default()
    };
    let hovered_off = ComponentState {
        hovered: true,
        ..ComponentState::default()
    };
    let disabled_off = ComponentState {
        disabled: true,
        ..ComponentState::default()
    };
    let disabled_on = ComponentState {
        disabled: true,
        selected: true,
        ..ComponentState::default()
    };

    let off_recipe = theme.toggle(off);
    assert_eq!(off_recipe.track, Brush::Solid(colors.border.default));
    assert_eq!(off_recipe.border.brush, Brush::Solid(colors.border.strong));
    assert_eq!(off_recipe.thumb, Brush::Solid(colors.content.muted));

    let hovered_recipe = theme.toggle(hovered_off);
    assert_eq!(hovered_recipe.track, off_recipe.track, "hover track");
    assert_eq!(hovered_recipe.thumb, off_recipe.thumb, "hover knob");

    let on_recipe = theme.toggle(on);
    assert_eq!(on_recipe.track, Brush::Solid(colors.accent.subtle));
    assert_eq!(on_recipe.border.brush, Brush::Solid(colors.border.strong));
    assert_eq!(on_recipe.thumb, Brush::Solid(colors.focus.indicator));

    for (state, name) in [(disabled_off, "disabled off"), (disabled_on, "disabled on")] {
        let recipe = theme.toggle(state);
        assert_eq!(
            recipe.track,
            Brush::Solid(colors.border.default),
            "{name} keeps off-style track fill"
        );
        assert_eq!(
            recipe.border.brush,
            Brush::Solid(colors.border.disabled),
            "{name} border"
        );
        assert_eq!(
            recipe.thumb,
            Brush::Solid(colors.content.disabled),
            "{name} knob"
        );
    }
}

/// Pins `theme.slider` outputs to
/// `docs/visual-spec/03-choice-sliders-tabs.md` §Slider (family issue
/// #912). Only an active drag (`pressed`) promotes the filled span to
/// `accent.hover`; hover alone only changes the resolved thumb color. The
/// track never draws an outline stroke — the file's "remainder" color is
/// the track's own fill, so `border` stays transparent at every state.
#[test]
fn slider_matches_visual_spec_state_colors() {
    let mut colors = ThemeColors::default_dark();
    colors.border.default = Color::rgb8(1, 2, 3);
    colors.border.subtle = Color::rgb8(4, 5, 6);
    colors.accent.default = Color::rgb8(7, 8, 9);
    colors.accent.hover = Color::rgb8(10, 11, 12);
    colors.content.primary = Color::rgb8(13, 14, 15);
    colors.content.on_accent = Color::rgb8(16, 17, 18);
    colors.content.disabled = Color::rgb8(19, 20, 21);
    let theme = default_dark_theme().with_colors(colors);

    let idle = ComponentState::default();
    let hovered = ComponentState {
        hovered: true,
        ..ComponentState::default()
    };
    let dragging = ComponentState {
        pressed: true,
        ..ComponentState::default()
    };
    let disabled = ComponentState {
        disabled: true,
        ..ComponentState::default()
    };

    let idle_recipe = theme.slider(idle);
    assert_eq!(idle_recipe.track, Brush::Solid(colors.border.default));
    assert_eq!(idle_recipe.fill, Brush::Solid(colors.accent.default));
    assert_eq!(idle_recipe.thumb, Brush::Solid(colors.content.primary));
    assert_eq!(idle_recipe.border.brush, Brush::Solid(Color::TRANSPARENT));

    let hovered_recipe = theme.slider(hovered);
    assert_eq!(hovered_recipe.track, idle_recipe.track);
    assert_eq!(
        hovered_recipe.fill, idle_recipe.fill,
        "hover alone doesn't promote the filled span"
    );
    assert_eq!(hovered_recipe.thumb, Brush::Solid(colors.content.on_accent));

    let dragging_recipe = theme.slider(dragging);
    assert_eq!(dragging_recipe.track, idle_recipe.track);
    assert_eq!(dragging_recipe.fill, Brush::Solid(colors.accent.hover));
    assert_eq!(
        dragging_recipe.thumb,
        Brush::Solid(colors.content.on_accent)
    );

    let disabled_recipe = theme.slider(disabled);
    assert_eq!(disabled_recipe.track, Brush::Solid(colors.border.subtle));
    assert_eq!(disabled_recipe.fill, Brush::Solid(colors.content.disabled));
    assert_eq!(disabled_recipe.thumb, Brush::Solid(colors.content.disabled));
    assert_eq!(
        disabled_recipe.border.brush,
        Brush::Solid(Color::TRANSPARENT)
    );
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
        // Ghost idle border is transparent (01-buttons.md §Quiet variant),
        // independent of `border.subtle` even though it is overridden above.
        Brush::Solid(Color::TRANSPARENT)
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
        // docs/visual-spec/02-fields.md: focused resolves to `border.strong`,
        // independent of `border.focused` even though it is overridden above
        // (the ring, not the border, carries the focus color).
        Brush::Solid(colors.border.strong)
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

/// Checks the vendored copy of the design-system token module against the
/// upstream `stern-design-system` checkout when it is present as a sibling of
/// the workspace root. Skips silently (with a note) when the sibling checkout
/// is absent, e.g. in CI.
#[test]
fn vendored_tokens_match_design_system_output() {
    let upstream_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("../stern-design-system/generated/rust/stern_tokens.rs");
    let Ok(upstream) = std::fs::read_to_string(&upstream_path) else {
        eprintln!(
            "skipping design-system token drift check: {} is not present",
            upstream_path.display()
        );
        return;
    };
    let upstream = upstream.replace("\r\n", "\n");

    let sha_line = upstream
        .lines()
        .find(|line| line.starts_with("pub const SOURCE_SHA256"))
        .expect("upstream stern_tokens.rs declares SOURCE_SHA256");
    let upstream_sha = sha_line
        .split('"')
        .nth(1)
        .expect("upstream SOURCE_SHA256 declaration carries a quoted value");
    assert_eq!(
        upstream_sha,
        generated_tokens::SOURCE_SHA256,
        "design-system tokens drifted: re-vendor \
         stern-design-system/generated/rust/stern_tokens.rs into \
         crates/stern-core/src/theme/generated_tokens.rs (keep the vendored \
         provenance header)"
    );

    // The vendored file is the upstream file with a provenance/doc header
    // inserted; everything after the upstream `@generated` header must match
    // byte for byte.
    let vendored = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/theme/generated_tokens.rs"),
    )
    .expect("vendored generated_tokens.rs is readable")
    .replace("\r\n", "\n");
    let upstream_body = upstream
        .split_once("\n\n")
        .expect("upstream stern_tokens.rs separates its header with a blank line")
        .1;
    assert!(
        vendored.ends_with(upstream_body),
        "design-system tokens drifted: the vendored generated_tokens.rs body \
         no longer matches stern-design-system/generated/rust/stern_tokens.rs; \
         re-vendor it (keep the vendored provenance header)"
    );
}

/// Resolves a design-system `#RRGGBB` color token into a [`Color`].
fn design_token_color(name: &str) -> Color {
    let token = generated_tokens::COLORS
        .iter()
        .find(|token| token.name == name)
        .unwrap_or_else(|| panic!("design-system color token `{name}` is missing"));
    let hex = token
        .value
        .strip_prefix('#')
        .unwrap_or_else(|| panic!("token `{name}` is not a hex color: {}", token.value));
    assert_eq!(
        hex.len(),
        6,
        "token `{name}` must be #RRGGBB: {}",
        token.value
    );
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16)
            .unwrap_or_else(|err| panic!("token `{name}` has an invalid hex channel: {err}"))
    };
    Color::rgb8(channel(0..2), channel(2..4), channel(4..6))
}

/// Asserts every listed [`SemanticColor`] key resolves, in the default dark
/// theme, to the exact `stern-design-system` token its
/// [`SemanticColor::design_token_name`] mapping points at.
fn assert_group_matches_design_system_tokens(colors: &ThemeColors, roles: &[SemanticColor]) {
    for &role in roles {
        let name = role.design_token_name();
        assert_eq!(
            colors.get(role),
            design_token_color(name),
            "{role:?} does not match design-system token `{name}`"
        );
    }
}

#[test]
fn default_dark_accent_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::AccentSubtle,
            SemanticColor::AccentDefault,
            SemanticColor::AccentHover,
            SemanticColor::AccentPressed,
            SemanticColor::AccentFocus,
            SemanticColor::AccentForeground,
        ],
    );
}

#[test]
fn default_dark_surface_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::SurfaceApplication,
            SemanticColor::SurfaceWorkspace,
            SemanticColor::SurfacePanel,
            SemanticColor::SurfacePanelRaised,
            SemanticColor::SurfaceRaised,
            SemanticColor::SurfaceControl,
            SemanticColor::SurfaceControlHover,
            SemanticColor::SurfaceControlPressed,
            SemanticColor::SurfaceControlDisabled,
            SemanticColor::SurfaceOverlay,
            SemanticColor::SurfaceHover,
            SemanticColor::SurfaceSunken,
        ],
    );
}

#[test]
fn default_dark_text_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::ContentPrimary,
            SemanticColor::ContentSecondary,
            SemanticColor::ContentMuted,
            SemanticColor::ContentDisabled,
            SemanticColor::ContentOnAccent,
            SemanticColor::ContentLink,
        ],
    );
}

#[test]
fn default_dark_border_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::BorderSubtle,
            SemanticColor::BorderDefault,
            SemanticColor::BorderStrong,
            SemanticColor::BorderHover,
            SemanticColor::BorderFocused,
            SemanticColor::BorderDisabled,
            SemanticColor::BorderInvalid,
        ],
    );
}

#[test]
fn default_dark_selection_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::SelectionBackground,
            SemanticColor::SelectionForeground,
        ],
    );
}

#[test]
fn default_dark_focus_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::FocusIndicator,
            SemanticColor::FocusSeparator,
            SemanticColor::FocusRing,
        ],
    );
}

#[test]
fn default_dark_overlay_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(&colors, &[SemanticColor::OverlayScrim]);
}

#[test]
fn default_dark_status_group_matches_design_system_tokens() {
    let colors = ThemeColors::default_dark();

    assert_group_matches_design_system_tokens(
        &colors,
        &[
            SemanticColor::StatusInfoForeground,
            SemanticColor::StatusInfoSurface,
            SemanticColor::StatusInfoBorder,
            SemanticColor::StatusInfoStrong,
            SemanticColor::StatusSuccessForeground,
            SemanticColor::StatusSuccessSurface,
            SemanticColor::StatusSuccessBorder,
            SemanticColor::StatusSuccessStrong,
            SemanticColor::StatusWarningForeground,
            SemanticColor::StatusWarningSurface,
            SemanticColor::StatusWarningBorder,
            SemanticColor::StatusWarningStrong,
            SemanticColor::StatusDangerForeground,
            SemanticColor::StatusDangerSurface,
            SemanticColor::StatusDangerBorder,
            SemanticColor::StatusDangerStrong,
        ],
    );
}

/// Completeness guard: every one of the 53 [`SemanticColor`] resolver keys
/// (`SemanticColor::ALL`) maps to a named `generated_tokens::COLORS` entry
/// and matches it exactly in the default dark theme. The per-group tests
/// above exist for readability; this test is the actual end-state guarantee
/// from the mapping (see `docs/design-system-tokens.md`): a vendored token
/// value can't drift from the theme without breaking a test, for every key,
/// not just the ones a group list happens to include.
#[test]
fn default_dark_maps_every_semantic_color_to_a_design_system_token() {
    let colors = ThemeColors::default_dark();

    assert_eq!(SemanticColor::ALL.len(), 53);
    assert_group_matches_design_system_tokens(&colors, SemanticColor::ALL);
}

// --- Metrics token mapping (issue #901: spacing, radii, sizes) ---
//
// Same mechanism as the semantic-color mapping above: each metric carries a
// `design_token_name()` naming the exact `generated_tokens` entry it must
// equal, keyed off `docs/visual-spec/00-language.md`'s Geometry ladder (see
// `docs/design-system-tokens.md`).

/// Resolves a design-system metric token's numeric value from a named
/// [`generated_tokens::NamedMetric`] slice (`SPACING`, `RADII`, or `SIZES`).
fn design_token_metric(tokens: &[generated_tokens::NamedMetric], name: &str) -> f32 {
    tokens
        .iter()
        .find(|token| token.name == name)
        .unwrap_or_else(|| panic!("design-system metric token `{name}` is missing"))
        .value
}

/// Completeness guard: every one of the 9 [`SpacingStep`] ladder rungs
/// (`SpacingStep::ALL`) resolves, in the default dark theme, to the exact
/// `spacing.N` token its [`SpacingStep::design_token_name`] mapping points
/// at.
#[test]
fn default_dark_spacing_ladder_matches_design_system_tokens() {
    let spacing = default_dark_theme().spacing;

    assert_eq!(SpacingStep::ALL.len(), 9);
    for &step in SpacingStep::ALL {
        let name = step.design_token_name();
        assert_eq!(
            spacing.get(step),
            design_token_metric(generated_tokens::SPACING, name),
            "{step:?} does not match design-system token `{name}`"
        );
    }
}

/// Completeness guard: every one of the 9 semantic [`SpacingRole`] keys
/// (`SpacingRole::ALL`) resolves, in the default dark theme, to the exact
/// named `spacing.gap.*` / `spacing.padding.*` token its
/// [`SpacingRole::design_token_name`] mapping points at (a separate, named
/// token from the generic ladder rung its `step()` also resolves to).
#[test]
fn default_dark_spacing_roles_match_design_system_named_tokens() {
    let spacing = default_dark_theme().spacing;

    assert_eq!(SpacingRole::ALL.len(), 9);
    for &role in SpacingRole::ALL {
        let name = role.design_token_name();
        assert_eq!(
            spacing.resolve(role),
            design_token_metric(generated_tokens::SPACING, name),
            "{role:?} does not match design-system token `{name}`"
        );
    }
}

/// Completeness guard: every one of the 14 [`SizeToken`] keys
/// (`SizeToken::ALL`) resolves, in the default dark theme, to the exact
/// `size.*` token its [`SizeToken::design_token_name`] mapping points at.
#[test]
fn default_dark_size_scale_matches_design_system_tokens() {
    let sizes = default_dark_theme().sizes;

    assert_eq!(SizeToken::ALL.len(), 14);
    for &token in SizeToken::ALL {
        let name = token.design_token_name();
        assert_eq!(
            sizes.get(token),
            design_token_metric(generated_tokens::SIZES, name),
            "{token:?} does not match design-system token `{name}`"
        );
    }
}

/// Density ladder assertion (issue #901): the exact geometry-ladder values
/// from `docs/visual-spec/00-language.md` §Geometry ladder — control heights
/// 20/24/28/32, panel header 30, workspace bar 40 — resolve through
/// `Theme::sizes` (`SizeScale`), which is the one stern API surface that
/// currently carries all six values pinned to the design-system tokens.
///
/// `Theme::sizes` is not yet a single authority consumed everywhere it
/// should be: only `sizes.icon.md` (icon rendering) and `sizes.workspace_bar`
/// (`stern_widgets::chrome::ApplicationBar`) are read by stern-widgets
/// layout code today;
/// `sizes.control.*`, `sizes.row.*`, `sizes.tab`, and `sizes.panel_header`
/// have no widget consumer yet, and the dock tab strip hardcodes its own
/// `DEFAULT_TAB_HEIGHT` literal instead of reading `sizes.tab` (see
/// `KNOWN-GAPS.md` item 15). This test pins the values `Theme::sizes` does
/// carry so that gap doesn't hide a value drift.
#[test]
fn default_dark_control_and_header_height_ladder_matches_geometry_spec() {
    let sizes = default_dark_theme().sizes;

    assert_eq!(sizes.control.xs, 20.0);
    assert_eq!(sizes.control.sm, 24.0);
    assert_eq!(sizes.control.md, 28.0);
    assert_eq!(sizes.control.lg, 32.0);
    assert_eq!(sizes.panel_header, 30.0);
    assert_eq!(sizes.workspace_bar, 40.0);

    assert_eq!(
        sizes.control.xs,
        design_token_metric(generated_tokens::SIZES, "size.control.xs")
    );
    assert_eq!(
        sizes.control.sm,
        design_token_metric(generated_tokens::SIZES, "size.control.sm")
    );
    assert_eq!(
        sizes.control.md,
        design_token_metric(generated_tokens::SIZES, "size.control.md")
    );
    assert_eq!(
        sizes.control.lg,
        design_token_metric(generated_tokens::SIZES, "size.control.lg")
    );
    assert_eq!(
        sizes.panel_header,
        design_token_metric(generated_tokens::SIZES, "size.panelHeader")
    );
    assert_eq!(
        sizes.workspace_bar,
        design_token_metric(generated_tokens::SIZES, "size.workspaceBar")
    );
}

/// Completeness guard: every one of the 5 [`RadiusToken`] keys
/// (`RadiusToken::ALL`) resolves, in the default dark theme, to the exact
/// `radius.*` token its [`RadiusToken::design_token_name`] mapping points
/// at.
#[test]
fn default_dark_radius_scale_matches_design_system_tokens() {
    let radii = default_dark_theme().radii;

    assert_eq!(RadiusToken::ALL.len(), 5);
    for &token in RadiusToken::ALL {
        let name = token.design_token_name();
        let value = design_token_metric(generated_tokens::RADII, name);
        assert_eq!(
            radii.get(token),
            CornerRadius::all(value),
            "{token:?} does not match design-system token `{name}`"
        );
    }
}
