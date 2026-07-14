//! Public semantic palette conformance tests.

#![allow(clippy::float_cmp)]

use std::collections::HashSet;

use stern_core::{Color, SemanticColor, ThemeColors, default_dark_theme};

macro_rules! token {
    ($role:ident, $r:expr, $g:expr, $b:expr) => {
        (SemanticColor::$role, Color::rgb8($r, $g, $b))
    };
}

const EXPECTED: [(SemanticColor, Color); 53] = [
    token!(SurfaceApplication, 0x11, 0x11, 0x11),
    token!(SurfaceWorkspace, 0x0B, 0x0B, 0x0B),
    token!(SurfacePanel, 0x14, 0x14, 0x14),
    token!(SurfacePanelRaised, 0x18, 0x18, 0x18),
    token!(SurfaceRaised, 0x18, 0x18, 0x18),
    token!(SurfaceControl, 0x18, 0x18, 0x18),
    token!(SurfaceControlHover, 0x1C, 0x1C, 0x1C),
    token!(SurfaceControlPressed, 0x2A, 0x2A, 0x2A),
    token!(SurfaceControlDisabled, 0x14, 0x14, 0x14),
    token!(SurfaceOverlay, 0x18, 0x18, 0x18),
    token!(SurfaceHover, 0x1C, 0x1C, 0x1C),
    token!(SurfaceSunken, 0x0B, 0x0B, 0x0B),
    token!(ContentPrimary, 0xE8, 0xE8, 0xE8),
    token!(ContentSecondary, 0xB8, 0xB8, 0xB8),
    token!(ContentMuted, 0x99, 0x99, 0x99),
    token!(ContentDisabled, 0x66, 0x66, 0x66),
    token!(ContentOnAccent, 0xFF, 0xFF, 0xFF),
    token!(ContentLink, 0x25, 0x9C, 0xF0),
    token!(BorderSubtle, 0x22, 0x22, 0x22),
    token!(BorderDefault, 0x2A, 0x2A, 0x2A),
    token!(BorderStrong, 0x3D, 0x3D, 0x3D),
    token!(BorderHover, 0x3D, 0x3D, 0x3D),
    token!(BorderFocused, 0x4D, 0xB2, 0xFF),
    token!(BorderDisabled, 0x22, 0x22, 0x22),
    token!(BorderInvalid, 0xF1, 0x8A, 0x90),
    token!(SelectionBackground, 0x0C, 0x8C, 0xE9),
    token!(SelectionForeground, 0xFF, 0xFF, 0xFF),
    token!(FocusIndicator, 0x4D, 0xB2, 0xFF),
    token!(FocusSeparator, 0x0B, 0x0B, 0x0B),
    token!(FocusRing, 0x4D, 0xB2, 0xFF),
    token!(OverlayScrim, 0x0B, 0x0B, 0x0B),
    token!(AccentSubtle, 0x0B, 0x2A, 0x3F),
    token!(AccentDefault, 0x0C, 0x8C, 0xE9),
    token!(AccentHover, 0x25, 0x9C, 0xF0),
    token!(AccentPressed, 0x08, 0x76, 0xC5),
    token!(AccentFocus, 0x4D, 0xB2, 0xFF),
    token!(AccentForeground, 0xFF, 0xFF, 0xFF),
    token!(StatusInfoForeground, 0x6C, 0xBF, 0xFF),
    token!(StatusInfoSurface, 0x10, 0x18, 0x20),
    token!(StatusInfoBorder, 0x25, 0x34, 0x3F),
    token!(StatusInfoStrong, 0x0C, 0x8C, 0xE9),
    token!(StatusSuccessForeground, 0x72, 0xD9, 0x98),
    token!(StatusSuccessSurface, 0x12, 0x1A, 0x15),
    token!(StatusSuccessBorder, 0x29, 0x37, 0x2E),
    token!(StatusSuccessStrong, 0x39, 0xB8, 0x68),
    token!(StatusWarningForeground, 0xF0, 0xC6, 0x6D),
    token!(StatusWarningSurface, 0x1A, 0x17, 0x11),
    token!(StatusWarningBorder, 0x3A, 0x33, 0x26),
    token!(StatusWarningStrong, 0xD9, 0xA4, 0x41),
    token!(StatusDangerForeground, 0xF1, 0x8A, 0x90),
    token!(StatusDangerSurface, 0x1B, 0x13, 0x14),
    token!(StatusDangerBorder, 0x3D, 0x29, 0x2B),
    token!(StatusDangerStrong, 0xD9, 0x53, 0x5B),
];

#[test]
fn every_dark_role_and_resolver_entry_is_exact_unique_and_stably_ordered() {
    let colors = ThemeColors::default_dark();
    let expected_roles = EXPECTED.map(|(role, _)| role);

    assert_eq!(SemanticColor::ALL.len(), 53);
    assert_eq!(SemanticColor::ALL, expected_roles.as_slice());
    assert_eq!(
        SemanticColor::ALL
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len(),
        SemanticColor::ALL.len()
    );
    for (role, expected) in EXPECTED {
        assert_eq!(colors.get(role), expected, "wrong value for {role:?}");
        assert_eq!(default_dark_theme().color(role), expected);
    }
    assert_eq!(colors, default_dark_theme().colors);
}

#[test]
fn equal_dark_values_remain_independently_overridable() {
    let defaults = ThemeColors::default_dark();
    let mut colors = defaults;

    colors.surface.panel_raised = Color::rgb8(1, 2, 3);
    colors.selection.background = Color::rgb8(4, 5, 6);
    colors.content.on_accent = Color::rgb8(7, 8, 9);
    colors.focus.indicator = Color::rgb8(10, 11, 12);

    assert_eq!(
        colors.get(SemanticColor::SurfacePanelRaised),
        Color::rgb8(1, 2, 3)
    );
    assert_eq!(colors.surface.raised, defaults.surface.raised);
    assert_eq!(colors.surface.control, defaults.surface.control);
    assert_eq!(colors.surface.overlay, defaults.surface.overlay);
    assert_eq!(
        colors.get(SemanticColor::SelectionBackground),
        Color::rgb8(4, 5, 6)
    );
    assert_eq!(colors.accent.default, defaults.accent.default);
    assert_eq!(colors.status.info.strong, defaults.status.info.strong);
    assert_eq!(
        colors.get(SemanticColor::ContentOnAccent),
        Color::rgb8(7, 8, 9)
    );
    assert_eq!(colors.selection.foreground, defaults.selection.foreground);
    assert_eq!(colors.accent.foreground, defaults.accent.foreground);
    assert_eq!(
        colors.get(SemanticColor::FocusIndicator),
        Color::rgb8(10, 11, 12)
    );
    assert_eq!(colors.focus.ring, defaults.focus.ring);
}

#[test]
fn with_colors_replaces_only_the_grouped_palette() {
    let original = default_dark_theme();
    let mut colors = ThemeColors::default_dark();
    colors.content.primary = Color::rgb8(0x12, 0x34, 0x56);
    let customized = original.with_colors(colors);

    assert_eq!(customized.colors, colors);
    assert_eq!(customized.spacing, original.spacing);
    assert_eq!(customized.radii, original.radii);
    assert_eq!(customized.typography, original.typography);
    assert_eq!(customized.opacity, original.opacity);
    assert_eq!(customized.elevation, original.elevation);
    assert_eq!(customized.duration, original.duration);
    assert_eq!(customized.controls, original.controls);
    assert_eq!(customized.radius, original.radius);
    assert_eq!(customized.border_width, original.border_width);
    assert_eq!(customized.text_size, original.text_size);
}
