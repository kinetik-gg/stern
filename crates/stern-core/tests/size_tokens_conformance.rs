//! Exact size-token foundation and legacy-isolation conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    Color, ControlMetrics, ControlSizeScale, CornerRadius, HandleSizeScale, IconSizeScale,
    RadiusScale, RowSizeScale, SizeScale, SizeToken, SpacingScale, StrokeScale, default_dark_theme,
};

const TOKENS: [SizeToken; 14] = [
    SizeToken::ControlXs,
    SizeToken::ControlSm,
    SizeToken::ControlMd,
    SizeToken::ControlLg,
    SizeToken::RowCompact,
    SizeToken::RowStandard,
    SizeToken::Tab,
    SizeToken::PanelHeader,
    SizeToken::WorkspaceBar,
    SizeToken::IconSm,
    SizeToken::IconMd,
    SizeToken::IconLg,
    SizeToken::HandleVisual,
    SizeToken::HandleHit,
];

const DEFAULT_VALUES: [f32; 14] = [
    20.0, 24.0, 28.0, 32.0, 24.0, 28.0, 28.0, 30.0, 40.0, 12.0, 16.0, 20.0, 1.0, 7.0,
];

const SENTINEL_VALUES: [f32; 14] = [
    101.0, 103.0, 107.0, 109.0, 113.0, 127.0, 131.0, 137.0, 139.0, 149.0, 151.0, 157.0, 163.0,
    167.0,
];

fn sentinel_sizes() -> SizeScale {
    SizeScale::new(
        ControlSizeScale::new(
            SENTINEL_VALUES[0],
            SENTINEL_VALUES[1],
            SENTINEL_VALUES[2],
            SENTINEL_VALUES[3],
        ),
        RowSizeScale::new(SENTINEL_VALUES[4], SENTINEL_VALUES[5]),
        SENTINEL_VALUES[6],
        SENTINEL_VALUES[7],
        SENTINEL_VALUES[8],
        IconSizeScale::new(SENTINEL_VALUES[9], SENTINEL_VALUES[10], SENTINEL_VALUES[11]),
        HandleSizeScale::new(SENTINEL_VALUES[12], SENTINEL_VALUES[13]),
    )
}

#[test]
fn default_size_scale_matches_all_fourteen_normative_tokens() {
    let sizes = default_dark_theme().sizes;

    assert_eq!(SizeToken::ALL, TOKENS);
    assert_eq!(sizes.control.xs, 20.0);
    assert_eq!(sizes.control.sm, 24.0);
    assert_eq!(sizes.control.md, 28.0);
    assert_eq!(sizes.control.lg, 32.0);
    assert_eq!(sizes.row.compact, 24.0);
    assert_eq!(sizes.row.standard, 28.0);
    assert_eq!(sizes.tab, 28.0);
    assert_eq!(sizes.panel_header, 30.0);
    assert_eq!(sizes.workspace_bar, 40.0);
    assert_eq!(sizes.icon.sm, 12.0);
    assert_eq!(sizes.icon.md, 16.0);
    assert_eq!(sizes.icon.lg, 20.0);
    assert_eq!(sizes.handle.visual, 1.0);
    assert_eq!(sizes.handle.hit, 7.0);
    assert_ne!(sizes.handle.visual, sizes.handle.hit);

    for ((token, expected), inventory_token) in TOKENS
        .into_iter()
        .zip(DEFAULT_VALUES)
        .zip(SizeToken::ALL.iter().copied())
    {
        assert_eq!(token, inventory_token);
        assert_eq!(sizes.get(token), expected, "wrong value for {token:?}");
    }
}

