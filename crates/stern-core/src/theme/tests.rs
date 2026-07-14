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
