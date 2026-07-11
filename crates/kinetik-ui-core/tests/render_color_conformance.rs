//! Public color contract conformance tests.

#![allow(clippy::float_cmp)]

use kinetik_ui_core::{Color, default_dark_theme};

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
}

#[test]
fn color_constants_and_default_theme_values_are_unchanged() {
    assert_eq!(Color::TRANSPARENT, Color::rgba(0.0, 0.0, 0.0, 0.0));
    assert_eq!(Color::BLACK, Color::rgba(0.0, 0.0, 0.0, 1.0));
    assert_eq!(Color::WHITE, Color::rgba(1.0, 1.0, 1.0, 1.0));

    assert_eq!(
        default_dark_theme().colors.accent,
        Color::rgba(0.13, 0.40, 0.96, 1.0)
    );
}
