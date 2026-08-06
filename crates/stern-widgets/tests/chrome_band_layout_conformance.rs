//! Deterministic conformance for the viewport-driven chrome band layout.

#![allow(clippy::float_cmp)] // Band layout math is exact token arithmetic.

use stern_core::{Rect, default_dark_theme};
use stern_widgets::ChromeBandLayout;

fn assert_tiles_exactly(layout: ChromeBandLayout, bounds: Rect) {
    let bands = [
        ("menu-bar", layout.menu_bar),
        ("toolbar", layout.toolbar),
        ("tab-strip", layout.tab_strip),
        ("content", layout.content),
        ("status-bar", layout.status_bar),
    ];
    let mut cursor = bounds.y;
    for (name, band) in bands {
        assert_eq!(band.x, bounds.x, "{name} keeps the viewport x origin");
        assert_eq!(band.width, bounds.width, "{name} spans the full width");
        assert_eq!(band.y, cursor, "{name} stacks without gaps or overlap");
        assert!(band.height >= 0.0, "{name} height is non-negative");
        cursor += band.height;
    }
    assert!(
        (cursor - bounds.max_y()).abs() < 1e-3,
        "bands tile the viewport exactly: reached {cursor}, expected {}",
        bounds.max_y()
    );
}

#[test]
fn band_heights_resolve_from_theme_size_tokens() {
    let theme = default_dark_theme();
    let bounds = Rect::new(0.0, 0.0, 960.0, 640.0);
    let layout = ChromeBandLayout::from_viewport(bounds, &theme);

    assert_eq!(layout.menu_bar.height, theme.sizes.control.md);
    assert_eq!(layout.toolbar.height, theme.sizes.control.lg);
    assert_eq!(layout.tab_strip.height, theme.sizes.tab);
    assert_eq!(layout.status_bar.height, theme.sizes.control.sm);
    assert_eq!(
        layout.content.height,
        640.0
            - theme.sizes.control.md
            - theme.sizes.control.lg
            - theme.sizes.tab
            - theme.sizes.control.sm
    );
    assert_tiles_exactly(layout, bounds);
}

#[test]
fn bands_track_every_viewport_size_including_4k_logical() {
    let theme = default_dark_theme();
    for (width, height) in [
        (960.0, 640.0),
        (1280.0, 800.0),
        (1920.0, 1080.0),
        (3840.0, 2160.0),
    ] {
        let bounds = Rect::new(0.0, 0.0, width, height);
        let layout = ChromeBandLayout::from_viewport(bounds, &theme);
        assert_tiles_exactly(layout, bounds);
        assert_eq!(layout.status_bar.max_y(), height, "status pins to bottom");
        assert!(
            layout.content.height > 0.0,
            "content band exists at {width}x{height}"
        );
    }
}

#[test]
fn bands_preserve_a_non_zero_viewport_origin() {
    let theme = default_dark_theme();
    let bounds = Rect::new(24.0, 16.0, 640.0, 480.0);
    let layout = ChromeBandLayout::from_viewport(bounds, &theme);
    assert_eq!(layout.menu_bar.y, 16.0);
    assert_eq!(layout.menu_bar.x, 24.0);
    assert_tiles_exactly(layout, bounds);
}

#[test]
fn short_viewports_clamp_bands_in_stacking_order_without_overflow() {
    let theme = default_dark_theme();
    // Enough for menu + part of the toolbar only.
    let bounds = Rect::new(0.0, 0.0, 320.0, theme.sizes.control.md + 10.0);
    let layout = ChromeBandLayout::from_viewport(bounds, &theme);
    assert_eq!(layout.menu_bar.height, theme.sizes.control.md);
    assert_eq!(layout.toolbar.height, 10.0);
    assert_eq!(layout.tab_strip.height, 0.0);
    assert_eq!(layout.content.height, 0.0);
    assert_eq!(layout.status_bar.height, 0.0);
    assert_tiles_exactly(layout, bounds);
}

#[test]
fn degenerate_viewports_produce_zero_area_bands() {
    let theme = default_dark_theme();
    for bounds in [
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Rect::new(0.0, 0.0, -10.0, -10.0),
    ] {
        let layout = ChromeBandLayout::from_viewport(bounds, &theme);
        for band in [
            layout.menu_bar,
            layout.toolbar,
            layout.tab_strip,
            layout.content,
            layout.status_bar,
        ] {
            assert!(band.width <= 0.0 || band.height <= 0.0);
            assert!(band.height >= 0.0);
        }
    }
}