#[test]
fn distinct_sentinels_prove_complete_independent_lookup_routing() {
    let sizes = sentinel_sizes();

    for (index, token) in TOKENS.into_iter().enumerate() {
        assert_eq!(sizes.get(token), SENTINEL_VALUES[index]);
        for other in &SENTINEL_VALUES[index + 1..] {
            assert_ne!(SENTINEL_VALUES[index], *other);
        }
    }

    assert_eq!(sizes.control.xs, SENTINEL_VALUES[0]);
    assert_eq!(sizes.control.sm, SENTINEL_VALUES[1]);
    assert_eq!(sizes.control.md, SENTINEL_VALUES[2]);
    assert_eq!(sizes.control.lg, SENTINEL_VALUES[3]);
    assert_eq!(sizes.row.compact, SENTINEL_VALUES[4]);
    assert_eq!(sizes.row.standard, SENTINEL_VALUES[5]);
    assert_eq!(sizes.tab, SENTINEL_VALUES[6]);
    assert_eq!(sizes.panel_header, SENTINEL_VALUES[7]);
    assert_eq!(sizes.workspace_bar, SENTINEL_VALUES[8]);
    assert_eq!(sizes.icon.sm, SENTINEL_VALUES[9]);
    assert_eq!(sizes.icon.md, SENTINEL_VALUES[10]);
    assert_eq!(sizes.icon.lg, SENTINEL_VALUES[11]);
    assert_eq!(sizes.handle.visual, SENTINEL_VALUES[12]);
    assert_eq!(sizes.handle.hit, SENTINEL_VALUES[13]);
    assert_ne!(sizes.handle.visual, sizes.handle.hit);
}

#[test]
fn with_sizes_changes_only_the_size_foundation() {
    let mut baseline = default_dark_theme();
    baseline.colors.surface.workspace = Color::rgb8(1, 3, 5);
    baseline.spacing = SpacingScale::new(
        173.0, 179.0, 181.0, 191.0, 193.0, 197.0, 199.0, 211.0, 223.0,
    );
    baseline.radii = RadiusScale::from_values(227.0, 229.0, 233.0, 239.0);
    baseline.strokes = StrokeScale::from_values(241.0, 251.0, 257.0, 263.0, 269.0);
    baseline.typography.label.size = 271.0;
    baseline.opacity.pressed = 277.0;
    baseline.elevation.medium = 281.0;
    baseline.duration.fast = 283.0;
    baseline.controls = ControlMetrics {
        control_height: 293.0,
        compact_control_height: 307.0,
        icon_size: 311.0,
        check_size: 313.0,
        padding_x: 317.0,
        padding_y: 331.0,
    };
    baseline.radius = CornerRadius::all(337.0);
    baseline.border_width = 347.0;
    baseline.text_size = 349.0;

    let customized = baseline.with_sizes(sentinel_sizes());

    assert_eq!(customized.sizes, sentinel_sizes());
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
}

#[test]
fn spacing_and_control_customization_do_not_mirror_size_tokens() {
    let controls = ControlMetrics {
        control_height: 353.0,
        compact_control_height: 359.0,
        icon_size: 367.0,
        check_size: 373.0,
        padding_x: 379.0,
        padding_y: 383.0,
    };
    let spacing = SpacingScale::new(
        389.0, 397.0, 401.0, 409.0, 419.0, 421.0, 431.0, 433.0, 439.0,
    );
    let customized = default_dark_theme()
        .with_sizes(sentinel_sizes())
        .with_controls(controls)
        .with_spacing(spacing);

    assert_eq!(customized.sizes, sentinel_sizes());
    assert_eq!(customized.spacing, spacing);
    assert_eq!(customized.controls, controls);
    assert_ne!(
        customized.controls.control_height,
        customized.sizes.control.md
    );
    assert_ne!(customized.controls.icon_size, customized.sizes.icon.md);

    assert_eq!(default_dark_theme().controls.control_height, 28.0);
    assert_eq!(default_dark_theme().controls.compact_control_height, 22.0);
    assert_eq!(default_dark_theme().controls.icon_size, 16.0);
    assert_eq!(default_dark_theme().controls.check_size, 14.0);
    assert_eq!(default_dark_theme().controls.padding_x, 8.0);
    assert_eq!(default_dark_theme().controls.padding_y, 4.0);
}
