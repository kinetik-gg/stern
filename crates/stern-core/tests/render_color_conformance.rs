//! Public color contract conformance tests.

#![allow(clippy::float_cmp)]

use stern_core::{Color, default_dark_theme};

#[test]
fn color_constructors_preserve_straight_srgb_inputs() {
    let color = Color::rgba(-0.25, 1.25, f32::NAN, f32::INFINITY);

    assert_eq!(color.r, -0.25);
    assert_eq!(color.g, 1.25);
    assert!(color.b.is_nan());
    assert_eq!(color.a, f32::INFINITY);

    let opaque = Color::rgb(0.25, 0.5, 0.75);
    assert_eq!(opaque, Color::rgba(0.25, 0.5, 0.75, 1.0));
    assert_eq!(opaque.with_alpha(0.4), Color::rgba(0.25, 0.5, 0.75, 0.4));

    assert_eq!(
        Color::rgba8(0, u8::MAX, 0, u8::MAX),
        Color::rgba(0.0, 1.0, 0.0, 1.0)
    );

    let bytes = Color::rgba8(0x0C, 0x8C, 0xE9, 0x7F);
    assert_eq!(
        bytes,
        Color::rgba(
            f32::from(0x0C_u8) / 255.0,
            f32::from(0x8C_u8) / 255.0,
            f32::from(0xE9_u8) / 255.0,
            f32::from(0x7F_u8) / 255.0,
        )
    );
    let opaque = Color::rgb8(0x0C, 0x8C, 0xE9);
    assert_eq!(opaque, Color::rgba8(0x0C, 0x8C, 0xE9, u8::MAX));
    assert_eq!(opaque.a, 1.0);
}

#[test]
fn color_constants_and_default_theme_accent_are_exact() {
    assert_eq!(Color::TRANSPARENT, Color::rgba(0.0, 0.0, 0.0, 0.0));
    assert_eq!(Color::BLACK, Color::rgba(0.0, 0.0, 0.0, 1.0));
    assert_eq!(Color::WHITE, Color::rgba(1.0, 1.0, 1.0, 1.0));

    assert_eq!(
        default_dark_theme().colors.accent.default,
        Color::rgb8(0x0C, 0x8C, 0xE9)
    );
    assert_eq!(
        default_dark_theme().colors.selection.background,
        default_dark_theme().colors.accent.default
    );
}
